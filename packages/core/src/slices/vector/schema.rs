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
pub fn validate_knowledge_schema(schema: &Schema) -> Result<(), String> {
    let expected = knowledge_record_schema();

    // Check field count
    if schema.fields().len() != expected.fields().len() {
        return Err(format!(
            "Field count mismatch: expected {}, got {}",
            expected.fields().len(),
            schema.fields().len()
        ));
    }

    // Validate each field comprehensively
    for (idx, expected_field) in expected.fields().iter().enumerate() {
        // Get actual field by index (first ensure we have enough fields)
        let actual_field = schema.fields().get(idx).ok_or_else(|| {
            format!(
                "Missing field at index {}: '{}'",
                idx,
                expected_field.name()
            )
        })?;

        // Validate field name
        if actual_field.name() != expected_field.name() {
            return Err(format!(
                "Field name mismatch at index {}: expected '{}', got '{}'",
                idx,
                expected_field.name(),
                actual_field.name()
            ));
        }

        // Validate data type
        if actual_field.data_type() != expected_field.data_type() {
            return Err(format!(
                "Data type mismatch for field '{}': expected {:?}, got {:?}",
                actual_field.name(),
                expected_field.data_type(),
                actual_field.data_type()
            ));
        }

        // Validate nullable flag
        if actual_field.is_nullable() != expected_field.is_nullable() {
            return Err(format!(
                "Nullable flag mismatch for field '{}': expected {}, got {}",
                actual_field.name(),
                expected_field.is_nullable(),
                actual_field.is_nullable()
            ));
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
        let field_names: Vec<_> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            field_names,
            vec![
                "id",
                "title",
                "summary",
                "content",
                "vector",
                "keywords",
                "source_path"
            ]
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
        let err = result.unwrap_err();
        assert!(err.contains("Data type mismatch"));
        assert!(err.contains("vector"));
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
        let err = result.unwrap_err();
        assert!(err.contains("Data type mismatch"));
        assert!(err.contains("vector"));
    }

    #[test]
    fn test_validate_knowledge_schema_wrong_field_name() {
        // Create a schema with wrong field name at index 1
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("subject", DataType::Utf8, false), // Wrong name, should be "title"
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Field name mismatch"));
        assert!(err.contains("index 1"));
        assert!(err.contains("expected 'title'"));
        assert!(err.contains("got 'subject'"));
    }

    #[test]
    fn test_validate_knowledge_schema_wrong_nullable_flag() {
        // Create a schema with wrong nullable flag for id field
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, true), // Should be non-null
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Nullable flag mismatch"));
        assert!(err.contains("id"));
    }

    #[test]
    fn test_validate_knowledge_schema_wrong_data_type() {
        // Create a schema with wrong data type for title field
        let wrong_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Int64, false), // Wrong type, should be Utf8
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&wrong_schema);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Data type mismatch"));
        assert!(err.contains("title"));
    }

    #[test]
    fn test_validate_knowledge_schema_all_fields_correct() {
        // Create a schema that exactly matches the expected schema
        let correct_schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("source_path", DataType::Utf8, false),
        ]);

        let result = validate_knowledge_schema(&correct_schema);
        assert!(
            result.is_ok(),
            "Schema validation should succeed for correct schema"
        );
    }
}
