//! Bridge API facade
//!
//! This module provides the public API surface for the bridge layer.
//! It demonstrates the proper usage of runtime_guard, dto, and error_map.
//!
//! **NOTE**: In Phase 1, this is a placeholder. The actual search functionality
//! will be implemented in Phase 2 (vector slice) and Phase 3 (BM25 + hybrid).
//!
//! ## Usage Pattern Examples
//!
//! ### Synchronous FFI (uses RuntimeGuard)
//! ```ignore
//! #[napi]
//! pub fn sync_search(query: QueryDto) -> napi::Result<SearchResponseDto> {
//!     // Use RuntimeGuard for sync FFI
//!     RuntimeGuard::block_on(async {
//!         // Call async business logic here
//!         do_search(query).await
//!     })
//!     .map_err(|e| BridgeError::from(e).into())
//! }
//! ```
//!
//! ### Asynchronous FFI (direct await)
//! ```ignore
//! #[napi]
//! pub async fn async_search(query: QueryDto) -> napi::Result<SearchResponseDto> {
//!     // Direct await - NO RuntimeGuard needed
//!     do_search(query).await.map_err(|e| BridgeError::from(e).into())
//! }
//! ```
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

use crate::bridge::dto::{QueryDto, SearchResponseDto};
use crate::bridge::error_map::BridgeError;
use crate::bridge::runtime_guard::RuntimeGuard;
use crate::kernel::errors::{AppError, DomainError};
use napi::Result;

/// Bridge API facade
///
/// This struct provides the public API surface for the bridge layer.
/// In Phase 1, it contains placeholder methods that demonstrate proper
/// usage of runtime guard, DTO conversion, and error mapping.
pub struct BridgeApi;

impl BridgeApi {
    /// Create a new bridge API instance
    pub fn new() -> Self {
        BridgeApi
    }

    /// Placeholder: Synchronous search via bridge
    ///
    /// **SYNC FFI PATTERN**: This method demonstrates the proper pattern for
    /// synchronous N-API functions that need to execute async Rust code.
    ///
    /// # Pattern
    /// 1. Convert DTO to kernel type
    /// 2. Use RuntimeGuard::block_on to execute async code
    /// 3. Convert kernel result back to DTO
    /// 4. Map errors using BridgeError
    ///
    /// # Returns
    /// Empty response in Phase 1 (will be implemented in Phase 2/3)
    pub fn sync_search(&self, query: QueryDto) -> Result<SearchResponseDto> {
        // Convert DTO to kernel type
        // Note: _kernel_query is intentionally unused in Phase 1 placeholder
        #[allow(unused_variables)]
        let _kernel_query: crate::kernel::types::Query = query.into();

        // Use RuntimeGuard for sync FFI - flattened with ? operator
        RuntimeGuard::block_on(async {
            // Phase 1: Return empty response
            // Phase 2/3: Call actual search logic
            Ok::<SearchResponseDto, AppError>(SearchResponseDto::empty(0))
        })
        .map_err(|e| BridgeError::from(e).into())
        .and_then(|inner_result| inner_result.map_err(|e| BridgeError::from(e).into()))
    }

    /// Placeholder: Asynchronous search via bridge
    ///
    /// **ASYNC FFI PATTERN**: This method demonstrates the proper pattern for
    /// asynchronous N-API functions.
    ///
    /// # Pattern
    /// 1. Convert DTO to kernel type
    /// 2. Directly await async operations (NO RuntimeGuard)
    /// 3. Convert kernel result back to DTO
    /// 4. Map errors using BridgeError
    ///
    /// # Returns
    /// Empty response in Phase 1 (will be implemented in Phase 2/3)
    pub async fn async_search(&self, query: QueryDto) -> Result<SearchResponseDto> {
        // Convert DTO to kernel type
        let _kernel_query: crate::kernel::types::Query = query.into();

        // Direct await - NO RuntimeGuard needed
        // Phase 1: Return empty response
        // Phase 2/3: Call actual search logic
        Ok(SearchResponseDto::empty(0))
    }

    /// Placeholder: Validate a query
    ///
    /// Demonstrates error handling with DomainError.
    pub fn validate_query(&self, query: QueryDto) -> Result<()> {
        // Validate query text
        if query.text.trim().is_empty() {
            let err = DomainError::invalid_query("query text cannot be empty");
            return Err(BridgeError::from(err).into());
        }

        // Validate limit
        if query.limit == 0 {
            let err = DomainError::invalid_query("query limit must be > 0");
            return Err(BridgeError::from(err).into());
        }

        if query.limit > 1000 {
            let err = DomainError::invalid_query("query limit must be <= 1000");
            return Err(BridgeError::from(err).into());
        }

        Ok(())
    }

    /// Check if currently inside a Tokio runtime context
    ///
    /// This is a health check method that verifies the runtime context detection.
    /// Returns true if called from within an async context, false otherwise.
    pub fn is_in_runtime_context(&self) -> bool {
        RuntimeGuard::is_in_runtime()
    }
}

impl Default for BridgeApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_api_creation() {
        let api = BridgeApi::new();
        assert!(!api.is_in_runtime_context()); // Not in runtime context
    }

    #[test]
    fn test_bridge_api_default() {
        let api = BridgeApi::default();
        assert!(!api.is_in_runtime_context());
    }

    #[test]
    fn test_validate_query_valid() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "test query".to_string(),
            limit: 10,
        };

        assert!(api.validate_query(query).is_ok());
    }

    #[test]
    fn test_validate_query_empty_text() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "".to_string(),
            limit: 10,
        };

        let result = api.validate_query(query);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("query text cannot be empty"));
    }

    #[test]
    fn test_validate_query_zero_limit() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "test".to_string(),
            limit: 0,
        };

        let result = api.validate_query(query);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("query limit must be > 0"));
    }

    #[test]
    fn test_validate_query_excessive_limit() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "test".to_string(),
            limit: 1001,
        };

        let result = api.validate_query(query);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("query limit must be <= 1000"));
    }

    #[test]
    fn test_sync_search_placeholder() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "test".to_string(),
            limit: 10,
        };

        let result = api.sync_search(query);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_empty());
        assert_eq!(response.total_count, 0);
    }

    #[tokio::test]
    async fn test_async_search_placeholder() {
        let api = BridgeApi::new();
        let query = QueryDto {
            text: "test".to_string(),
            limit: 10,
        };

        let result = api.async_search(query).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_empty());
        assert_eq!(response.total_count, 0);
    }
}
