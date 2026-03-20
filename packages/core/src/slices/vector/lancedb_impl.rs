/// LanceDB implementation of VectorStoreTrait
///
/// This module provides the concrete LanceDB backend for vector storage.
/// It implements the VectorStoreTrait while keeping LanceDB-specific
/// types isolated within this module.
///
/// Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use async_trait::async_trait;
use anyhow::{Context, Result as AnyhowResult};
use lancedb::connection::Connection as LanceConnection;
use lancedb::table::Table as LanceTable;

use crate::kernel::types::{Query, Hit, Score};
use crate::kernel::errors::{AppError, InfraError};

use super::trait_::VectorStoreTrait;

/// LanceDB vector store implementation
///
/// This struct holds the LanceDB connection and implements VectorStoreTrait.
///
/// # Fields
///
/// * `conn` - LanceDB connection object
/// * `table_name` - Name of the table to use
pub struct LanceDbStore {
    conn: LanceConnection,
    table_name: String,
}

impl LanceDbStore {
    /// Create a new LanceDB store
    ///
    /// # Parameters
    ///
    /// * `conn` - LanceDB connection object
    /// * `table_name` - Name of the table to use
    ///
    /// # Returns
    ///
    /// Returns a new `LanceDbStore` instance.
    #[allow(dead_code)]
    pub fn new(conn: LanceConnection, table_name: impl Into<String>) -> Self {
        Self {
            conn,
            table_name: table_name.into(),
        }
    }

    /// Get the underlying LanceDB table
    ///
    /// This is a convenience method for internal use.
    async fn get_table(&self) -> AnyhowResult<LanceTable> {
        self.conn
            .open_table(&self.table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table: {}", self.table_name))
    }

    /// Normalize a raw distance score to [0.0, 1.0] range
    ///
    /// LanceDB returns different distance metrics depending on the index type.
    /// This method converts raw distances to normalized relevance scores.
    ///
    /// # Parameters
    ///
    /// * `distance` - Raw distance from LanceDB
    ///
    /// # Returns
    ///
    /// Normalized score in [0.0, 1.0] where 1.0 is best match.
    #[allow(dead_code)]
    fn normalize_score(distance: f32) -> Score {
        // For cosine distance (range [0.0, 2.0]): score = 1 - distance/2
        // For L2 distance: score = 1 / (1 + distance)
        // Default: assume cosine distance
        let normalized = (1.0 - (distance / 2.0)).clamp(0.0, 1.0);
        Score::new(normalized as f64)
    }
}

#[async_trait]
impl VectorStoreTrait for LanceDbStore {
    /// Search for similar vectors
    ///
    /// # Implementation Notes
    ///
    /// 1. Query text should be embedded to vector before calling (currently placeholder)
    /// 2. Performs vector similarity search using LanceDB
    /// 3. Converts results to kernel Hit types with normalized scores
    /// 4. Returns Ok(Some(vec[])) if no results found (not an error)
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder implementation. Actual vector search requires:
    /// - Embedding model integration
    /// - Query vectorization
    /// - Full LanceDB search query construction
    async fn search(&self, query: &Query) -> Result<Option<Vec<Hit>>, AppError> {
        // Phase 1: Placeholder - return empty results
        // Phase 2: Implement actual vector search:
        // 1. Embed query.text to vector
        // 2. Execute LanceDB vector search
        // 3. Convert results to Hit types

        let _ = query; // Suppress unused warning in Phase 1

        // Placeholder: return empty results
        // In Phase 2, this will be:
        // let table = self.get_table().await
        //     .map_err(|e| InfraError::database("search failed", Some(e)))?;
        // let results = table.search(&query_vector)
        //     .limit(query.limit)
        //     .execute()
        //     .await
        //     .map_err(|e| InfraError::database("search failed", Some(e)))?;
        // Convert results to Hit types...

        Ok(Some(vec![]))
    }

    /// Add a document to the vector store
    ///
    /// # Implementation Notes
    ///
    /// 1. Document text should be embedded to vector before calling (currently placeholder)
    /// 2. Adds record with id, text, vector, and metadata to LanceDB
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder implementation. Actual add requires:
    /// - Embedding model integration
    /// - Document vectorization
    /// - LanceDB record insertion
    async fn add(
        &self,
        id: &str,
        text: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<(), AppError> {
        let _ = (id, text, metadata); // Suppress unused warnings in Phase 1

        // Phase 1: Placeholder - do nothing
        // Phase 2: Implement actual insertion:
        // 1. Embed text to vector
        // 2. Create LanceDB record with all fields
        // 3. Insert into table

        Ok(())
    }

    /// Delete a document from the vector store
    ///
    /// # Implementation Notes
    ///
    /// Uses LanceDB's delete operation to remove document by id.
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a placeholder implementation.
    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        let _ = id; // Suppress unused warning in Phase 1

        // Phase 1: Placeholder - return true
        // Phase 2: Implement actual deletion:
        // let table = self.get_table().await
        //     .map_err(|e| InfraError::database("delete failed", Some(e)))?;
        // table.delete(&format!("id == '{}'", id))
        //     .execute()
        //     .await
        //     .map_err(|e| InfraError::database("delete failed", Some(e)))?;

        Ok(true)
    }

    /// Check if the store is healthy and accessible
    ///
    /// # Implementation Notes
    ///
    /// Verifies connection is active and table exists.
    async fn health_check(&self) -> Result<bool, AppError> {
        self.get_table()
            .await
            .map(|_| true)
            .map_err(|e| {
                AppError::Infra(InfraError::database(
                    "health check failed",
                    Some::<anyhow::Error>(e),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slices::vector::connection::{connect, create_table_if_not_exists};

    /// Helper to create a test store
    async fn create_test_store() -> (LanceDbStore, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_knowledge";

        let conn = connect(db_uri)
            .await
            .expect("Failed to connect to LanceDB");

        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to create table");

        let store = LanceDbStore::new(conn, table_name);

        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_lancedb_store_creation() {
        let (store, _temp_dir) = create_test_store().await;

        assert_eq!(store.table_name, "test_knowledge");
    }

    #[tokio::test]
    async fn test_search_placeholder() {
        let (store, _temp_dir) = create_test_store().await;

        let query = Query::new("test query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let hits = result.unwrap().unwrap();
        assert_eq!(hits.len(), 0); // Phase 1: empty results
    }

    #[tokio::test]
    async fn test_add_placeholder() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store
            .add("doc1", "test content", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_placeholder() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store.delete("doc1").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_health_check() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store.health_check().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_health_check_with_invalid_table() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");

        let conn = connect(db_uri)
            .await
            .expect("Failed to connect");

        // Create store with non-existent table
        let store = LanceDbStore::new(conn, "nonexistent_table");

        let result = store.health_check().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::Database { .. }) => {}
            _ => panic!("Expected InfraError::Database"),
        }
    }

    #[test]
    fn test_normalize_score() {
        // Test cosine distance normalization
        // Cosine distance range: [0.0, 2.0]
        // 0.0 distance → 1.0 score (perfect match)
        assert_eq!(LanceDbStore::normalize_score(0.0).value(), 1.0);

        // 1.0 distance → 0.5 score
        assert_eq!(LanceDbStore::normalize_score(1.0).value(), 0.5);

        // 2.0 distance → 0.0 score (worst match)
        assert_eq!(LanceDbStore::normalize_score(2.0).value(), 0.0);

        // Clamping test: value > 2.0 should be clamped to 0.0
        assert_eq!(LanceDbStore::normalize_score(3.0).value(), 0.0);

        // Clamping test: value < 0.0 should be clamped to 1.0
        assert_eq!(LanceDbStore::normalize_score(-0.5).value(), 1.0);
    }
}
