//! Tantivy schema definitions for BM25 full-text search
//!
//! This module contains the Tantivy schema definitions used by the BM25 backend.
//! This is isolated to the BM25 slice to avoid leaking Tantivy types into the kernel.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use tantivy::schema::{Schema, TextFieldIndexing, TextOptions, STORED};

/// Document field name constants
///
/// These constants define the field names used in the Tantivy index.
pub(crate) const FIELD_ID: &str = "id";
pub(crate) const FIELD_TITLE: &str = "title";
pub(crate) const FIELD_SUMMARY: &str = "summary";
pub(crate) const FIELD_CONTENT: &str = "content";
pub(crate) const FIELD_KEYWORDS: &str = "keywords";

/// Create Tantivy schema for BM25 full-text search
///
/// This schema defines the structure of documents stored in Tantivy.
///
/// # Fields
///
/// - `id`: Unique record identifier (STRING, STORED, not tokenized)
/// - `title`: Document title (TEXT, TOKENIZED, STORED, with jieba tokenizer)
/// - `summary`: Document summary (TEXT, TOKENIZED, STORED, with jieba tokenizer)
/// - `content`: Document content (TEXT, TOKENIZED, STORED, with jieba tokenizer)
/// - `keywords`: Document keywords (TEXT, TOKENIZED, STORED, with jieba tokenizer)
///
/// # Tokenization
///
/// All TEXT fields use the Jieba tokenizer for Chinese text segmentation.
/// The tokenizer must be registered with the index before searching.
///
/// # Invariants
///
/// - ID field is STRING type for exact matching (not tokenized)
/// - TEXT fields support tokenization and are stored for retrieval
/// - Jieba tokenizer with name "jieba" must be registered on the index
pub(crate) fn create_bm25_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Create Jieba tokenizer text indexing configuration
    let text_indexing = TextFieldIndexing::default().set_tokenizer("jieba");

    // Create text field options: tokenized + stored + custom tokenizer
    let text_options = TextOptions::default()
        .set_indexing_options(text_indexing)
        .set_stored();

    // Add id field (STRING type, exact match, not tokenized)
    schema_builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);

    // Add TEXT fields with tokenization and storage, using Jieba tokenizer
    schema_builder.add_text_field(FIELD_TITLE, text_options.clone());
    schema_builder.add_text_field(FIELD_SUMMARY, text_options.clone());
    schema_builder.add_text_field(FIELD_CONTENT, text_options.clone());
    schema_builder.add_text_field(FIELD_KEYWORDS, text_options);

    schema_builder.build()
}

/// Validate that a schema matches the expected BM25 schema
///
/// This is used to verify that an existing Tantivy index is compatible
/// with our expected schema. The validation performs comprehensive checks:
///
/// # Validation Approach
///
/// The validation uses a two-tier approach:
///
/// 1. **Field type comparison** (primary): Compares the complete `FieldType` enum
///    which includes `TextOptions` containing:
///    - Indexing options (tokenizer name, index record option, fieldnorms)
///    - Stored flag
///    - Fast field options
///    - Coerce flag
///
///    This comprehensive comparison catches most differences (tokenizer,
///    storage flags, indexing options) in a single check.
///
/// 2. **Field-specific validation** (secondary): After the type comparison passes,
///    performs additional checks for specific fields:
///    - ID field: Validates use of "raw" tokenizer (STRING type, not tokenized)
///    - TEXT fields: Validates use of "jieba" tokenizer for Chinese text
///    - All fields: Validates that they are stored
///
/// # Validation Checks
///
/// 1. Field count matches expected count
/// 2. All required field names exist
/// 3. Each field has the correct `FieldType` (including all options)
/// 4. ID field uses "raw" tokenizer (no tokenization)
/// 5. TEXT fields use "jieba" tokenizer (Chinese text segmentation)
/// 6. All fields are stored
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
pub(crate) fn validate_bm25_schema(schema: &Schema) -> Result<(), String> {
    let expected = create_bm25_schema();

    // Check field count by collecting iterator
    let field_count = schema.fields().count();
    let expected_count = expected.fields().count();

    if field_count != expected_count {
        return Err(format!(
            "Field count mismatch: expected {}, got {}",
            expected_count, field_count
        ));
    }

    // Verify all expected fields exist with correct types and properties
    for field_name in &[
        FIELD_ID,
        FIELD_TITLE,
        FIELD_SUMMARY,
        FIELD_CONTENT,
        FIELD_KEYWORDS,
    ] {
        // Check field exists
        let field = schema
            .get_field(field_name)
            .map_err(|e| format!("Missing field '{}': {}", field_name, e))?;

        // Get the field entry
        let entry = schema.get_field_entry(field);

        // Get expected field entry for comparison
        let expected_field = expected.get_field(field_name).map_err(|_| {
            format!(
                "Expected field '{}' not found in expected schema",
                field_name
            )
        })?;
        let expected_entry = expected.get_field_entry(expected_field);

        // Validate field type and properties match
        // This catches differences in tokenizer, stored flag, indexing options, etc.
        if entry.field_type() != expected_entry.field_type() {
            return Err(format!(
                "Field '{}' type mismatch: expected {:?}, got {:?}",
                field_name,
                expected_entry.field_type(),
                entry.field_type()
            ));
        }

        // For TEXT fields (all except ID), validate tokenizer and storage
        if *field_name != FIELD_ID {
            // TEXT fields should have indexing options with tokenizer
            let text_options = match entry.field_type() {
                tantivy::schema::FieldType::Str(opts) => opts,
                _ => return Err(format!("Field '{}' should be Str(TEXT) type", field_name)),
            };

            let indexing = text_options.get_indexing_options().ok_or_else(|| {
                format!("Field '{}' is TEXT but has no indexing options", field_name)
            })?;

            let tokenizer = indexing.tokenizer();
            if tokenizer != "jieba" {
                return Err(format!(
                    "Field '{}' has incorrect tokenizer: expected 'jieba', got '{}'",
                    field_name, tokenizer
                ));
            }

            // Verify TEXT fields are stored
            if !text_options.is_stored() {
                return Err(format!(
                    "Field '{}' should be STORED but is not",
                    field_name
                ));
            }
        } else {
            // ID field should be STRING type (indexed but not tokenized with "raw" tokenizer)
            let text_options = match entry.field_type() {
                tantivy::schema::FieldType::Str(opts) => opts,
                _ => return Err(format!("Field '{}' should be Str type", field_name)),
            };

            // ID field should have indexing options with "raw" tokenizer (not tokenized)
            let indexing = text_options.get_indexing_options().ok_or_else(|| {
                format!(
                    "Field '{}' should have indexing options with 'raw' tokenizer",
                    field_name
                )
            })?;

            let tokenizer = indexing.tokenizer();
            if tokenizer != "raw" {
                return Err(format!(
                    "Field '{}' has incorrect tokenizer: expected 'raw' (for STRING type), got '{}'",
                    field_name, tokenizer
                ));
            }

            // Verify ID field is stored
            if !text_options.is_stored() {
                return Err(format!(
                    "Field '{}' should be STORED but is not",
                    field_name
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::schema::TEXT;

    #[test]
    fn test_create_bm25_schema() {
        let schema = create_bm25_schema();

        // Verify 5 fields
        assert_eq!(schema.fields().count(), 5);

        // Verify field names
        let field_names: Vec<_> = schema
            .fields()
            .map(|(f, _entry)| schema.get_field_name(f))
            .collect();
        assert_eq!(
            field_names,
            vec![
                FIELD_ID,
                FIELD_TITLE,
                FIELD_SUMMARY,
                FIELD_CONTENT,
                FIELD_KEYWORDS
            ]
        );
    }

    #[test]
    fn test_validate_bm25_schema_valid() {
        let schema = create_bm25_schema();
        assert!(validate_bm25_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_bm25_schema_field_count_mismatch() {
        // Create a schema with wrong field count
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Field count mismatch"));
    }

    #[test]
    fn test_validate_bm25_schema_missing_field() {
        // Create a schema with correct field count but missing title field
        // We need to add an extra field to keep count at 5
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        builder.add_text_field("extra_field", TEXT | STORED); // Extra field to maintain count
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing field 'title'"));
    }

    #[test]
    fn test_validate_bm25_schema_wrong_field_type() {
        // Create a schema where ID field is TEXT instead of STRING
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, TEXT | STORED); // Wrong: should be STRING
        builder.add_text_field(FIELD_TITLE, TEXT | STORED);
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Field 'id' type mismatch"));
    }

    #[test]
    fn test_validate_bm25_schema_missing_tokenizer() {
        // Create a schema where TEXT fields don't have jieba tokenizer
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);
        // Add TEXT fields without custom tokenizer (uses default)
        builder.add_text_field(FIELD_TITLE, TEXT | STORED);
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        // Field type comparison catches the tokenizer difference
        assert!(err_msg.contains("type mismatch") || err_msg.contains("tokenizer"));
    }

    #[test]
    fn test_validate_bm25_schema_not_stored() {
        // Create a schema where a field is not stored
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);
        builder.add_text_field(FIELD_TITLE, TEXT); // Not stored
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        // Field type comparison catches the stored flag difference
        assert!(err_msg.contains("type mismatch") || err_msg.contains("STORED"));
    }

    #[test]
    fn test_validate_bm25_schema_id_tokenized() {
        // Create a schema where ID field has jieba tokenizer (should use "raw" tokenizer instead)
        let text_indexing = TextFieldIndexing::default().set_tokenizer("jieba");
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, text_options); // Wrong: ID should use "raw" tokenizer
        builder.add_text_field(FIELD_TITLE, TEXT | STORED);
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        // Field type comparison catches the tokenizer difference
        assert!(
            err_msg.contains("type mismatch")
                || err_msg.contains("tokenizer")
                || err_msg.contains("raw")
        );
    }

    #[test]
    fn test_field_constants() {
        assert_eq!(FIELD_ID, "id");
        assert_eq!(FIELD_TITLE, "title");
        assert_eq!(FIELD_SUMMARY, "summary");
        assert_eq!(FIELD_CONTENT, "content");
        assert_eq!(FIELD_KEYWORDS, "keywords");
    }
}
