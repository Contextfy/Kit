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
pub(crate) fn validate_bm25_schema(schema: &Schema) -> Result<(), String> {
    let expected = create_bm25_schema();

    // Check field count by collecting iterator
    let field_count = schema.fields().count();
    let expected_count = expected.fields().count();

    if field_count != expected_count {
        return Err(format!(
            "Field count mismatch: expected {}, got {}",
            expected_count,
            field_count
        ));
    }

    // Verify all expected fields exist
    for field_name in &[FIELD_ID, FIELD_TITLE, FIELD_SUMMARY, FIELD_CONTENT, FIELD_KEYWORDS] {
        schema
            .get_field(field_name)
            .map_err(|e| format!("Missing field '{}': {}", field_name, e))?;
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
            vec![FIELD_ID, FIELD_TITLE, FIELD_SUMMARY, FIELD_CONTENT, FIELD_KEYWORDS]
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
        // Create a schema without title field
        let mut builder = Schema::builder();
        builder.add_text_field(FIELD_ID, tantivy::schema::STRING | STORED);
        builder.add_text_field(FIELD_SUMMARY, TEXT | STORED);
        builder.add_text_field(FIELD_CONTENT, TEXT | STORED);
        builder.add_text_field(FIELD_KEYWORDS, TEXT | STORED);
        let wrong_schema = builder.build();

        let result = validate_bm25_schema(&wrong_schema);
        assert!(result.is_err());
        // Field count is checked first, so we expect that error
        assert!(result.unwrap_err().contains("Field count mismatch"));
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
