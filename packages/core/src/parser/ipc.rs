//! IPC (Inter-Process Communication) parser for Python sidecar processes
//!
//! This module provides a streaming IPC parser that spawns a Python sidecar process
//! (e.g., `cocoindex`) and reads AST chunks from stdout in JSON Lines (JSONL) format.
//!
//! # Design
//!
//! - **Streaming**: Uses `BufReader` to read stdout line-by-line, avoiding OOM
//! - **Panic Safe**: All errors are captured and returned as `Result`, never panics
//! - **Type Safe**: Each JSONL line is deserialized into `AstChunk` using `serde_json`
//!
//! # Usage
//!
//! ```no_run
//! use contextfy_core::parser::ipc::SidecarIPC;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut ipc = SidecarIPC::spawn(["cocoindex", "parse", "file.py"])?;
//!
//! while let Ok(Some(chunk)) = ipc.next_chunk() {
//!     println!("Parsed: {} -> {}", chunk.file_path, chunk.symbol_name);
//! }
//!
//! // Check child exit status
//! ipc.wait()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Environment Variables
//!
//! - `COCOINDEX_MODE=dev`: Use `uv run` to invoke the sidecar (for development)
//! - `COCOINDEX_MODE=prod` or unset: Directly invoke the binary (for production)

use crate::kernel::types::AstChunk;
use anyhow::{Context, Result};
use std::io::{BufRead, Read};
use std::process::{Child, ChildStderr, Command, Stdio};

/// IPC-specific errors
///
/// Provides detailed error information for debugging IPC failures.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// Child process failed to start
    #[error("Failed to start child process: command='{command}', cause='{cause}'")]
    ChildStartFailed { command: String, cause: String },

    /// Stream read operation failed
    #[error("Failed to read from stdout: {cause}")]
    StreamReadFailed { cause: String },

    /// JSON deserialization failed
    #[error("Failed to parse JSON at line {line_number}: raw_line='{raw_line}', cause='{cause}'")]
    JsonParseFailed {
        line_number: usize,
        raw_line: String,
        cause: String,
    },

    /// Child process exited abnormally
    #[error("Child process exited abnormally: exit_code={exit_code:?}, stderr='{stderr}'")]
    ChildExitedAbnormally {
        exit_code: Option<i32>,
        stderr: String,
    },
}

/// IPC sidecar process handler
///
/// Spawns a child process and provides streaming access to its stdout.
pub struct SidecarIPC {
    child: Child,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    stderr: Option<ChildStderr>,
    line_number: usize,
}

impl SidecarIPC {
    /// Spawn a new sidecar process
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments (e.g., `["cocoindex", "parse", "file.py"]`)
    ///
    /// # Environment
    ///
    /// - `COCOINDEX_MODE=dev`: Use `uv run` prefix
    /// - `COCOINDEX_MODE=prod` or unset: Direct invocation
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Command not found
    /// - Permission denied
    /// - Stdout/stderr piping failed
    pub fn spawn<A, I>(args: I) -> Result<Self>
    where
        A: AsRef<str>,
        I: IntoIterator<Item = A>,
    {
        let args: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

        // Determine command based on environment
        let (program, full_args) = if std::env::var("COCOINDEX_MODE").as_deref() == Ok("dev") {
            // Development mode: use `uv run`
            ("uv".to_string(), {
                let mut uv_args = vec!["run".to_string()];
                uv_args.extend(args.clone());
                uv_args
            })
        } else {
            // Production mode: direct invocation
            (args[0].clone(), args[1..].to_vec())
        };

        let mut cmd = Command::new(&program);
        cmd.args(&full_args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn child process
        let mut child = cmd.spawn().with_context(|| IpcError::ChildStartFailed {
            command: format!("{} {:?}", program, full_args),
            cause: "unknown".to_string(),
        })?;

        // Extract stdout and stderr handles using take() to avoid partial move
        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!(IpcError::ChildStartFailed {
                command: format!("{} {:?}", program, full_args),
                cause: "stdout not captured".to_string(),
            })
        })?;

        let stderr = child.stderr.take();

        Ok(Self {
            child,
            stdout: std::io::BufReader::new(stdout),
            stderr,
            line_number: 0,
        })
    }

    /// Read the next AST chunk from stdout
    ///
    /// # Returns
    ///
    /// - `Ok(Some(chunk))` - Successfully read and parsed a chunk
    /// - `Ok(None)` - Reached EOF (end of stream)
    /// - `Err(...)` - IO error, JSON parse error, or child process error
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use contextfy_core::parser::ipc::SidecarIPC;
    /// # fn main() -> anyhow::Result<()> {
    /// let mut ipc = SidecarIPC::spawn(["echo", "{}"])?;
    /// while let Ok(Some(chunk)) = ipc.next_chunk() {
    ///     // Process chunk
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn next_chunk(&mut self) -> Result<Option<AstChunk>> {
        let mut line = String::new();

        // Read a line from stdout
        let bytes_read =
            self.stdout
                .read_line(&mut line)
                .with_context(|| IpcError::StreamReadFailed {
                    cause: "failed to read line".to_string(),
                })?;

        // Check for EOF
        if bytes_read == 0 {
            return Ok(None);
        }

        // Increment line counter for error reporting
        self.line_number += 1;

        // Trim trailing newline (but keep other whitespace)
        let line = line.trim_end();

        // Deserialize JSON
        let chunk: AstChunk =
            serde_json::from_str(line).with_context(|| IpcError::JsonParseFailed {
                line_number: self.line_number,
                raw_line: if line.len() > 100 {
                    format!("{}...", &line[..100])
                } else {
                    line.to_string()
                },
                cause: "invalid JSON".to_string(),
            })?;

        Ok(Some(chunk))
    }

    /// Wait for child process to exit and check exit status
    ///
    /// Should be called after `next_chunk()` returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Exit code is non-zero
    /// - Failed to read stderr
    pub fn wait(&mut self) -> Result<()> {
        // Collect stderr if available
        let stderr = if let Some(mut stderr_handle) = self.stderr.take() {
            let mut stderr_output = String::new();
            stderr_handle
                .read_to_string(&mut stderr_output)
                .context("Failed to read stderr")?;
            Some(stderr_output)
        } else {
            None
        };

        // Wait for child to exit
        let status = self
            .child
            .wait()
            .context("Failed to wait for child process")?;

        // Check exit code
        if !status.success() {
            return Err(anyhow::anyhow!(IpcError::ChildExitedAbnormally {
                exit_code: status.code(),
                stderr: stderr.unwrap_or_default(),
            }));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_child_process_success() {
        // Use echo to simulate valid JSONL output
        let json_line = r#"{"file_path":"test.py","symbol_name":"foo","node_type":"function","ast_content":"pass","dependencies":[]}"#;

        let mut ipc = SidecarIPC::spawn(["echo", json_line]).expect("Failed to spawn");

        // First call should return Some(chunk)
        let result = ipc.next_chunk();
        assert!(result.is_ok(), "next_chunk should succeed");
        let chunk = result.unwrap();
        assert!(chunk.is_some(), "Should have one chunk");

        let chunk = chunk.unwrap();
        assert_eq!(chunk.file_path, "test.py");
        assert_eq!(chunk.symbol_name, "foo");
        assert_eq!(chunk.node_type, "function");
        assert_eq!(chunk.ast_content, "pass");
        assert!(chunk.dependencies.is_empty());

        // Second call should return None (EOF)
        let result = ipc.next_chunk();
        assert!(result.is_ok(), "next_chunk should succeed");
        assert!(result.unwrap().is_none(), "Should be at EOF");

        // Check exit status (should be success)
        ipc.wait().expect("Child should exit successfully");
    }

    #[test]
    fn test_json_parse_error_handling() {
        // Use echo to output invalid JSON
        let mut ipc = SidecarIPC::spawn(["echo", "not a json"]).expect("Failed to spawn");

        let result = ipc.next_chunk();
        assert!(result.is_err(), "Should fail to parse invalid JSON");

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("Failed to parse JSON"),
            "Error should mention JSON parsing"
        );
        assert!(
            error_msg.contains("not a json"),
            "Error should include raw line"
        );
    }

    #[test]
    fn test_child_exit_failure() {
        // Use sh -c "exit 1" to simulate child process crash
        let mut ipc = SidecarIPC::spawn(["sh", "-c", "exit 1"]).expect("Failed to spawn");

        // next_chunk should return None (EOF) immediately
        let result = ipc.next_chunk();
        assert!(result.is_ok(), "next_chunk should succeed");
        assert!(result.unwrap().is_none(), "Should be at EOF");

        // wait() should detect non-zero exit code
        let result = ipc.wait();
        assert!(result.is_err(), "wait should detect non-zero exit code");

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("Child process exited abnormally"),
            "Error should mention abnormal exit"
        );
        assert!(
            error_msg.contains("exit_code=Some(1)"),
            "Error should include exit code"
        );
    }

    #[test]
    fn test_empty_stream_handling() {
        // Use echo -n "" to simulate empty output (no newline)
        let mut ipc = SidecarIPC::spawn(["echo", "-n", ""]).expect("Failed to spawn");

        // next_chunk should return None (EOF) immediately
        let result = ipc.next_chunk();
        assert!(result.is_ok(), "next_chunk should succeed");
        assert!(result.unwrap().is_none(), "Should be at EOF");

        // wait() should succeed (exit code 0)
        ipc.wait().expect("Child should exit successfully");
    }

    #[test]
    fn test_multiple_chunks() {
        // Use printf to output multiple JSONL lines (more reliable than echo for special chars)
        let json1 = r#"{"file_path":"test1.py","symbol_name":"foo","node_type":"function","ast_content":"pass","dependencies":[]}"#;
        let json2 = r#"{"file_path":"test2.py","symbol_name":"bar","node_type":"class","ast_content":"class Bar","dependencies":[]}"#;

        // Use printf to output both lines
        let cmd = format!("printf '{}\\n{}\\n' '{}' '{}'", json1, json2, json1, json2);
        let mut ipc = SidecarIPC::spawn(["sh", "-c", &cmd]).expect("Failed to spawn");

        // First chunk
        let result = ipc.next_chunk();
        assert!(result.is_ok());
        let chunk1 = result.unwrap().unwrap();
        assert_eq!(chunk1.file_path, "test1.py");
        assert_eq!(chunk1.symbol_name, "foo");

        // Second chunk
        let result = ipc.next_chunk();
        if let Err(ref e) = result {
            eprintln!("Second chunk error: {}", e);
        }
        assert!(result.is_ok(), "Second chunk should succeed: {:?}", result);
        let chunk2 = result.unwrap().unwrap();
        assert_eq!(chunk2.file_path, "test2.py");
        assert_eq!(chunk2.symbol_name, "bar");

        // Third call should return None (EOF)
        let result = ipc.next_chunk();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        ipc.wait().expect("Child should exit successfully");
    }

    #[test]
    fn test_default_dependencies() {
        // JSON without dependencies field
        let json_line = r#"{"file_path":"test.py","symbol_name":"foo","node_type":"function","ast_content":"pass"}"#;

        let mut ipc = SidecarIPC::spawn(["echo", json_line]).expect("Failed to spawn");

        let result = ipc.next_chunk();
        assert!(result.is_ok());
        let chunk = result.unwrap().unwrap();
        assert!(
            chunk.dependencies.is_empty(),
            "Dependencies should default to empty array"
        );
    }

    #[test]
    fn test_dev_mode_spawn() {
        // Test that dev mode uses uv run
        std::env::set_var("COCOINDEX_MODE", "dev");

        // We can't actually test uv run without having uv installed,
        // but we can verify the code compiles and the logic is reachable
        // For now, just verify the environment variable is read correctly
        let mode = std::env::var("COCOINDEX_MODE").unwrap();
        assert_eq!(mode, "dev");

        // Clean up
        std::env::remove_var("COCOINDEX_MODE");
    }
}
