use anyhow::Result;
use contextfy_core::{parse_markdown, KnowledgeStore};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// 默认文档目录路径
const DEFAULT_DOCS_PATH: &str = "docs/examples";

/// Contextfy 项目配置
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Config {
    /// 文档目录路径
    #[serde(default = "default_docs_path")]
    docs_path: String,
    _name: Option<String>,
    _version: Option<String>,
    _description: Option<String>,
}

fn default_docs_path() -> String {
    DEFAULT_DOCS_PATH.to_string()
}

/// 构建知识库
///
/// 从 contextfy.json 读取配置，扫描指定文档目录，解析 Markdown 文档并存储到知识库中。
/// 每个文档会被切片并存储为独立的可检索单元。
///
/// # Errors
///
/// 如果配置文件格式错误、文档目录不存在或文档解析失败，返回错误
///
/// # Examples
///
/// ```no_run
/// # use contextfy_cli::commands::build;
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// build().await?;
/// # Ok(())
/// # }
/// ```
pub async fn build() -> Result<()> {
    let store = KnowledgeStore::new(".contextfy/data").await?;

    // 读取配置文件
    let config_path = Path::new("contextfy.json");
    let docs_path = if config_path.exists() {
        let config_content = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse contextfy.json: {}", e))?;
        config.docs_path
    } else {
        default_docs_path()
    };

    let examples_dir = Path::new(&docs_path);
    if !examples_dir.exists() {
        anyhow::bail!(
            "docs directory '{}' not found. Please check contextfy.json or create the directory.",
            docs_path
        );
    }

    let mut documents_count = 0;
    let mut sections_count = 0;

    for entry in fs::read_dir(examples_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_path = path.to_string_lossy();
            println!("Processing: {}", file_path);

            match parse_markdown(&file_path) {
                Ok(doc) => {
                    let ids = store.add(&doc).await?;
                    documents_count += 1;
                    sections_count += doc.sections.len();
                    if doc.sections.is_empty() {
                        println!("  → Stored: {} (ID: {})", doc.title, ids[0]);
                    } else {
                        println!("  → Stored: {} ({} slices)", doc.title, ids.len());
                        for (i, id) in ids.iter().enumerate() {
                            println!("      [{}] Slice ID: {}", i + 1, id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to parse {}: {}", file_path, e);
                }
            }
        }
    }

    println!("\n✓ Build complete!");
    println!(
        "Found {} documents, {} sections",
        documents_count, sections_count
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// 测试：contextfy.json 存在且包含 docs_path 字段
    #[test]
    fn test_config_with_docs_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("contextfy.json");
        let mut file = File::create(&config_file).unwrap();
        writeln!(file, r#"{{"name": "test", "docs_path": "custom/docs"}}"#).unwrap();

        let config_content = fs::read_to_string(&config_file).unwrap();
        let config: Config = serde_json::from_str(&config_content).unwrap();
        assert_eq!(config.docs_path, "custom/docs");
    }

    /// 测试：contextfy.json 存在但不包含 docs_path 字段（回退到默认路径）
    #[test]
    fn test_config_without_docs_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("contextfy.json");
        let mut file = File::create(&config_file).unwrap();
        writeln!(file, r#"{{"name": "test"}}"#).unwrap();

        let config_content = fs::read_to_string(&config_file).unwrap();
        let config: Config = serde_json::from_str(&config_content).unwrap();
        assert_eq!(config.docs_path, DEFAULT_DOCS_PATH);
    }

    /// 测试：默认路径函数返回正确的值
    #[test]
    fn test_default_docs_path() {
        assert_eq!(default_docs_path(), DEFAULT_DOCS_PATH);
    }

    /// 测试：JSON 格式错误时返回友好的错误消息
    #[test]
    fn test_invalid_json_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("contextfy.json");
        let mut file = File::create(&config_file).unwrap();
        writeln!(file, r#"{{"name": invalid}}"#).unwrap();

        let config_content = fs::read_to_string(&config_file).unwrap();
        let result: std::result::Result<Config, _> = serde_json::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse contextfy.json: {}", e));

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("Failed to parse contextfy.json"));
    }

    /// 测试：完整的 Config 结构体可以正确反序列化
    #[test]
    fn test_full_config_deserialization() {
        let json = r#"{
            "name": "test-project",
            "version": "0.1.0",
            "description": "A test project",
            "docs_path": "test/docs"
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.docs_path, "test/docs");
    }
}
