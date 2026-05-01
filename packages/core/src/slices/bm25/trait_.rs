//! BM25 full-text search trait
//!
//! This module defines the backend-agnostic interface for BM25 full-text search.
//! Concrete implementations (Tantivy, etc.) implement this trait.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use async_trait::async_trait;

use crate::kernel::errors::AppError;
use crate::kernel::types::{Hit, Query, Score};

/// BM25 search result with document metadata
///
/// This struct contains the search result from BM25 full-text search.
/// Only contains stable types that don't leak backend-specific details.
#[derive(Debug, Clone, PartialEq)]
pub struct Bm25Result {
    /// Document ID
    pub id: String,
    /// Document title
    pub title: String,
    /// Document summary
    pub summary: String,
    /// Document content (optional, only populated when retrieved via get_by_id)
    pub content: Option<String>,
    /// BM25 relevance score
    pub score: Score,
}

impl Bm25Result {
    /// Create a new BM25 result (without content - for search results)
    pub fn new(id: String, title: String, summary: String, score: Score) -> Self {
        Self {
            id,
            title,
            summary,
            content: None,
            score,
        }
    }

    /// Create a new BM25 result with full content (for get_by_id results)
    pub fn with_content(
        id: String,
        title: String,
        summary: String,
        content: String,
        score: Score,
    ) -> Self {
        Self {
            id,
            title,
            summary,
            content: Some(content),
            score,
        }
    }

    /// Convert to kernel Hit type
    pub fn to_hit(self) -> Hit {
        Hit::new(self.id, self.score)
    }
}

/// Trait for BM25 full-text search storage backends
///
/// This trait defines the interface for BM25 full-text search operations.
/// It is intentionally minimal to allow for multiple backend implementations.
///
/// # Design Principles
///
/// - Uses only kernel types (`Query`, `Hit`, `Score`) for interface
/// - Returns backend-agnostic `Bm25Result` that can be converted to `Hit`
/// - Errors are mapped to `AppError` for consistent error handling
///
/// # Phase 1 Limitation
///
/// This is a placeholder interface. Concrete implementation will be in Phase 2.
#[async_trait]
pub trait Bm25StoreTrait: Send + Sync {
    /// Search for documents using BM25 full-text search
    ///
    /// # Parameters
    ///
    /// * `query` - Search query containing text and limit
    ///
    /// # Returns
    ///
    /// * `Ok(Some(results))` - Search succeeded with results
    /// * `Ok(None)` - No results found (not an error)
    /// * `Err(AppError)` - Search failed
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder. Actual BM25 search requires:
    /// - Tantivy query parsing
    /// - BM25 scoring
    /// - Result ranking
    async fn search(&self, query: &Query) -> Result<Option<Vec<Bm25Result>>, AppError>;

    /// Add a document to the BM25 index
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID
    /// * `title` - Document title
    /// * `summary` - Document summary
    /// * `content` - Document content
    /// * `keywords` - Space-separated keywords for boosted search ranking
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder. Actual add requires:
    /// - Tantivy document creation
    /// - Index writer insertion
    async fn add(
        &self,
        id: &str,
        title: &str,
        summary: &str,
        content: &str,
        keywords: &str,
    ) -> Result<(), AppError>;

    /// Delete a document from the BM25 index
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID to delete
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Document was deleted
    /// * `Ok(false)` - Document was not found
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder. Actual delete requires:
    /// - Tantivy term deletion
    async fn delete(&self, id: &str) -> Result<bool, AppError>;

    /// Check if the BM25 store is healthy and accessible
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Store is healthy
    /// * `Ok(false)` - Store is unhealthy
    /// * `Err(AppError)` - Health check failed
    async fn health_check(&self) -> Result<bool, AppError>;

    /// Get a document by ID
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Some(doc))` - Document found with full details
    /// * `Ok(None)` - Document not found
    /// * `Err(AppError)` - Retrieval failed
    async fn get_by_id(&self, id: &str) -> Result<Option<Bm25Result>, AppError>;

    /// Get multiple documents by IDs
    ///
    /// # Parameters
    ///
    /// * `ids` - Slice of document IDs to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(vec)` - Vector of options in the same order as input IDs.
    ///   Each element is `Some(doc)` if found, `None` if not found.
    /// * `Err(AppError)` - Retrieval failed
    ///
    /// # Performance
    ///
    /// This method should be more efficient than calling `get_by_id` multiple times,
    /// as it can batch the database queries.
    ///
    /// # Default Implementation
    ///
    /// The default implementation calls `get_by_id` for each ID concurrently.
    /// Implementations may override this to provide batched queries for better performance.
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Option<Bm25Result>>, AppError> {
        use futures::future::try_join_all;
        let futures = ids.iter().map(|id| self.get_by_id(id));
        try_join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::types::Query;

    /// Mock Bm25StoreTrait for testing
    struct MockBm25Store;

    #[async_trait]
    impl Bm25StoreTrait for MockBm25Store {
        async fn search(&self, _query: &Query) -> Result<Option<Vec<Bm25Result>>, AppError> {
            Ok(Some(vec![]))
        }

        async fn add(
            &self,
            _id: &str,
            _title: &str,
            _summary: &str,
            _content: &str,
            _keywords: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn delete(&self, _id: &str) -> Result<bool, AppError> {
            Ok(true)
        }

        async fn health_check(&self) -> Result<bool, AppError> {
            Ok(true)
        }

        async fn get_by_id(&self, _id: &str) -> Result<Option<Bm25Result>, AppError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_bm25_result_creation() {
        let result = Bm25Result::new(
            "test-id".to_string(),
            "Test Title".to_string(),
            "Test Summary".to_string(),
            Score::new(0.9),
        );

        assert_eq!(result.id, "test-id");
        assert_eq!(result.title, "Test Title");
        assert_eq!(result.summary, "Test Summary");
        assert_eq!(result.score.value(), 0.9);
    }

    #[tokio::test]
    async fn test_bm25_result_to_hit() {
        let result = Bm25Result::new(
            "test-id".to_string(),
            "Test Title".to_string(),
            "Test Summary".to_string(),
            Score::new(0.8),
        );

        let hit = result.to_hit();
        assert_eq!(hit.id, "test-id");
        assert_eq!(hit.score.value(), 0.8);
    }

    #[tokio::test]
    async fn test_mock_store_search() {
        let store = MockBm25Store;
        let query = Query::new("test query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(vec![]));
    }

    #[tokio::test]
    async fn test_mock_store_add() {
        let store = MockBm25Store;
        let result = store.add("id", "title", "summary", "content", "").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_store_delete() {
        let store = MockBm25Store;
        let result = store.delete("id").await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_store_health_check() {
        let store = MockBm25Store;
        let result = store.health_check().await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
