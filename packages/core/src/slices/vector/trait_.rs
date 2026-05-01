//! Vector store trait
//!
//! This module defines the abstract interface for vector storage backends.
//! This allows the core engine to remain agnostic to the specific vector database implementation.
//!
//! **MANDATORY CONSTRAINT**: This trait MUST NOT expose LanceDB-specific types.
//! All methods use stable kernel types (`kernel::types::Query`, `kernel::types::Hit`).
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use crate::kernel::errors::AppError;
use crate::kernel::types::{Hit, Query};
use async_trait::async_trait;

/// Vector store trait
///
/// This abstract interface defines vector storage operations without committing
/// to a specific implementation (LanceDB, Qdrant, Milvus, etc.).
///
/// # Design Principles
///
/// 1. **Infrastructure Agnostic**: Methods accept/return kernel types only
/// 2. **Error Mapping**: Implementation maps backend errors to `InfraError`
/// 3. **Async**: All operations are async to support I/O-bound vector databases
/// 4. **Option Semantics**: Clear distinction between empty results and no data
///
/// # Option Semantics (`Ok(None)` vs `Ok(Some(vec![]))`)
///
/// **IMPORTANT**: Choose the right return type based on semantic meaning:
///
/// - **`Ok(Some(vec![]))`**: Search completed successfully, but found zero matches
///   - Use this for normal "no results found" cases
///   - Indicates the search was executed, but no documents matched
///   - Example: Searching for "xyz" in a database with no matching documents
///
/// - **`Ok(None)`**: Search completed but returned no data (backend-specific)
///   - Use this when the backend has a semantic concept of "no data"
///   - Rare: Most implementations should prefer `Ok(Some(vec![]))`
///   - Example: A cached backend that hasn't been populated yet
///
/// **Guideline**: Default to `Ok(Some(vec![]))` for empty search results.
/// Only use `Ok(None)` if your backend has a strong semantic reason to distinguish
/// "not found" from "empty results".
///
/// # Usage Example
///
/// ```ignore
/// use async_trait::async_trait;
///
/// struct MyVectorStore {
///     // backend-specific fields
/// }
///
/// #[async_trait]
/// impl VectorStoreTrait for MyVectorStore {
///     async fn search(&self, query: &Query) -> Result<Option<Vec<Hit>>, AppError> {
///         // 1. Convert kernel Query to backend-specific query
///         // 2. Execute search against backend
///         // 3. Convert backend results to kernel Hit types
///         // 4. Return Ok(Some(hits)) or Ok(None)
///     }
/// }
/// ```
#[async_trait]
pub trait VectorStoreTrait: Send + Sync {
    /// Search for similar vectors
    ///
    /// This method performs semantic vector search based on the query text.
    ///
    /// # Parameters
    ///
    /// * `query` - Search query containing text and limit
    ///
    /// # Returns
    ///
    /// * `Ok(Some(hits))` - Search completed successfully
    ///   - `hits` may be empty (`vec![]`) if no matches found
    ///   - **IMPORTANT**: Empty results should return `Ok(Some(vec![]))`, not `Ok(None)`
    /// * `Ok(None)` - Search completed but returned no data (backend-specific, rare)
    /// * `Err(AppError)` - Infrastructure error (connection, serialization, etc.)
    ///
    /// # Implementation Notes
    ///
    /// - Query text should be embedded to a vector before searching
    /// - Results must be converted to `Hit` with normalized scores [0.0, 1.0]
    /// - Scores should be descending (best match first)
    /// - Preserve the original error chain when mapping to `InfraError`
    ///
    /// # Example: Empty Results
    ///
    /// ```ignore
    /// // When search finds no matches, return Ok(Some(vec![]))
    /// let hits = vec![];  // Empty, not None
    /// Ok(Some(hits))
    /// ```
    ///
    /// # Example: No Data (Backend-Specific)
    ///
    /// ```ignore
    /// // Only use Ok(None) if backend has semantic "no data" concept
    /// if !backend.is_initialized() {
    ///     return Ok(None);
    /// }
    /// ```
    async fn search(&self, query: &Query) -> Result<Option<Vec<Hit>>, AppError>;

    /// Add a document to the vector store
    ///
    /// # Parameters
    ///
    /// * `id` - Unique identifier for the document
    /// * `text` - Document text to be embedded and stored
    /// * `metadata` - Optional metadata (title, source path, etc.)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Document added successfully
    /// * `Err(AppError)` - Infrastructure error
    async fn add(
        &self,
        id: &str,
        text: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<(), AppError>;

    /// Delete a document from the vector store
    ///
    /// # Parameters
    ///
    /// * `id` - Identifier of the document to delete
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Document was deleted
    /// * `Ok(false)` - Document was not found (idempotent)
    /// * `Err(AppError)` - Infrastructure error
    async fn delete(&self, id: &str) -> Result<bool, AppError>;

    /// Check if the store is healthy and accessible
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Store is healthy
    /// * `Ok(false)` - Store is unhealthy but not an error
    /// * `Err(AppError)` - Infrastructure error (connection failed, etc.)
    async fn health_check(&self) -> Result<bool, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::errors::InfraError;
    use crate::kernel::types::Score;

    /// Mock vector store for testing
    struct MockVectorStore {
        should_fail: bool,
        empty_results: bool,
    }

    #[async_trait]
    impl VectorStoreTrait for MockVectorStore {
        async fn search(&self, _query: &Query) -> Result<Option<Vec<Hit>>, AppError> {
            if self.should_fail {
                return Err(AppError::Infra(InfraError::database(
                    "mock connection failed",
                    None::<std::io::Error>,
                )));
            }

            if self.empty_results {
                return Ok(Some(vec![]));
            }

            // Return mock results
            let hits = vec![
                Hit::new("doc1", Score::new(0.9)),
                Hit::new("doc2", Score::new(0.7)),
                Hit::new("doc3", Score::new(0.5)),
            ];

            Ok(Some(hits))
        }

        async fn add(
            &self,
            _id: &str,
            _text: &str,
            _metadata: Option<&serde_json::Value>,
        ) -> Result<(), AppError> {
            if self.should_fail {
                return Err(AppError::Infra(InfraError::database(
                    "mock insert failed",
                    None::<std::io::Error>,
                )));
            }
            Ok(())
        }

        async fn delete(&self, _id: &str) -> Result<bool, AppError> {
            if self.should_fail {
                return Err(AppError::Infra(InfraError::database(
                    "mock delete failed",
                    None::<std::io::Error>,
                )));
            }

            // Mock: delete succeeds
            Ok(true)
        }

        async fn health_check(&self) -> Result<bool, AppError> {
            if self.should_fail {
                return Err(AppError::Infra(InfraError::database(
                    "mock health check failed",
                    None::<std::io::Error>,
                )));
            }
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_vector_store_trait_search() {
        let store = MockVectorStore {
            should_fail: false,
            empty_results: false,
        };

        let query = Query::new("test query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let hits = result.unwrap().unwrap();
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].id, "doc1");
        assert_eq!(hits[0].score.value(), 0.9);
    }

    #[tokio::test]
    async fn test_vector_store_trait_search_empty() {
        let store = MockVectorStore {
            should_fail: false,
            empty_results: true,
        };

        let query = Query::new("test query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let hits = result.unwrap().unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[tokio::test]
    async fn test_vector_store_trait_search_error() {
        let store = MockVectorStore {
            should_fail: true,
            empty_results: false,
        };

        let query = Query::new("test query", 10);
        let result = store.search(&query).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::Database { .. }) => {}
            _ => panic!("Expected InfraError::Database"),
        }
    }

    #[tokio::test]
    async fn test_vector_store_trait_add() {
        let store = MockVectorStore {
            should_fail: false,
            empty_results: false,
        };

        let result = store.add("doc1", "test content", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_vector_store_trait_delete() {
        let store = MockVectorStore {
            should_fail: false,
            empty_results: false,
        };

        let result = store.delete("doc1").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_vector_store_trait_health_check() {
        let store = MockVectorStore {
            should_fail: false,
            empty_results: false,
        };

        let result = store.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
