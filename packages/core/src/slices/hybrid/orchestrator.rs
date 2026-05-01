//! Hybrid retrieval orchestrator
//!
//! This module provides high-level orchestration for hybrid retrieval,
//! combining multiple search backends (BM25, vector) using RRF fusion.
//!
//! ## Architecture
//!
//! - Uses `VectorStoreTrait` for semantic search
//! - Uses `Bm25StoreTrait` for full-text search
//! - Uses `RrfOrchestrator` to merge results
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

use std::sync::Arc;
use tracing::{warn, error, info};

use crate::kernel::types::{Query, Hit};
use crate::kernel::errors::{AppError, DomainError};

use super::super::vector::VectorStoreTrait;
use super::super::bm25::Bm25StoreTrait;
use super::rrf::RrfOrchestrator;

/// Result of a hybrid delete operation
///
/// Contains the individual deletion results from both backends.
pub struct DeleteResult {
    /// Result of vector store deletion
    pub vector_deleted: Result<bool, AppError>,
    /// Result of BM25 store deletion
    pub bm25_deleted: Result<bool, AppError>,
}

impl DeleteResult {
    /// Check if at least one backend succeeded in deletion
    pub fn any_success(&self) -> bool {
        matches!((&self.vector_deleted, &self.bm25_deleted), (Ok(true), _) | (_, Ok(true)))
    }

    /// Check if both backends succeeded in deletion
    pub fn both_success(&self) -> bool {
        matches!((&self.vector_deleted, &self.bm25_deleted), (Ok(true), Ok(true)))
    }

    /// Get the first error encountered, if any
    pub fn first_error(&self) -> Option<&AppError> {
        self.vector_deleted
            .as_ref()
            .err()
            .or_else(|| self.bm25_deleted.as_ref().err())
    }
}

/// Hybrid search orchestrator
///
/// Combines results from BM25 and vector search using RRF fusion.
/// This provides better relevance than either method alone.
pub struct HybridOrchestrator {
    /// Vector store for semantic search
    vector_store: Arc<dyn VectorStoreTrait>,
    /// BM25 store for full-text search
    bm25_store: Arc<dyn Bm25StoreTrait>,
    /// RRF fusion orchestrator
    rrf: RrfOrchestrator,
}

impl HybridOrchestrator {
    /// Create a new hybrid orchestrator
    ///
    /// # Parameters
    ///
    /// * `vector_store` - Vector store implementation
    /// * `bm25_store` - BM25 store implementation
    /// * `k` - RRF constant (default: 60)
    ///
    /// # Returns
    ///
    /// Returns a new `HybridOrchestrator` instance.
    pub fn new(
        vector_store: Arc<dyn VectorStoreTrait>,
        bm25_store: Arc<dyn Bm25StoreTrait>,
        k: i32,
    ) -> Self {
        Self {
            vector_store,
            bm25_store,
            rrf: RrfOrchestrator::new(k),
        }
    }

    /// Create with default RRF k=60
    pub fn default_with_stores(
        vector_store: Arc<dyn VectorStoreTrait>,
        bm25_store: Arc<dyn Bm25StoreTrait>,
    ) -> Self {
        Self::new(vector_store, bm25_store, 60)
    }

    /// Perform hybrid search
    ///
    /// Executes both BM25 and vector searches, then fuses results using RRF.
    ///
    /// # Parameters
    ///
    /// * `query` - Search query
    ///
    /// # Returns
    ///
    /// Returns fused and ranked results. Returns empty Vec if no documents match.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Query validation fails
    /// - Both searches fail
    pub async fn search(&self, query: &Query) -> Result<Vec<Hit>, AppError> {
        // Validate query
        if query.text.trim().is_empty() {
            return Err(AppError::Domain(DomainError::invalid_query(
                "Query text cannot be empty",
            )));
        }

        // Execute both searches in parallel
        let (vector_result, bm25_result) = tokio::join!(
            self.vector_store.search(query),
            self.bm25_store.search(query)
        );

        // Process vector search results
        let vector_hits = match vector_result {
            Ok(Some(hits)) if !hits.is_empty() => {
                info!("Vector search returned {} results", hits.len());
                Ok(hits)
            }
            Ok(Some(_)) | Ok(None) => {
                info!("Vector search returned no results");
                Ok(vec![])
            }
            Err(e) => {
                warn!(error = ?e, "Vector search failed, will try BM25 only");
                Err(e)
            }
        };

        // Process BM25 search results
        let bm25_hits = match bm25_result {
            Ok(Some(results)) if !results.is_empty() => {
                info!("BM25 search returned {} results", results.len());
                Ok(results.into_iter().map(|r| r.to_hit()).collect())
            }
            Ok(Some(_)) | Ok(None) => {
                info!("BM25 search returned no results");
                Ok(vec![])
            }
            Err(e) => {
                warn!(error = ?e, "BM25 search failed, will try vector only");
                Err(e)
            }
        };

        // Process results according to exact degradation logic:
        // 1. Both Ok → RRF fusion
        // 2. One Ok, One Err → log warning, return Ok result (degradation)
        // 3. Both Err → combine errors, return AppError (NOT empty array)
        match (vector_hits, bm25_hits) {
            (Ok(v), Ok(b)) => {
                // Both searches succeeded - perform RRF fusion
                // Fuse results using RRF (no .clone() - move ownership)
                let fused = if !v.is_empty() && !b.is_empty() {
                    self.rrf.fuse_two(v, b)  // Move ownership, zero copy
                        .map_err(|e| AppError::Domain(DomainError::Other(format!(
                            "RRF fusion failed: {}", e
                        ))))?
                        .into_iter()
                        .map(|r| r.to_hit())
                        .collect()
                } else if !v.is_empty() {
                    v
                } else {
                    b
                };

                Ok(fused)
            }
            (Ok(v), Err(e)) => {
                // Vector OK, BM25 failed - log warning and return vector results or error
                warn!(error = ?e, "BM25 backend failed, using vector results only");
                if v.is_empty() {
                    // Preserve root cause - don't wrap the error
                    Err(e)
                } else {
                    Ok(v)
                }
            }
            (Err(e), Ok(b)) => {
                // BM25 OK, Vector failed - log warning and return BM25 results or error
                warn!(error = ?e, "Vector backend failed, using BM25 results only");
                if b.is_empty() {
                    // Preserve root cause - don't wrap the error
                    Err(e)
                } else {
                    Ok(b)
                }
            }
            (Err(vec_err), Err(bm25_err)) => {
                // Both searches failed - log both and return first error
                error!(
                    vector_error = ?vec_err,
                    bm25_error = ?bm25_err,
                    "Both search backends failed"
                );
                // Return vector error, log that BM25 also failed
                Err(vec_err)
            }
        }
    }

    /// Add a document to both stores
    ///
    /// This is a convenience method for adding documents to both backends.
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID
    /// * `title` - Document title
    /// * `summary` - Document summary
    /// * `content` - Document content
    /// * `keywords` - Optional space-separated keywords for boosted BM25 ranking
    ///
    /// # Errors
    ///
    /// Returns error if either store fails to add the document.
    pub async fn add(
        &self,
        id: &str,
        title: &str,
        summary: &str,
        content: &str,
        keywords: Option<&str>,
    ) -> Result<(), AppError> {
        // Construct metadata for vector store with title and summary
        let metadata = serde_json::json!({
            "title": title,
            "summary": summary
        });

        // Add to both stores in parallel
        let (vector_result, bm25_result) = tokio::join!(
            self.vector_store.add(id, content, Some(&metadata)),
            self.bm25_store.add(id, title, summary, content, keywords.unwrap_or(""))
        );

        // Handle all four states with compensating rollback to prevent orphan documents
        match (vector_result, bm25_result) {
            (Ok(()), Ok(())) => {
                info!(id = %id, "Document added to both stores successfully");
                Ok(())
            }
            (Ok(()), Err(bm25_err)) => {
                // Vector succeeded but BM25 failed - rollback vector to prevent orphan
                warn!(
                    id = %id,
                    error = ?bm25_err,
                    "BM25 add failed, attempting vector rollback"
                );
                if let Err(rollback_err) = self.vector_store.delete(id).await {
                    error!(
                        id = %id,
                        original_error = ?bm25_err,
                        rollback_error = ?rollback_err,
                        "Failed to rollback vector store - orphan document may exist"
                    );
                } else {
                    info!(id = %id, "Vector store rolled back successfully");
                }
                Err(bm25_err)
            }
            (Err(vector_err), Ok(())) => {
                // BM25 succeeded but Vector failed - rollback BM25 to prevent orphan
                warn!(
                    id = %id,
                    error = ?vector_err,
                    "Vector add failed, attempting BM25 rollback"
                );
                if let Err(rollback_err) = self.bm25_store.delete(id).await {
                    error!(
                        id = %id,
                        original_error = ?vector_err,
                        rollback_error = ?rollback_err,
                        "Failed to rollback BM25 store - orphan document may exist"
                    );
                } else {
                    info!(id = %id, "BM25 store rolled back successfully");
                }
                Err(vector_err)
            }
            (Err(vector_err), Err(bm25_err)) => {
                // Both failed - no rollback needed, log and return first error
                error!(
                    id = %id,
                    vector_error = ?vector_err,
                    bm25_error = ?bm25_err,
                    "Both stores failed to add document"
                );
                Err(vector_err)
            }
        }
    }

    /// Delete a document from both stores
    ///
    /// This is a convenience method for deleting documents from both backends.
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID to delete
    ///
    /// # Returns
    ///
    /// Returns `DeleteResult` containing the individual results from each backend.
    /// This allows the caller to inspect which specific deletion (vector or BM25) failed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = orchestrator.delete("doc-id").await;
    /// if result.any_success() {
    ///     println!("Document deleted from at least one backend");
    /// }
    /// if let Some(e) = result.first_error() {
    ///     eprintln!("One backend failed: {:?}", e);
    /// }
    /// ```
    pub async fn delete(&self, id: &str) -> DeleteResult {
        // Delete from both stores in parallel
        let (vector_result, bm25_result) = tokio::join!(
            self.vector_store.delete(id),
            self.bm25_store.delete(id)
        );

        // Preserve individual results - don't suppress errors
        let vector_deleted = match vector_result {
            Ok(deleted) => Ok(deleted),
            Err(e) => {
                warn!(error = ?e, id = %id, "Vector delete failed");
                Err(e)
            }
        };

        let bm25_deleted = match bm25_result {
            Ok(deleted) => Ok(deleted),
            Err(e) => {
                warn!(error = ?e, id = %id, "BM25 delete failed");
                Err(e)
            }
        };

        // Return detailed results
        DeleteResult {
            vector_deleted,
            bm25_deleted,
        }
    }

    /// Check health of both stores
    ///
    /// Returns true if both stores are healthy.
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let (vector_result, bm25_result) = tokio::join!(
            self.vector_store.health_check(),
            self.bm25_store.health_check()
        );

        // Handle errors gracefully - if one fails, treat as unhealthy
        let vector_healthy = match vector_result {
            Ok(healthy) => healthy,
            Err(e) => {
                warn!(error = ?e, "Vector health check failed, treating as unhealthy");
                false
            }
        };

        let bm25_healthy = match bm25_result {
            Ok(healthy) => healthy,
            Err(e) => {
                warn!(error = ?e, "BM25 health check failed, treating as unhealthy");
                false
            }
        };

        Ok(vector_healthy && bm25_healthy)
    }

    /// Get reference to BM25 store (for advanced usage)
    pub fn bm25_store(&self) -> &Arc<dyn Bm25StoreTrait> {
        &self.bm25_store
    }

    /// Get reference to vector store (for advanced usage)
    pub fn vector_store(&self) -> &Arc<dyn VectorStoreTrait> {
        &self.vector_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::kernel::types::Score;
    use crate::kernel::errors::{AppError, InfraError};
    use crate::slices::bm25::trait_::Bm25Result;

    // Local mock implementations for testing
    struct MockVectorStore {
        should_fail: bool,
        empty_results: bool,
        delete_should_fail: bool,
        add_should_fail: bool,
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

            let hits = vec![
                Hit::new("vec-doc1", Score::new(0.95)),
                Hit::new("vec-doc2", Score::new(0.85)),
            ];
            Ok(Some(hits))
        }

        async fn add(
            &self,
            _id: &str,
            _text: &str,
            _metadata: Option<&serde_json::Value>,
        ) -> Result<(), AppError> {
            if self.add_should_fail {
                Err(AppError::Infra(InfraError::database(
                    "mock vector add failed",
                    None::<std::io::Error>,
                )))
            } else {
                Ok(())
            }
        }

        async fn delete(&self, _id: &str) -> Result<bool, AppError> {
            if self.delete_should_fail {
                Err(AppError::Infra(InfraError::database(
                    "mock vector delete failed",
                    None::<std::io::Error>,
                )))
            } else {
                Ok(true)
            }
        }

        async fn health_check(&self) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    struct MockBm25Store {
        should_fail: bool,
        empty_results: bool,
        delete_should_fail: bool,
        add_should_fail: bool,
    }

    #[async_trait]
    impl Bm25StoreTrait for MockBm25Store {
        async fn search(&self, _query: &Query) -> Result<Option<Vec<Bm25Result>>, AppError> {
            if self.should_fail {
                return Err(AppError::Infra(InfraError::database(
                    "mock connection failed",
                    None::<std::io::Error>,
                )));
            }

            if self.empty_results {
                return Ok(Some(vec![]));
            }

            let results = vec![
                Bm25Result::new("bm25-doc1".to_string(), "Title 1".to_string(), "Summary 1".to_string(), Score::new(0.9)),
                Bm25Result::new("bm25-doc2".to_string(), "Title 2".to_string(), "Summary 2".to_string(), Score::new(0.8)),
            ];
            Ok(Some(results))
        }

        async fn add(
            &self,
            _id: &str,
            _title: &str,
            _summary: &str,
            _content: &str,
            _keywords: &str,
        ) -> Result<(), AppError> {
            if self.add_should_fail {
                Err(AppError::Infra(InfraError::database(
                    "mock BM25 add failed",
                    None::<std::io::Error>,
                )))
            } else {
                Ok(())
            }
        }

        async fn delete(&self, _id: &str) -> Result<bool, AppError> {
            if self.delete_should_fail {
                Err(AppError::Infra(InfraError::database(
                    "mock BM25 delete failed",
                    None::<std::io::Error>,
                )))
            } else {
                Ok(true)
            }
        }

        async fn health_check(&self) -> Result<bool, AppError> {
            Ok(true)
        }

        async fn get_by_id(&self, _id: &str) -> Result<Option<Bm25Result>, AppError> {
            Ok(None)
        }

        async fn get_by_ids(&self, _ids: &[String]) -> Result<Vec<Option<Bm25Result>>, AppError> {
            Ok(vec![])
        }
    }

    /// Helper to create a test orchestrator
    async fn create_test_orchestrator() -> HybridOrchestrator {
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        HybridOrchestrator::default_with_stores(vector_store, bm25_store)
    }

    #[tokio::test]
    async fn test_hybrid_orchestrator_creation() {
        let orchestrator = create_test_orchestrator().await;
        assert!(orchestrator.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_search() {
        let orchestrator = create_test_orchestrator().await;
        let query = Query::new("test query", 10);

        let result = orchestrator.search(&query).await;
        assert!(result.is_ok());

        let hits = result.unwrap();
        // We should get fused results from both stores
        assert!(!hits.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_search_empty_query() {
        let orchestrator = create_test_orchestrator().await;
        let query = Query::new("", 10);

        let result = orchestrator.search(&query).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hybrid_add() {
        let orchestrator = create_test_orchestrator().await;
        let result = orchestrator.add("id", "title", "summary", "content", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_delete() {
        let orchestrator = create_test_orchestrator().await;
        let result = orchestrator.delete("id").await;
        assert!(result.any_success());
        assert!(result.both_success());
        assert!(result.first_error().is_none());
        assert!(matches!(result.vector_deleted, Ok(true)));
        assert!(matches!(result.bm25_deleted, Ok(true)));
    }

    #[tokio::test]
    async fn test_hybrid_health_check() {
        let orchestrator = create_test_orchestrator().await;
        let result = orchestrator.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_hybrid_search_vector_fails_bm25_succeeds() {
        // Test degradation: Vector fails, BM25 succeeds
        let vector_store = Arc::new(MockVectorStore {
            should_fail: true,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let query = Query::new("test query", 10);

        let result = orchestrator.search(&query).await;
        assert!(result.is_ok(), "Should return Ok when BM25 succeeds even if vector fails");

        let hits = result.unwrap();
        assert!(!hits.is_empty(), "Should return BM25 results");
        // Should contain BM25 document IDs
        assert!(hits.iter().any(|h| h.id.starts_with("bm25-doc")), "Should have BM25 results");
    }

    #[tokio::test]
    async fn test_hybrid_search_both_empty_returns_empty() {
        // Test that both stores returning empty is not an error
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: true,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: true,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let query = Query::new("test query", 10);

        let result = orchestrator.search(&query).await;
        assert!(result.is_ok(), "Should return Ok when both stores return empty, not an error");

        let hits = result.unwrap();
        assert!(hits.is_empty(), "Should return empty array when no results found");
    }

    #[tokio::test]
    async fn test_hybrid_search_both_fail() {
        // Test that both stores failing returns an error
        let vector_store = Arc::new(MockVectorStore {
            should_fail: true,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: true,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let query = Query::new("test query", 10);

        let result = orchestrator.search(&query).await;
        assert!(result.is_err(), "Should return error when both stores fail");
    }

    #[tokio::test]
    async fn test_hybrid_delete_vector_fails_bm25_succeeds() {
        // Test delete failure: Vector fails, BM25 succeeds
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: false,
            delete_should_fail: true,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let result = orchestrator.delete("test-id").await;

        assert!(!result.both_success(), "Should not have both success when vector fails");
        assert!(result.any_success(), "Should have BM25 success");
        assert!(result.first_error().is_some(), "Should have vector error");

        assert!(matches!(result.vector_deleted, Err(_)), "Vector delete should fail");
        assert!(matches!(result.bm25_deleted, Ok(true)), "BM25 delete should succeed");
    }

    #[tokio::test]
    async fn test_hybrid_delete_bm25_fails_vector_succeeds() {
        // Test delete failure: BM25 fails, Vector succeeds
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: true,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let result = orchestrator.delete("test-id").await;

        assert!(!result.both_success(), "Should not have both success when BM25 fails");
        assert!(result.any_success(), "Should have vector success");
        assert!(result.first_error().is_some(), "Should have BM25 error");

        assert!(matches!(result.vector_deleted, Ok(true)), "Vector delete should succeed");
        assert!(matches!(result.bm25_deleted, Err(_)), "BM25 delete should fail");
    }

    #[tokio::test]
    async fn test_hybrid_add_vector_succeeds_bm25_fails_rollback_vector() {
        // Test add rollback: Vector succeeds, BM25 fails → verify vector rollback is attempted
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: true,  // BM25 add will fail
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let result = orchestrator.add("test-id", "title", "summary", "content", None).await;

        assert!(result.is_err(), "Should return error when BM25 add fails");

        // Verify that the error is from BM25
        let error = result.unwrap_err();
        assert!(matches!(error, AppError::Infra(_)), "Should be an Infra error from BM25");

        // Note: We can't directly verify the rollback happened without more sophisticated mocking,
        // but the test ensures the error path is exercised and rollback code is executed.
    }

    #[tokio::test]
    async fn test_hybrid_add_bm25_succeeds_vector_fails_rollback_bm25() {
        // Test add rollback: BM25 succeeds, Vector fails → verify BM25 rollback is attempted
        let vector_store = Arc::new(MockVectorStore {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: true,  // Vector add will fail
        });

        let bm25_store = Arc::new(MockBm25Store {
            should_fail: false,
            empty_results: false,
            delete_should_fail: false,
            add_should_fail: false,
        });

        let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
        let result = orchestrator.add("test-id", "title", "summary", "content", None).await;

        assert!(result.is_err(), "Should return error when Vector add fails");

        // Verify that the error is from Vector
        let error = result.unwrap_err();
        assert!(matches!(error, AppError::Infra(_)), "Should be an Infra error from Vector");

        // Note: We can't directly verify the rollback happened without more sophisticated mocking,
        // but the test ensures the error path is exercised and rollback code is executed.
    }
}
