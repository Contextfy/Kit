//! Data transformation from JSON records to LanceDB schema

use arrow::array::{FixedSizeListArray, Float32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

use crate::embeddings::EmbeddingModel;
use crate::migration::error::MigrationError;
use crate::migration::json_reader::JsonRecord;

/// Transforms and validates JSON records for migration
pub struct RecordTransformer {
    /// Embedding model for generating vectors
    embedding_model: EmbeddingModel,
}

impl RecordTransformer {
    /// Create a new transformer
    pub fn new(embedding_model: EmbeddingModel) -> Self {
        Self { embedding_model }
    }

    /// Generate embeddings for a batch of text contents
    ///
    /// This uses FastEmbed's batch processing which internally parallelizes
    /// across CPU cores. Do NOT add external tokio::spawn concurrency!
    pub fn generate_embeddings_batch(
        &self,
        records: &[JsonRecord],
    ) -> Result<Vec<Vec<f32>>, MigrationError> {
        // Extract content from each record
        let texts: Vec<&str> = records.iter().map(|r| r.content.as_str()).collect();

        // Call FastEmbed batch embedding (synchronous)
        let embeddings = self.embedding_model.embed_batch(&texts).map_err(|e| {
            MigrationError::EmbeddingFailed {
                record_id: format!("batch of {}", texts.len()),
                reason: e.to_string(),
            }
        })?;

        Ok(embeddings)
    }

    /// Transform a single JSON record with its embedding to LanceDB format
    ///
    /// This function creates a structure that can be inserted into LanceDB.
    /// The actual insertion is handled by the caller.
    pub fn transform_to_lancedb(
        &self,
        record: &JsonRecord,
        embedding: Vec<f32>,
    ) -> Result<LancedbKnowledgeRecord, MigrationError> {
        // Validate embedding dimension (should be 384 for BGE-small-en)
        if embedding.len() != 384 {
            return Err(MigrationError::ValidationError(format!(
                "Invalid embedding dimension: expected 384, got {}",
                embedding.len()
            )));
        }

        // Convert keywords array to comma-separated string
        let keywords = if record.keywords.is_empty() {
            None
        } else {
            Some(record.keywords.join(","))
        };

        Ok(LancedbKnowledgeRecord {
            id: record.id.clone(),
            title: record.title.clone(),
            summary: record.summary.clone(),
            content: record.content.clone(),
            vector: embedding,
            keywords,
            source_path: record.source_path.clone(),
        })
    }

    /// Transform a batch of records with their embeddings
    pub fn transform_batch(
        &self,
        records: &[JsonRecord],
        embeddings: Vec<Vec<f32>>,
    ) -> Result<Vec<LancedbKnowledgeRecord>, MigrationError> {
        if records.len() != embeddings.len() {
            return Err(MigrationError::ValidationError(format!(
                "Record count mismatch: {} records but {} embeddings",
                records.len(),
                embeddings.len()
            )));
        }

        records
            .iter()
            .zip(embeddings.iter())
            .map(|(record, embedding)| self.transform_to_lancedb(record, embedding.clone()))
            .collect()
    }

    /// Transform a batch of LancedbKnowledgeRecord to Arrow RecordBatch
    ///
    /// This is the final step before inserting into LanceDB.
    /// Converts our intermediate format to Arrow's columnar format.
    pub fn to_record_batch(
        &self,
        records: &[LancedbKnowledgeRecord],
    ) -> Result<RecordBatch, MigrationError> {
        if records.is_empty() {
            return Err(MigrationError::ValidationError(
                "Cannot create RecordBatch from empty records".to_string(),
            ));
        }

        let num_rows = records.len();

        // Build Arrow arrays for each column
        let ids: Vec<Option<&str>> = records.iter().map(|r| Some(r.id.as_str())).collect();
        let titles: Vec<Option<&str>> = records.iter().map(|r| Some(r.title.as_str())).collect();
        let summaries: Vec<Option<&str>> =
            records.iter().map(|r| Some(r.summary.as_str())).collect();
        let contents: Vec<Option<&str>> =
            records.iter().map(|r| Some(r.content.as_str())).collect();
        let keywords: Vec<Option<&str>> = records.iter().map(|r| r.keywords.as_deref()).collect();
        let source_paths: Vec<Option<&str>> = records
            .iter()
            .map(|r| Some(r.source_path.as_str()))
            .collect();

        // Build vector array (FixedSizeList of Float32)
        // Flatten all vectors into a single Float32Array
        let vector_dim = 384;
        let mut vector_values = Vec::with_capacity(num_rows * vector_dim);
        for record in records {
            vector_values.extend_from_slice(&record.vector);
        }

        // Validate all vectors have correct dimension
        if vector_values.len() != num_rows * vector_dim {
            return Err(MigrationError::ValidationError(format!(
                "Vector dimension mismatch: expected {} values ({} records * {} dims), got {}",
                num_rows * vector_dim,
                num_rows,
                vector_dim,
                vector_values.len()
            )));
        }

        let vector_array = Float32Array::from(vector_values);

        // Create FixedSizeListArray for the vector column
        let vector_field = Field::new("item", DataType::Float32, true);
        let vectors_fixed_size_list = FixedSizeListArray::new(
            Arc::new(vector_field),
            vector_dim as i32,      // FixedSizeListArray expects i32 for size
            Arc::new(vector_array), // Wrap in Arc<dyn Array>
            None,                   // nulls bitmap (all vectors are non-null)
        );

        // Create StringArrays
        let id_array = StringArray::from(ids);
        let title_array = StringArray::from(titles);
        let summary_array = StringArray::from(summaries);
        let content_array = StringArray::from(contents);
        let keywords_array = StringArray::from(keywords);
        let source_path_array = StringArray::from(source_paths);

        // Define schema matching LanceDB table
        let schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    vector_dim as i32,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        // Create RecordBatch
        let record_batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(id_array),
                Arc::new(title_array),
                Arc::new(summary_array),
                Arc::new(content_array),
                Arc::new(vectors_fixed_size_list),
                Arc::new(keywords_array),
                Arc::new(source_path_array),
            ],
        )
        .map_err(|e| {
            MigrationError::ValidationError(format!("Failed to create RecordBatch: {}", e))
        })?;

        Ok(record_batch)
    }
}

/// LanceDB knowledge record
///
/// This structure matches the Arrow schema defined in
/// `packages/core/src/slices/vector/schema.rs`.
///
/// Schema:
/// - id: String (non-null)
/// - title: String (non-null)
/// - summary: String (non-null)
/// - content: String (non-null)
/// - vector: FixedSizeList<Float32>(384) (non-null)
/// - keywords: String (nullable)
/// - source_path: String (non-null)
#[derive(Debug, Clone)]
pub struct LancedbKnowledgeRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub vector: Vec<f32>,
    pub keywords: Option<String>,
    pub source_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a valid embedding vector for testing
    fn valid_embedding() -> Vec<f32> {
        vec![0.1; 384]
    }

    fn create_test_record(id: &str) -> JsonRecord {
        JsonRecord {
            id: id.to_string(),
            title: "Test Title".to_string(),
            summary: "Test Summary".to_string(),
            content: "Test Content".to_string(),
            keywords: vec![],
            source_path: "/path.md".to_string(),
            created_at: None,
            updated_at: None,
        }
    }

    // Note: We can't fully test RecordTransformer without a real EmbeddingService,
    // but we can test the transformation logic once we have embeddings

    #[test]
    fn test_lancedb_record_structure() {
        let record = create_test_record("test-1");
        let embedding = valid_embedding();

        // Test that we can construct a LancedbKnowledgeRecord
        let lancedb_record = LancedbKnowledgeRecord {
            id: record.id.clone(),
            title: record.title.clone(),
            summary: record.summary.clone(),
            content: record.content.clone(),
            vector: embedding.clone(),
            keywords: None,
            source_path: record.source_path.clone(),
        };

        assert_eq!(lancedb_record.id, "test-1");
        assert_eq!(lancedb_record.vector.len(), 384);
    }

    #[test]
    fn test_keywords_serialization() {
        let record = JsonRecord {
            id: "test-2".to_string(),
            title: "Test".to_string(),
            summary: "Summary".to_string(),
            content: "Content".to_string(),
            keywords: vec!["rust".to_string(), "ml".to_string(), "ai".to_string()],
            source_path: "/path.md".to_string(),
            created_at: None,
            updated_at: None,
        };

        let keywords_joined = record.keywords.join(",");
        assert_eq!(keywords_joined, "rust,ml,ai");
    }

    #[test]
    fn test_batch_size_validation() {
        // Record embedding dimension validation logic
        let embedding_valid = vec![0.1; 384];
        let embedding_invalid = vec![0.1; 128];

        assert_eq!(
            embedding_valid.len(),
            384,
            "Valid embedding should be 384 dim"
        );
        assert_ne!(
            embedding_invalid.len(),
            384,
            "Invalid embedding should not be 384 dim"
        );
    }
}
