//! Tantivy index management for BM25 full-text search
//!
//! This module handles Tantivy index creation, opening, and initialization.
//! It isolates Tantivy-specific types from the kernel layer.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use anyhow::{Context, Result};
use std::path::Path;
use tantivy::{Index, ReloadPolicy};
use tantivy_jieba::JiebaTokenizer;

use super::schema::{create_bm25_schema, validate_bm25_schema};

/// Create Tantivy index for BM25 full-text search
///
/// Supports two modes:
/// - **Memory index**: No directory parameter, index exists only in memory
/// - **Filesystem index**: Directory path provided, index persists to disk
///
/// For filesystem indexes, if the directory already contains an index,
/// it will be opened and validated; otherwise, a new index will be created.
///
/// # Parameters
///
/// * `directory` - Optional directory path. If None, creates in-memory index.
///
/// # Returns
///
/// Returns `Result<Index>` on success, error on failure.
///
/// # Errors
///
/// Returns error if:
/// - Opening existing index fails
/// - Creating new index fails
/// - Existing index has incompatible schema
///
/// # Examples
///
/// ```ignore
/// use contextfy_core::slices::bm25::create_bm25_index;
///
/// // Create in-memory index
/// let index = create_bm25_index(None).unwrap();
///
/// // Create filesystem index
/// let index = create_bm25_index(Some(Path::new("/tmp/index"))).unwrap();
/// ```
///
/// # Architecture Note
///
/// This is a low-level internal API used by the facade layer.
/// Prefer using `SearchEngine::new()` or `build_hybrid_orchestrator()` instead.
#[doc(hidden)]
pub fn create_bm25_index(directory: Option<&Path>) -> Result<Index> {
    let schema = create_bm25_schema();

    let index = match directory {
        Some(path) => {
            // Try to open existing index first, create if it doesn't exist
            match Index::open_in_dir(path) {
                Ok(idx) => {
                    // IMPORTANT: Validate schema of existing index to catch incompatibility early
                    validate_bm25_schema(&idx.schema())
                        .map_err(|e| anyhow::anyhow!("Existing index has incompatible schema: {}", e))?;
                    idx
                }
                Err(open_err) => {
                    // Create new index if opening failed
                    Index::create_in_dir(path, schema).map_err(|create_err| {
                        anyhow::anyhow!(
                            "Failed to open or create index in directory {}: open error: {}, create error: {}",
                            path.display(),
                            open_err,
                            create_err
                        )
                    })?
                }
            }
        }
        None => Index::create_in_ram(schema),
    };

    // IMPORTANT: Always register Jieba tokenizer, even when reopening existing index
    // Tantivy does NOT persist tokenizer registrations, so we must re-register on every startup
    index.tokenizers().register("jieba", JiebaTokenizer {});

    Ok(index)
}

/// Validate and optionally initialize an existing Tantivy index
///
/// This function checks if an existing index is compatible with our schema.
/// It can be used to verify index compatibility before opening.
///
/// # Parameters
///
/// * `directory` - Directory path containing the index
///
/// # Returns
///
/// * `Ok(())` - Index exists and schema is valid
/// * `Err(anyhow::Error)` - Index doesn't exist or schema is invalid
#[allow(dead_code)]
pub(crate) fn validate_existing_index(directory: &Path) -> Result<()> {
    let index = Index::open_in_dir(directory)
        .context("Failed to open index")?;

    validate_bm25_schema(&index.schema())
        .map_err(|e| anyhow::anyhow!("Schema validation failed: {}", e))
}

/// Initialize BM25 index in directory
///
/// This is a convenience function that ensures a valid index exists
/// in the specified directory. Creates a new index if needed.
///
/// # Parameters
///
/// * `directory` - Directory path for the index
///
/// # Returns
///
/// Returns `Result<Index>` on success.
///
/// # Errors
///
/// Returns error if:
/// - Opening existing index fails
/// - Existing index has incompatible schema
/// - Creating new index fails
#[allow(dead_code)]
pub(crate) fn initialize_bm25_index(directory: &Path) -> Result<Index> {
    match Index::open_in_dir(directory) {
        Ok(idx) => {
            // Validate schema
            validate_bm25_schema(&idx.schema())
                .map_err(|e| anyhow::anyhow!("Existing index has incompatible schema: {}", e))?;

            // CRITICAL: Re-register Jieba tokenizer after reopening
            // Tantivy does NOT persist tokenizer registrations
            idx.tokenizers().register("jieba", JiebaTokenizer {});

            Ok(idx)
        }
        Err(_) => {
            // Create new index (which includes tokenizer registration)
            create_bm25_index(Some(directory))
        }
    }
}

/// Create index reader with automatic reload policy
///
/// This is a convenience function for creating index readers
/// with the recommended reload policy.
///
/// # Parameters
///
/// * `index` - Tantivy index instance
///
/// # Returns
///
/// Returns `Result<IndexReader>` on success.
pub(crate) fn create_index_reader(index: &Index) -> Result<tantivy::IndexReader> {
    index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .context("Failed to create index reader")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::FIELD_TITLE;
    use tempfile::TempDir;

    #[test]
    fn test_create_in_memory_index() {
        let index = create_bm25_index(None);
        assert!(index.is_ok(), "Should create in-memory index successfully");

        let index = index.unwrap();
        // Verify index can be used
        let schema = index.schema();
        assert!(schema.get_field(FIELD_TITLE).is_ok());
    }

    #[test]
    fn test_create_filesystem_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        let index = create_bm25_index(Some(index_path));
        assert!(index.is_ok(), "Should create filesystem index successfully");

        let index = index.unwrap();
        // Verify index can be used
        let schema = index.schema();
        assert!(schema.get_field(FIELD_TITLE).is_ok());

        // Verify index files were created (check for Tantivy's core metadata file)
        assert!(index_path.join("meta.json").exists(), "meta.json should exist after index creation");
    }

    #[test]
    fn test_reopen_existing_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        // Create index
        let index1 = create_bm25_index(Some(index_path));
        assert!(index1.is_ok());

        // Reopen existing index
        let index2 = Index::open_in_dir(index_path);
        assert!(index2.is_ok(), "Should be able to reopen existing index");

        let index2 = index2.unwrap();
        // Verify schema consistency
        assert!(index2.schema().get_field(FIELD_TITLE).is_ok());
    }

    #[test]
    fn test_jieba_tokenizer_registered() {
        // Test in-memory index Jieba tokenizer registration
        let index = create_bm25_index(None).expect("Failed to create in-memory index");
        let tokenizer = index.tokenizers().get("jieba");
        assert!(tokenizer.is_some(), "Jieba tokenizer should be registered");

        // Test filesystem index Jieba tokenizer registration
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        let index = create_bm25_index(Some(index_path)).expect("Failed to create filesystem index");
        let tokenizer = index.tokenizers().get("jieba");
        assert!(
            tokenizer.is_some(),
            "Jieba tokenizer should be registered in filesystem index"
        );
    }

    #[test]
    fn test_validate_existing_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        // Create index first
        create_bm25_index(Some(index_path)).expect("Failed to create index");

        // Validate existing index
        let result = validate_existing_index(index_path);
        assert!(result.is_ok(), "Should validate existing index");
    }

    #[test]
    fn test_initialize_bm25_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        // Initialize new index
        let index = initialize_bm25_index(index_path);
        assert!(index.is_ok(), "Should initialize index");

        // Reopen existing index
        let index2 = initialize_bm25_index(index_path);
        assert!(index2.is_ok(), "Should reopen existing index");

        // CRITICAL: Verify tokenizer is registered after reopening
        // This is a regression test for the tokenizer re-registration bug
        let index2 = index2.unwrap();
        let tokenizer = index2.tokenizers().get("jieba");
        assert!(
            tokenizer.is_some(),
            "Jieba tokenizer MUST be re-registered after reopening existing index"
        );
    }

    #[test]
    fn test_reopen_preserves_tokenizer_registration() {
        // Regression test: Verify that create_bm25_index re-registers tokenizer on reopen
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        // Create index with tokenizer
        let index1 = create_bm25_index(Some(index_path))
            .expect("Failed to create index");
        assert!(
            index1.tokenizers().get("jieba").is_some(),
            "Tokenizer should be registered initially"
        );

        // Reopen index (simulates process restart)
        let index2 = create_bm25_index(Some(index_path))
            .expect("Failed to reopen index");

        // CRITICAL: Tokenizer MUST still be registered after reopening
        // Tantivy does NOT persist tokenizer registrations, so this is required
        assert!(
            index2.tokenizers().get("jieba").is_some(),
            "Tokenizer must be re-registered after reopening (Tantivy doesn't persist it)"
        );
    }

    #[test]
    fn test_create_index_reader() {
        let index = create_bm25_index(None).expect("Failed to create index");
        let reader = create_index_reader(&index);
        assert!(reader.is_ok(), "Should create index reader");
    }
}
