use anyhow::{Context, Result as AnyhowResult};
/// LanceDB implementation of VectorStoreTrait
///
/// This module provides the concrete LanceDB backend for vector storage.
/// It implements the VectorStoreTrait while keeping LanceDB-specific
/// types isolated within this module.
///
/// Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2
use async_trait::async_trait;
use arrow::array::{Float32Array, RecordBatch, StringArray};
use arrow::record_batch::RecordBatchIterator;
use lancedb::connection::Connection as LanceConnection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table as LanceTable;
use std::sync::Arc;
use futures::StreamExt;

use crate::embeddings::EmbeddingModel;
use crate::kernel::errors::{AppError, InfraError};
use crate::kernel::types::{Hit, Query, Score};

use super::trait_::VectorStoreTrait;

/// Distance metric types supported by the vector store
///
/// Defines how vector similarity is calculated. Different metrics
/// have different ranges and normalization requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Reserved for future metric selection feature
pub enum DistanceMetric {
    /// Cosine distance (range: [0.0, 2.0])
    /// 0.0 = identical vectors, 2.0 = orthogonal vectors
    Cosine,

    /// Euclidean/L2 distance (range: [0.0, +infinity))
    /// 0.0 = identical vectors, higher values = more different
    L2,

    /// Dot product (range: [-infinity, +infinity))
    /// Higher values = more similar, but unbounded and sign-dependent
    Dot,
}

/// LanceDB vector store implementation
///
/// This struct holds the LanceDB connection and implements VectorStoreTrait.
///
/// # Fields
///
/// * `conn` - LanceDB connection object
/// * `table_name` - Name of the table to use
/// * `embedding_model` - Embedding model for vectorizing text
pub struct LanceDbStore {
    conn: LanceConnection,
    table_name: String,
    embedding_model: Arc<EmbeddingModel>,
}

impl LanceDbStore {
    /// Create a new LanceDB store
    ///
    /// # Parameters
    ///
    /// * `conn` - LanceDB connection object
    /// * `table_name` - Name of the table to use
    /// * `embedding_model` - Embedding model for vectorizing text
    ///
    /// # Returns
    ///
    /// Returns a new `LanceDbStore` instance.
    pub fn new(
        conn: LanceConnection,
        table_name: impl Into<String>,
        embedding_model: Arc<EmbeddingModel>,
    ) -> Self {
        Self {
            conn,
            table_name: table_name.into(),
            embedding_model,
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
    /// * `metric` - The distance metric type to use for normalization
    ///
    /// # Returns
    ///
    /// Normalized score in [0.0, 1.0] where 1.0 is best match.
    #[allow(dead_code)]
    fn normalize_score(distance: f32, metric: DistanceMetric) -> Score {
        let normalized = match metric {
            // Cosine distance (range [0.0, 2.0]): score = 1 - distance/2
            DistanceMetric::Cosine => (1.0 - (distance / 2.0)).clamp(0.0, 1.0),

            // L2 distance (range [0.0, +infinity)): score = 1 / (1 + distance)
            DistanceMetric::L2 => (1.0 / (1.0 + distance)).clamp(0.0, 1.0),

            // Dot product (range [-infinity, +infinity))
            // Use sigmoid-like normalization: score = 1 / (1 + exp(-distance))
            DistanceMetric::Dot => {
                // Sigmoid function to map unbounded dot product to [0, 1]
                let sigmoid = 1.0 / (1.0 + (-distance as f64).exp());
                sigmoid as f32
            }
        };

        Score::new(normalized as f64)
    }
}

#[async_trait]
impl VectorStoreTrait for LanceDbStore {
    /// Search for similar vectors
    ///
    /// # Implementation Notes
    ///
    /// 1. Query text is embedded to vector using BGE-small-en model
    /// 2. Performs vector similarity search using LanceDB
    /// 3. Converts results to kernel Hit types with normalized scores
    /// 4. Returns Ok(Some(vec[])) if no results found (not an error)
    ///
    /// # Phase 2 Implementation
    ///
    /// - Generates embedding vector for the query text
    /// - Executes LanceDB vector search with the query vector
    /// - Converts LanceDB results to Hit types
    async fn search(&self, query: &Query) -> Result<Option<Vec<Hit>>, AppError> {
        // Step 1: Generate embedding vector for the query text
        let query_vector = self
            .embedding_model
            .embed_text(&query.text)
            .map_err(|e| AppError::Infra(InfraError::Other(format!(
                "Failed to generate query embedding: {}",
                e
            ))))?;

        // Step 2: Get the LanceDB table
        let table = self
            .get_table()
            .await
            .map_err(|e| AppError::Infra(InfraError::database(
                "Failed to open table for search",
                Some(e),
            )))?;

        // Step 3: Execute vector search using LanceDB's query API
        // API: table.query().nearest_to(query_vector).limit(n).execute().await
        // Note: IntoQueryVector is implemented for Vec<f32>, so we pass query_vector directly
        let vector_query = table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| AppError::Infra(InfraError::database(
                "Failed to create vector query",
                Some(e),
            )))?
            .limit(query.limit);

        // Execute the query
        let mut results_stream: lancedb::arrow::SendableRecordBatchStream = vector_query
            .execute()
            .await
            .map_err(|e| AppError::Infra(InfraError::database(
                "Vector search execution failed",
                Some(e),
            )))?;

        // Step 4: Collect results from the stream and convert to Hit types
        let mut hits = Vec::new();

        while let Some(batch_result) = results_stream.next().await {
            let batch = batch_result.map_err(|e| {
                AppError::Infra(InfraError::database(
                    "Failed to read result batch",
                    Some(e),
                ))
            })?;

            // Get the _distance column (auto-added by LanceDB)
            let distance_col = batch
                .column_by_name("_distance")
                .ok_or_else(|| {
                    AppError::Infra(InfraError::Other(
                        "Missing _distance column in search results".to_string(),
                    ))
                })?;

            let distances = distance_col
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| {
                    AppError::Infra(InfraError::Other(
                        "Failed to cast _distance column to Float32Array".to_string(),
                    ))
                })?;

            // Get the id column
            let id_col = batch.column_by_name("id").ok_or_else(|| {
                AppError::Infra(InfraError::Other(
                    "Missing id column in search results".to_string(),
                ))
            })?;

            let ids = id_col.as_any().downcast_ref::<StringArray>().ok_or_else(|| {
                AppError::Infra(InfraError::Other(
                    "Failed to cast id column to StringArray".to_string(),
                ))
            })?;

            // Convert each row to a Hit
            for row in 0..batch.num_rows() {
                let distance = distances.value(row);
                let id = ids.value(row).to_string();

                // Normalize the distance to a relevance score
                // LanceDB uses L2 distance by default, which ranges [0, +infinity)
                // We convert to [0, 1] where 1.0 is best match
                let score = Self::normalize_score(distance, DistanceMetric::L2);

                hits.push(Hit { id, score });
            }
        }

        if hits.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hits))
        }
    }

    /// Add a document to the vector store
    ///
    /// # Implementation Notes
    ///
    /// 1. Document text is embedded to vector using BGE-small-en model
    /// 2. Adds record with id, title, summary, content, vector, and keywords to LanceDB
    ///
    /// # Phase 2 Implementation
    ///
    /// - Generates embedding vector for the text content
    /// - Extracts title and summary from metadata
    /// - Creates LanceDB record and inserts into table
    async fn add(
        &self,
        id: &str,
        text: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<(), AppError> {
        // Step 1: Generate embedding vector for the text
        let embedding = self
            .embedding_model
            .embed_text(text)
            .map_err(|e| AppError::Infra(InfraError::Other(format!(
                "Failed to generate embedding: {}",
                e
            ))))?;

        // Step 2: Extract title and summary from metadata
        let (title, summary, keywords) = if let Some(meta) = metadata {
            let title = meta
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let summary = meta
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let keywords = meta.get("keywords").and_then(|v| v.as_str());
            (title, summary, keywords)
        } else {
            (String::new(), String::new(), None)
        };

        // Step 3: Get the LanceDB table
        let table = self
            .get_table()
            .await
            .map_err(|e| AppError::Infra(InfraError::database(
                "Failed to open table for add",
                Some(e),
            )))?;

        // Step 4: Create Arrow record batch using shared schema
        // Use the canonical schema from schema.rs to ensure alignment with table.add()
        use crate::slices::vector::schema::{knowledge_record_schema, VECTOR_DIM};
        use arrow::array::{FixedSizeListArray, Float32Array};

        // Import the canonical schema
        let schema = Arc::new(knowledge_record_schema());

        // Create arrays directly (must match schema field order)
        let id_array = StringArray::from(vec![id]);
        let title_array = StringArray::from(vec![title.as_str()]);
        let summary_array = StringArray::from(vec![summary.as_str()]);
        let content_array = StringArray::from(vec![text]);
        let keywords_array = StringArray::from(vec![keywords]);
        let source_path_array = StringArray::from(vec!["unknown"]);

        // Create FixedSizeListArray for vector (use VECTOR_DIM constant)
        let vector_values = Float32Array::from(embedding.clone());
        // The field should describe the item inside the list (Float32), not the list itself
        // FixedSizeListArray::new() will wrap it in FixedSizeList automatically
        let vector_item_field = arrow::datatypes::Field::new("item", arrow::datatypes::DataType::Float32, false);
        let vector_array = FixedSizeListArray::new(
            Arc::new(vector_item_field),
            VECTOR_DIM, // Use constant instead of hardcoded 384
            Arc::new(vector_values),
            None, // null bitmap (None means all values are valid)
        );

        // Create the record batch
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as Arc<dyn arrow::array::Array>,
                Arc::new(title_array) as Arc<dyn arrow::array::Array>,
                Arc::new(summary_array) as Arc<dyn arrow::array::Array>,
                Arc::new(content_array) as Arc<dyn arrow::array::Array>,
                Arc::new(vector_array) as Arc<dyn arrow::array::Array>,
                Arc::new(keywords_array) as Arc<dyn arrow::array::Array>,
                Arc::new(source_path_array) as Arc<dyn arrow::array::Array>,
            ],
        )
        .map_err(|e| {
            AppError::Infra(InfraError::Other(format!("Failed to create RecordBatch: {}", e)))
        })?;

        // Step 5: Wrap in RecordBatchIterator and add to table
        let batches = vec![batch];
        let reader = RecordBatchIterator::new(
            batches.into_iter().map(Ok),
            schema,
        );

        table
            .add(reader)
            .execute()
            .await
            .map_err(|e| AppError::Infra(InfraError::database(
                "Failed to add record to LanceDB",
                Some(e),
            )))?;

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
        //
        // SECURITY WARNING: The following line is vulnerable to SQL injection!
        // table.delete(&format!("id == '{}'", id))
        //
        // DO NOT USE string formatting for query parameters. An attacker could inject
        // malicious SQL by crafting an id like: "doc1' OR '1'='1"
        //
        // Instead, use parameter binding or proper escaping. LanceDB supports safer
        // alternatives such as:
        //
        // Option 1: Use LanceDB's builder API with proper parameter binding (preferred):
        // table.delete()
        //     .only_if("id == $1")
        //     .execute(&[id])  // Parameter binding prevents injection
        //
        // Option 2: If using raw filter strings, properly escape the input:
        // let escaped_id = id.replace('\\', "\\\\").replace('\'', "\\'");
        // table.delete(&format!("id == '{}'", escaped_id))
        //
        // Option 3: Use LanceDB's expression builder if available:
        // use lancedb::dsl::col;
        // table.delete(col("id").eq(lancedb::Literal::new(id)))
        //
        // Reference: https://lancedb.github.io/lancedb/security/
        //
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
        self.get_table().await.map(|_| true).map_err(|e| {
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

    /// Helper to create a test store with fake embedding backend
    async fn create_test_store() -> (LanceDbStore, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_knowledge";

        let conn = connect(db_uri).await.expect("Failed to connect to LanceDB");

        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to create table");

        // Use test stub to avoid expensive model download in unit tests
        let embedding_model = Arc::new(EmbeddingModel::test_stub());

        let store = LanceDbStore::new(conn, table_name, embedding_model);

        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_lancedb_store_creation() {
        let (store, _temp_dir) = create_test_store().await;

        assert_eq!(store.table_name, "test_knowledge");
    }

    #[tokio::test]
    async fn test_search_and_add() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a test document
        let metadata = serde_json::json!({
            "title": "Test Document",
            "summary": "A test document for vector search"
        });

        let add_result = store.add("doc1", "test content for search", Some(&metadata)).await;
        assert!(add_result.is_ok(), "Add should succeed");

        // Search for similar documents
        let query = Query::new("test content", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok(), "Search should succeed");
        let hits = result.unwrap();
        assert!(hits.is_some(), "Should return Some(hits), not None");

        let hits = hits.unwrap();
        assert!(hits.len() > 0, "Should find at least one result");
        assert_eq!(hits[0].id, "doc1", "First hit should be doc1");
    }

    #[tokio::test]
    async fn test_add_with_metadata() {
        let (store, _temp_dir) = create_test_store().await;

        let metadata = serde_json::json!({
            "title": "Test Title",
            "summary": "Test Summary"
        });

        let result = store.add("doc1", "test content", Some(&metadata)).await;

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

        let conn = connect(db_uri).await.expect("Failed to connect");

        // Use test stub to avoid expensive model download
        let embedding_model = Arc::new(EmbeddingModel::test_stub());

        // Create store with non-existent table
        let store = LanceDbStore::new(conn, "nonexistent_table", embedding_model);

        let result = store.health_check().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::Database { .. }) => {}
            _ => panic!("Expected InfraError::Database"),
        }
    }

    #[test]
    fn test_normalize_score_cosine() {
        // Test cosine distance normalization
        // Cosine distance range: [0.0, 2.0]
        // 0.0 distance → 1.0 score (perfect match)
        assert_eq!(
            LanceDbStore::normalize_score(0.0, DistanceMetric::Cosine).value(),
            1.0
        );

        // 1.0 distance → 0.5 score
        assert_eq!(
            LanceDbStore::normalize_score(1.0, DistanceMetric::Cosine).value(),
            0.5
        );

        // 2.0 distance → 0.0 score (worst match)
        assert_eq!(
            LanceDbStore::normalize_score(2.0, DistanceMetric::Cosine).value(),
            0.0
        );

        // Clamping test: value > 2.0 should be clamped to 0.0
        assert_eq!(
            LanceDbStore::normalize_score(3.0, DistanceMetric::Cosine).value(),
            0.0
        );

        // Clamping test: value < 0.0 should be clamped to 1.0
        assert_eq!(
            LanceDbStore::normalize_score(-0.5, DistanceMetric::Cosine).value(),
            1.0
        );
    }

    #[test]
    fn test_normalize_score_l2() {
        // Test L2 distance normalization
        // L2 distance range: [0.0, +infinity)
        // 0.0 distance → 1.0 score (perfect match)
        assert!(
            (LanceDbStore::normalize_score(0.0, DistanceMetric::L2).value() - 1.0).abs()
                < f64::EPSILON
        );

        // 1.0 distance → 0.5 score
        assert!(
            (LanceDbStore::normalize_score(1.0, DistanceMetric::L2).value() - 0.5).abs()
                < f64::EPSILON
        );

        // Large distance → small score
        assert!(LanceDbStore::normalize_score(10.0, DistanceMetric::L2).value() < 0.1);

        // Very large distance → very small score
        assert!(LanceDbStore::normalize_score(100.0, DistanceMetric::L2).value() < 0.01);
    }

    #[test]
    fn test_normalize_score_dot() {
        // Test dot product normalization (sigmoid)
        // Dot product range: [-infinity, +infinity)

        // High positive dot product → score near 1.0
        let score_high = LanceDbStore::normalize_score(5.0, DistanceMetric::Dot).value();
        assert!(score_high > 0.95);

        // Dot product of 0 → score of 0.5
        let score_zero = LanceDbStore::normalize_score(0.0, DistanceMetric::Dot).value();
        assert!((score_zero - 0.5).abs() < 0.01);

        // Negative dot product → score near 0.0
        let score_negative = LanceDbStore::normalize_score(-5.0, DistanceMetric::Dot).value();
        assert!(score_negative < 0.05);
    }
}
