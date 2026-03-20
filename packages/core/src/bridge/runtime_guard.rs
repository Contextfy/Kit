//! Tokio runtime guard for safe FFI boundary crossing
//!
//! **PROBLEM**: N-API synchronous functions can deadlock if we call `block_on` from within
//! an existing Tokio runtime context (e.g., when JS async code calls our sync FFI).
//!
//! **SOLUTION**: This module provides:
//! 1. A global Tokio runtime singleton for blocking operations
//! 2. A guard API that detects existing runtime contexts and avoids nested `block_on`
//! 3. Clear distinction between sync FFI (use guard) and async FFI (direct await)
//!
//! **MANDATORY CONSTRAINT**:
//! - Sync FFI: Always use `RuntimeGuard::block_on()` to execute async operations
//! - Async FFI: Use `#[napi] async fn` and await directly - NO blocking wrapper
//! - NEVER manually call `Handle::try_current()` or `Runtime::new()` in bridge code
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 1
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/specs/bridge-layer/spec.md`

use once_cell::sync::OnceCell;
use std::future::Future;
use tokio::runtime::{Handle, Runtime};
use crate::bridge::error_map::BridgeError;

/// Global Tokio runtime singleton
///
/// This is created once and reused for all synchronous FFI calls that need to block.
/// Using a singleton prevents the overhead of creating a new runtime for each call.
///
/// **SAFETY**: Uses OnceCell with get_or_try_init to avoid panic on initialization failure.
static GLOBAL_RUNTIME: OnceCell<Runtime> = OnceCell::new();

/// Safely get or initialize the global runtime
///
/// This function uses get_or_try_init to safely create the runtime on first access,
/// returning a BridgeError if initialization fails instead of panicking.
fn get_global_runtime() -> Result<&'static Runtime, BridgeError> {
    GLOBAL_RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("contextfy-bridge-runtime")
            .enable_all()
            .build()
            .map_err(|e| BridgeError::runtime(
                "Failed to initialize global Tokio runtime",
                Some(Box::new(e)),
            ))
    })
}

/// Runtime guard for safe async execution in FFI context
///
/// This guard provides a safe way to execute async operations from synchronous FFI functions
/// without risking deadlock from nested `block_on` calls.
///
/// # Usage Pattern
///
/// ## Synchronous FFI (use this guard)
/// ```ignore
/// #[napi]
/// pub fn sync_search(query: String) -> napi::Result<SearchResult> {
///     // Use the guard to safely execute async code
///     RuntimeGuard::block_on(async {
///         do_async_search(query).await
///     })
///     .map_err(|e| BridgeError::from(e).into())
/// }
/// ```
///
/// ## Asynchronous FFI (direct await)
/// ```ignore
/// #[napi]
/// pub async fn async_search(query: String) -> napi::Result<SearchResult> {
///     // Direct await - no guard needed
///     do_async_search(query).await.map_err(|e| BridgeError::from(e).into())
/// }
/// ```
pub struct RuntimeGuard;

impl RuntimeGuard {
    /// Execute an async operation from a synchronous FFI context
    ///
    /// This method:
    /// 1. Detects if we're already inside a Tokio runtime context
    /// 2. If yes: Returns an error instead of panicking (safer for production)
    /// 3. If no: Uses the global runtime singleton (initialized safely)
    ///
    /// # Safety
    ///
    /// This approach is safe because:
    /// - When called from sync FFI, there's no runtime context → uses global runtime
    /// - When called from async context, returns a clear error instead of panicking
    /// - Runtime initialization failure returns BridgeError instead of panicking
    ///
    /// **IMPORTANT**: Nested block_on calls are not allowed in Tokio. If you're
    /// inside an async context, use `.await` instead of calling this function.
    ///
    /// # Parameters
    ///
    /// * `future` - The async operation to execute
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The result of the async operation
    /// * `Err(BridgeError)` - If called from within an async context or runtime init fails
    pub fn block_on<F, T>(future: F) -> Result<T, BridgeError>
    where
        F: Future<Output = T>,
        T: Send + 'static,
    {
        // Check if we're already in a runtime context
        match Handle::try_current() {
            Ok(_handle) => {
                // We're inside a runtime context - return error instead of panicking
                // This is safer for production than a panic
                Err(BridgeError::runtime(
                    "Cannot call RuntimeGuard::block_on from within an async context. Use .await instead.",
                    None::<std::io::Error>,
                ))
            }
            Err(_) => {
                // No runtime context (e.g., called from sync FFI)
                // Use the global runtime singleton - safely initialized with error handling
                let rt = get_global_runtime()?;
                Ok(rt.block_on(future))
            }
        }
    }

    /// Get a reference to the global runtime
    ///
    /// This is useful for scenarios where you need the runtime reference
    /// but don't want to immediately execute a future.
    ///
    /// # Returns
    ///
    /// * `Ok(&Runtime)` - Reference to the global runtime
    /// * `Err(BridgeError)` - If runtime initialization failed
    pub fn global() -> Result<&'static Runtime, BridgeError> {
        get_global_runtime()
    }

    /// Get the current thread's runtime handle if it exists
    ///
    /// Returns `None` if called from outside a runtime context.
    pub fn try_current_handle() -> Option<Handle> {
        Handle::try_current().ok()
    }

    /// Check if we're currently inside a Tokio runtime context
    pub fn is_in_runtime() -> bool {
        Handle::try_current().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_runtime_exists() {
        // The global runtime should be initialized safely
        let rt = RuntimeGuard::global().expect("Global runtime should initialize");
        assert!(rt.block_on(async { true }));
    }

    #[test]
    fn test_guard_outside_runtime() {
        // This test runs outside a Tokio runtime (typical for sync FFI)
        let result = RuntimeGuard::block_on(async { 42 });
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_guard_with_complex_future() {
        let result = RuntimeGuard::block_on(async {
            let mut sum = 0;
            for i in 1..=5 {
                sum += i;
            }
            sum
        });
        assert_eq!(result.unwrap(), 15); // 1+2+3+4+5 = 15
    }

    #[test]
    fn test_is_in_runtime() {
        // Outside a runtime context
        assert!(!RuntimeGuard::is_in_runtime());

        // Inside a runtime context
        let rt = RuntimeGuard::global().expect("Global runtime should initialize");
        let _ = rt.block_on(async {
            assert!(RuntimeGuard::is_in_runtime());
        });
    }

    #[test]
    fn test_try_current_handle() {
        // Outside a runtime context
        assert!(RuntimeGuard::try_current_handle().is_none());

        // Inside a runtime context
        let rt = RuntimeGuard::global().expect("Global runtime should initialize");
        let _ = rt.block_on(async {
            assert!(RuntimeGuard::try_current_handle().is_some());
        });
    }

    #[test]
    fn test_nested_guard_calls() {
        // Test that nested calls are properly detected and rejected with an error
        // instead of panicking
        let rt = RuntimeGuard::global().expect("Global runtime should initialize");
        let result = rt.block_on(async {
            // This inner call should return an error
            let inner = RuntimeGuard::block_on(async { 1 + 1 });
            assert!(inner.is_err(), "Nested block_on should return error");
            inner
        });

        assert!(result.is_err(), "Nested block_on should return error");
        assert!(result.unwrap_err().to_string().contains("async context"));
    }

    #[test]
    fn test_guard_returns_error_on_nested_call() {
        // This test verifies that calling block_on from within an async context
        // returns a proper BridgeError instead of panicking
        let rt = RuntimeGuard::global().expect("Global runtime should initialize");
        let err = rt.block_on(async {
            RuntimeGuard::block_on(async { 1 + 1 })
                .unwrap_err()
        });

        assert!(err.to_string().contains("Cannot call RuntimeGuard::block_on"));
    }

    #[test]
    fn test_safe_initialization() {
        // Test that get_global_runtime() returns Result instead of panicking
        // This test ensures the OnceCell + get_or_try_init pattern works correctly
        let result = std::panic::catch_unwind(|| {
            let _rt = get_global_runtime();
        });

        // Should never panic
        assert!(result.is_ok());

        // Should always succeed
        let rt = get_global_runtime();
        assert!(rt.is_ok());
    }
}
