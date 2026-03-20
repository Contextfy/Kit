//! LanceDB schema definitions
//!
//! This module contains the Arrow schema definitions used by the LanceDB backend.
//! This is isolated to the vector slice to avoid leaking Arrow types into the kernel.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

/// Vector dimension constant
///
/// This value must match the embedding model being used:
/// - BGE-small-en-v1.5: 384
/// - BGE-base-en-v1.5: 768
/// - text-embedding-ada-002 (OpenAI): 1536
/// - bge-m3 (multilingual): 1024
///
/// Current value is configured for BGE-small-en-v1.5 (project default).
///
/// **NOTE**: Uses `i32` type to match Arrow DataType::FixedSizeList requirements.
#[allow(dead_code)]
pub(crate) const VECTOR_DIM: i32 = 384;

/// Knowledge record Arrow schema
///
/// This schema defines the structure of documents stored in LanceDB.
///
/// # Fields
///
/// - `id`: Unique record identifier (Utf8, non-null)
/// - `title`: Record title (Utf8, non-null)
/// - `summary`: Content summary for Scout retrieval (Utf8, non-null)
/// - `content`: Full content for Inspect retrieval (Utf8, non-null)
/// - `vector`: Vector embedding (384-dim FixedSizeList(Float32), non-null)
/// - `keywords`: JSON-serialized keyword array (Utf8, nullable)
/// - `source_path`: Original file path (Utf8, non-null)
///
/// # Invariants
///
/// - Vector dimension must match `VECTOR_DIM` constant
/// - Vector elements must be Float32
/// - Only `keywords` field is nullable
#[allow(dead_code)]
pub(crate) fn knowledge_record_schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("summary", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        // vector: 384-dim Float32 fixed-size list
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                VECTOR_DIM,
            ),
            false,
        ),
        // keywords: JSON-serialized array (stored as string)
        Field::new("keywords", DataType::Utf8, true),
        Field::new("source_path", DataType::Utf8, false),
    ])
}

/// Validate that a schema matches the expected knowledge record schema
///
/// This is used to verify that an existing LanceDB table is compatible
/// with our expected schema.
///
/// # Parameters
///
/// * `schema` - The schema to validate
///
/// # Returns
///
/// * `Ok(())` - Schema is valid
/// * `Err(String)` - Schema validation failed with descriptive message
#[allow(dead_code)]
pub(crate) fn validate_knowledge_schema(schema: &Schema) -> Result<(), String> {
    let expected = knowledge_record_schema();

    // Check field count
    if schema.fields().len() != expected.fields().len() {
        return Err(format!(
            "Field count mismatch: expected {}, got {}",
            expected.fields().len(),
            schema.fields().len()
        ));
    }

    // Validate vector field specifically
    let vector_field = schema
        .field_with_name("vector")
        .map_err(|e| format!("Missing 'vector' field: {}", e))?;

    match vector_field.data_type() {
        DataType::FixedSizeList(field, size) => {
            if *size != VECTOR_DIM {
                return Err(format!(
                    "Vector dimension mismatch: expected {}, got {}",
                    VECTOR_DIM, size
                ));
            }
            if field.data_type() != &DataType::Float32 {
                return Err(format!(
                    "Vector element type mismatch: expected Float32, got {:?}",
                    field.data_type()
                ));
            }
        }
        _ => {
            return Err(format!(
                "Vector field is not FixedSizeList, got {:?}",
                vector_field.data_type()
            ))
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_record_schema() {
        let schema = knowledge_record_schema();

        // Verify 7 fields
        assert_eq!(schema.fields().len(), 7);

        // Verify field names
        let field_names: Vec<_> = schema
            .fields()
            .iter()
            .map(|f| f.name().as_str())
            .collect();
        assert_eq!(
            field_names,
            vec!["id", "title", "summary", "content", "vector", "keywords", "source_path"]
        );

        // Verify id field type
        let id_field = schema.field(0);
        assert_eq!(id_field.data_type(), &DataType::Utf8);
        assert!(!id_field.is_nullable());

        // Verify vector field type and dimension
        let vector_field = schema.field(4);
        assert!(!vector_field.is_nullable());
        match vector_field.data_type() {
            DataType::FixedSizeList(field, size) => {
                assert_eq!(*size, VECTOR_DIM);
                assert_eq!(field.data_type(), &DataType::Float32);
            }
            _ => panic!("vector field should be FixedSizeList"),
        }

        // Verify keywords field is nullable
        let keywords_field = schema.field(5);
        assert!(keywords_field.is_nullable());
    }

    #[test]
    fn test_validate_knowledge_schema_valid() {
        let schema = knowledge_record_schema();
        assert!(validate_knowledge_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_knowledge_schema_field_count_mismatch() {
        // Create a schema with wrong field count
        let wrong_schema = Schema::new(vec![Field::new("id", DataType::Utf8, false)]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Field count mismatch"));
    }

    #[test]
    fn test_validate_knowledge_schema_missing_vector() {
        // Create a schema without vector field
        // Note: Field count mismatch will be detected before missing vector
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        // Field count is checked first, so we get that error
        assert!(result.unwrap_err().contains("Field count mismatch"));
    }

    #[test]
    fn test_validate_knowledge_schema_wrong_vector_dimension() {
        // Create a schema with wrong vector dimension
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    128, // Wrong dimension
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Vector dimension mismatch"));
    }

    #[test]
    fn test_validate_knowledge_schema_wrong_vector_type() {
        // Create a schema with wrong vector element type
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float64, true)), // Wrong type
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Vector element type mismatch"));
    }
}
