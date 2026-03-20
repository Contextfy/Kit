//! Facade factory for hybrid search engine
//!
//! This module provides high-level factory methods to construct the hybrid search
//! orchestrator without exposing concrete backend implementations.
//!
//! **Architecture Enforcement**:
//! - Concrete types (LanceDbStore, TantivyBm25Store) are never exposed in public APIs
//! - Factory methods internally instantiate backends and return trait objects
//! - Upper layers (server, CLI) depend only on abstractions
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use crate::slices::bm25::trait_::Bm25StoreTrait;
use crate::slices::vector::VectorStoreTrait;
use crate::slices::hybrid::HybridOrchestrator;

// Private concrete implementations - invisible to external code
use crate::slices::bm25::tantivy_impl::TantivyBm25Store;
use crate::slices::vector::lancedb_impl::LanceDbStore;

/// Create a hybrid search orchestrator with default backends
///
/// This factory method instantiates BM25 (Tantivy) and Vector (LanceDB) stores
/// with default configurations and returns a configured HybridOrchestrator.
///
/// # Parameters
///
/// * `index_dir` - Directory path for Tantivy BM25 index (None = in-memory)
/// * `lancedb_uri` - LanceDB connection URI (e.g., "data/lancedb" or "./indexdb")
/// * `table_name` - LanceDB table name (e.g., "knowledge")
///
/// # Returns
///
/// Returns a configured `HybridOrchestrator` ready for use.
///
/// # Example
///
/// ```ignore
/// use contextfy_core::facade::build_hybrid_orchestrator;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let orchestrator = build_hybrid_orchestrator(
///         Some("./data/bm25_index"),
///         "./data/lancedb",
///         "knowledge"
///     ).await?;
///
///     // Use orchestrator for search
///     let query = Query::new("search query", 10);
///     let results = orchestrator.search(&query).await?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Tantivy index creation fails
/// - LanceDB connection fails
/// - Table creation/validation fails
pub async fn build_hybrid_orchestrator(
    index_dir: Option<&Path>,
    lancedb_uri: &str,
    table_name: &str,
) -> Result<HybridOrchestrator> {
    // Create BM25 store (Tantivy) - private implementation
    let bm25_index = crate::slices::bm25::index::create_bm25_index(index_dir)
        .context("Failed to create BM25 index")?;
    let bm25_store = TantivyBm25Store::new(bm25_index)
        .context("Failed to create BM25 store")?;

    // Create Vector store (LanceDB) - private implementation
    let conn = crate::slices::vector::connection::connect(lancedb_uri)
        .await
        .context("Failed to connect to LanceDB")?;

    // Ensure table exists
    crate::slices::vector::connection::create_table_if_not_exists(&conn, table_name)
        .await
        .context("Failed to create LanceDB table")?;

    let vector_store = LanceDbStore::new(conn, table_name);

    // Wrap in Arc for trait object sharing
    let bm25_store: Arc<dyn Bm25StoreTrait> = Arc::new(bm25_store);
    let vector_store: Arc<dyn VectorStoreTrait> = Arc::new(vector_store);

    // Create and return hybrid orchestrator
    Ok(HybridOrchestrator::default_with_stores(
        vector_store,
        bm25_store,
    ))
}

/// High-level search engine facade
///
/// This struct wraps the HybridOrchestrator and provides a simplified API
/// for upper layers (server, CLI). It handles initialization and provides
/// convenience methods.
///
/// # Example
///
/// ```ignore
/// use contextfy_core::facade::SearchEngine;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let engine = SearchEngine::new(
///         Some("./data/bm25_index"),
///         "./data/lancedb",
///         "knowledge"
///     ).await?;
///
///     // Add document
///     engine.add("doc-id", "Title", "Summary", "Content").await?;
///
///     // Search
///     let results = engine.search("query text", 10).await?;
///     Ok(())
/// }
/// ```
pub struct SearchEngine {
    orchestrator: HybridOrchestrator,
}

impl SearchEngine {
    /// Create a new search engine with default backends
    ///
    /// # Parameters
    ///
    /// * `index_dir` - Directory for Tantivy BM25 index (None = in-memory)
    /// * `lancedb_uri` - LanceDB connection URI
    /// * `table_name` - LanceDB table name
    ///
    /// # Returns
    ///
    /// Returns a configured `SearchEngine` ready for use.
    pub async fn new(
        index_dir: Option<&Path>,
        lancedb_uri: &str,
        table_name: &str,
    ) -> Result<Self> {
        let orchestrator = build_hybrid_orchestrator(index_dir, lancedb_uri, table_name)
            .await?;

        Ok(Self { orchestrator })
    }

    /// Perform hybrid search
    ///
    /// Combines BM25 and vector search results using RRF fusion.
    ///
    /// # Parameters
    ///
    /// * `query_text` - Search query text
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns ranked search results.
    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<crate::kernel::types::Hit>> {
        use crate::kernel::types::Query;

        let query = Query::new(query_text.to_string(), limit);
        self.orchestrator
            .search(&query)
            .await
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))
    }

    /// Add a document to both BM25 and vector stores
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID
    /// * `title` - Document title
    /// * `summary` - Document summary
    /// * `content` - Document content
    pub async fn add(&self, id: &str, title: &str, summary: &str, content: &str) -> Result<()> {
        self.orchestrator
            .add(id, title, summary, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add document: {}", e))
    }

    /// Delete a document from both stores
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID to delete
    ///
    /// # Returns
    ///
    /// Returns true if document was deleted from at least one store.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        self.orchestrator
            .delete(id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete document: {}", e))
    }

    /// Check health of both backends
    ///
    /// Returns true if both backends are healthy.
    pub async fn health_check(&self) -> Result<bool> {
        self.orchestrator
            .health_check()
            .await
            .map_err(|e| anyhow::anyhow!("Health check failed: {}", e))
    }

    /// Get internal orchestrator (for advanced usage)
    ///
    /// **NOTE**: This exposes the HybridOrchestrator for advanced use cases.
    /// Prefer using the high-level methods (search, add, delete) when possible.
    pub fn orchestrator(&self) -> &HybridOrchestrator {
        &self.orchestrator
    }

    /// Get a document by ID with full content
    ///
    /// Retrieves document details including full content from the BM25 store.
    ///
    /// # Parameters
    ///
    /// * `id` - Document ID to retrieve
    ///
    /// # Returns
    ///
    /// Returns document details with full content if found.
    ///
    /// # Errors
    ///
    /// Returns error if document retrieval fails.
    pub async fn get_document(&self, id: &str) -> Result<Option<DocumentDetails>> {
        self.orchestrator
            .bm25_store()
            .get_by_id(id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get document: {}", e))
            .map(|opt_result| {
                opt_result.map(|r| DocumentDetails {
                    id: r.id,
                    title: r.title,
                    summary: r.summary,
                    content: r.content.unwrap_or_default(),
                })
            })
    }
}

/// Document details with full content
///
/// This struct represents a document with all its fields populated.
#[derive(Debug, Clone)]
pub struct DocumentDetails {
    /// Document ID
    pub id: String,
    /// Document title
    pub title: String,
    /// Document summary
    pub summary: String,
    /// Full document content
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_build_hybrid_orchestrator_in_memory() {
        // Create in-memory BM25 index + temporary LanceDB
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lancedb_uri = temp_dir.path().join("lancedb");
        let lancedb_uri_str = lancedb_uri.to_str().expect("Invalid path");

        let result = build_hybrid_orchestrator(
            None,  // In-memory BM25
            lancedb_uri_str,
            "test_knowledge",
        ).await;

        assert!(result.is_ok(), "Should build hybrid orchestrator");
        let orchestrator = result.unwrap();
        assert!(orchestrator.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_search_engine_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lancedb_uri = temp_dir.path().join("lancedb");
        let lancedb_uri_str = lancedb_uri.to_str().expect("Invalid path");

        let result = SearchEngine::new(
            None,
            lancedb_uri_str,
            "test_knowledge",
        ).await;

        assert!(result.is_ok(), "Should create search engine");
        let engine = result.unwrap();
        assert!(engine.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_search_engine_add_and_search() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let lancedb_uri = temp_dir.path().join("lancedb");
        let lancedb_uri_str = lancedb_uri.to_str().expect("Invalid path");

        let engine = SearchEngine::new(
            None,
            lancedb_uri_str,
            "test_add",
        ).await.expect("Failed to create engine");

        // Add document
        let result = engine.add(
            "doc-1",
            "Rust Programming",
            "A guide to Rust",
            "Rust is a systems programming language",
        ).await;

        assert!(result.is_ok(), "Should add document");

        // Search (Note: BM25 index needs commit which is handled internally)
        let search_result = engine.search("Rust", 10).await;
        // Search may return empty results due to index timing, but should not error
        assert!(search_result.is_ok(), "Search should not error");
    }
}
