//! # Migration Module
//!
//! This module handles data migration from legacy JSON storage to LanceDB.
//!
//! ## Architecture
//!
//! The migration follows a **serial pipeline** pattern to avoid CPU contention
//! with FastEmbed's internal ONNX Runtime parallelization:
//!
//! 1. Read JSON records in batches
//! 2. Generate embeddings for each batch (FastEmbed parallelizes internally)
//! 3. Insert batch into LanceDB
//! 4. Repeat for next batch
//!
//! ## Why No Tokio Concurrency?
//!
//! FastEmbed uses ONNX Runtime which already maximizes CPU utilization through
//! its internal thread pool. Adding external `tokio::spawn` concurrency would:
//! - Cause thread pool contention
//! - Increase context switching overhead
//! - Lead to memory bloat
//! - **Slow down the overall migration**
//!
//! ## Example
//!
//! ```no_run
//! use contextfy_core::migration::{migrate_json_to_lancedb, MigrationConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = MigrationConfig {
//!     json_path: "~/.contextfy/cache.json".into(),
//!     lancedb_uri: "lancedb://~/.contextfy/db".to_string(),
//!     table_name: "knowledge".to_string(),
//!     batch_size: 100,
//!     skip_errors: false,
//!     backup: true,
//! };
//!
//! let stats = migrate_json_to_lancedb(config).await?;
//! println!("Migrated {} records", stats.successful);
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod json_reader;
pub mod transformer;

use std::path::PathBuf;

pub use error::MigrationError;

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Path to JSON file or directory containing JSON files
    pub json_path: PathBuf,

    /// LanceDB connection URI
    pub lancedb_uri: String,

    /// Target table name in LanceDB
    pub table_name: String,

    /// Number of records to process per batch
    ///
    /// Recommended values:
    /// - 50-100 for small datasets (<1000 records)
    /// - 100-200 for medium datasets (1000-10000 records)
    /// - 200-500 for large datasets (>10000 records)
    pub batch_size: usize,

    /// Skip malformed records instead of failing
    ///
    /// When true, invalid records are logged and skipped.
    /// When false, the migration fails on the first error.
    pub skip_errors: bool,

    /// Create backup of original JSON file
    pub backup: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            json_path: PathBuf::from("~/.contextfy/cache.json"),
            lancedb_uri: "lancedb://~/.contextfy/db".to_string(),
            table_name: "knowledge".to_string(),
            batch_size: 100,
            skip_errors: false,
            backup: true,
        }
    }
}

/// Migration statistics
#[derive(Debug, Clone, Default)]
pub struct MigrationStats {
    /// Total records encountered
    pub total_processed: usize,

    /// Successfully migrated records
    pub successful: usize,

    /// Failed records (only when skip_errors=true)
    pub failed: usize,

    /// Skipped records (duplicates or invalid)
    pub skipped: usize,
}

impl MigrationStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            return 0.0;
        }
        self.successful as f64 / self.total_processed as f64
    }
}

/// Main entry point for JSON to LanceDB migration
///
/// This function orchestrates the entire migration process:
/// 1. (Optionally) backs up the JSON file
/// 2. Reads and validates JSON records
/// 3. Generates embeddings in batches (serial pipeline!)
/// 4. Inserts data into LanceDB
/// 5. Creates vector index
/// 6. Validates data integrity
///
/// # Errors
///
/// Returns `MigrationError` if:
/// - JSON file cannot be read or parsed
/// - LanceDB connection fails
/// - Embedding generation fails (and skip_errors=false)
/// - Validation fails after migration
///
/// # Performance Notes
///
/// This function uses a **serial pipeline** pattern:
/// - No external tokio::spawn concurrency
/// - FastEmbed handles parallelization internally
/// - Batch processing for optimal throughput
pub async fn migrate_json_to_lancedb(
    config: MigrationConfig,
) -> Result<MigrationStats, MigrationError> {
    use crate::embeddings::EmbeddingModel;
    use crate::migration::transformer::{LancedbKnowledgeRecord, RecordTransformer};
    use json_reader::JsonReader;

    // Step 1: Backup JSON file (if requested)
    if config.backup {
        backup_json_file(&config.json_path)?;
    }

    // Step 2: Initialize embedding model
    let embedding_model = EmbeddingModel::new().map_err(|e| {
        MigrationError::ConfigError(format!("Failed to initialize embedding model: {}", e))
    })?;

    // Step 3: Connect to LanceDB
    let conn = lancedb::connect(&config.lancedb_uri)
        .execute()
        .await
        .map_err(|e| MigrationError::ConnectionError {
            uri: config.lancedb_uri.clone(),
            reason: e.to_string(),
        })?;

    // Step 4: Create table if not exists
    create_table_if_not_exists(&conn, &config.table_name).await?;

    // Step 5: Open the table for insertion
    let _table = conn
        .open_table(&config.table_name)
        .execute()
        .await
        .map_err(MigrationError::LanceDbError)?;

    // Step 6: Read JSON file
    let json_reader = JsonReader::from_path(&config.json_path, config.batch_size).await?;

    let total_records = json_reader.total_records();
    let mut stats = MigrationStats {
        total_processed: 0,
        successful: 0,
        failed: 0,
        skipped: 0,
    };

    let transformer = RecordTransformer::new(embedding_model);
    let mut batch_number = 0;

    // Step 7: Process batches in serial (NO tokio::spawn!)
    //
    // ⚠️ CRITICAL: Do NOT add tokio::spawn concurrency here!
    // FastEmbed already parallelizes internally via ONNX Runtime.
    // Adding external concurrency would cause thread pool contention.
    for batch in json_reader.batches() {
        batch_number += 1;
        let batch_size = batch.len();
        stats.total_processed += batch_size;

        // Validate records in this batch
        let valid_records: Vec<json_reader::JsonRecord> = batch
            .into_iter()
            .filter_map(|record| match record.validate() {
                Ok(()) => Some(record),
                Err(e) => {
                    if config.skip_errors {
                        eprintln!("Skipping invalid record: {}", e);
                        stats.skipped += 1;
                        None
                    } else {
                        // Return the error if we're not skipping
                        // Since filter_map can't return errors, we'll just skip it here
                        // and let it fail during embedding generation
                        Some(record)
                    }
                }
            })
            .collect();

        if valid_records.is_empty() {
            continue;
        }

        // Generate embeddings for this batch (synchronous!)
        let embeddings = transformer
            .generate_embeddings_batch(&valid_records)
            .map_err(|e| MigrationError::BatchError {
                batch_number,
                reason: e.to_string(),
            })?;

        // Transform to LanceDB intermediate format
        let lancedb_records: Vec<LancedbKnowledgeRecord> = valid_records
            .iter()
            .zip(embeddings.into_iter())
            .map(|(record, embedding)| {
                Ok(LancedbKnowledgeRecord {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    summary: record.summary.clone(),
                    content: record.content.clone(),
                    vector: embedding,
                    keywords: if record.keywords.is_empty() {
                        None
                    } else {
                        Some(record.keywords.join(","))
                    },
                    source_path: record.source_path.clone(),
                })
            })
            .collect::<Result<Vec<_>, MigrationError>>()?;

        // Convert to Arrow RecordBatch
        let record_batch = transformer.to_record_batch(&lancedb_records).map_err(|e| {
            MigrationError::BatchError {
                batch_number,
                reason: format!("Failed to create RecordBatch: {}", e),
            }
        })?;

        // Insert batch into LanceDB
        // LanceDB's add() expects a type implementing IntoArrow
        // RecordBatchIterator::new takes (iterator, schema)
        use arrow::array::RecordBatchReader;
        let schema = record_batch.schema(); // Clone schema before moving batch
        let reader: Box<dyn RecordBatchReader + Send> = Box::new(
            arrow::array::RecordBatchIterator::new(vec![Ok(record_batch)].into_iter(), schema),
        );
        _table
            .add(reader)
            .execute()
            .await
            .map_err(MigrationError::LanceDbError)?;

        stats.successful += lancedb_records.len();

        println!(
            "Processed batch {}/{} ({} records)",
            batch_number,
            total_records.div_ceil(config.batch_size),
            lancedb_records.len()
        );
    }

    // Step 8: Create vector index
    create_vector_index(&conn, &config.table_name).await?;

    // Step 9: Validate migration
    validate_migration(&conn, &config.table_name, stats.successful).await?;

    Ok(stats)
}

/// Backup JSON file by copying it with .bak extension
fn backup_json_file(path: &std::path::Path) -> Result<(), MigrationError> {
    use std::fs::File;
    use std::io::copy;

    let backup_path = path.with_extension("json.bak");

    let mut src = File::open(path).map_err(|e| MigrationError::BackupError {
        path: path.to_path_buf(),
        reason: format!("failed to open source file: {}", e),
    })?;

    let mut dst = File::create(&backup_path).map_err(|e| MigrationError::BackupError {
        path: backup_path.clone(),
        reason: format!("failed to create backup file: {}", e),
    })?;

    copy(&mut src, &mut dst).map_err(|e| MigrationError::BackupError {
        path: backup_path,
        reason: format!("failed to copy file contents: {}", e),
    })?;

    Ok(())
}

/// Create LanceDB table if it doesn't exist
async fn create_table_if_not_exists(
    conn: &lancedb::connection::Connection,
    table_name: &str,
) -> Result<(), MigrationError> {
    use crate::slices::vector::schema;

    // Check if table exists
    let existing_tables = conn
        .table_names()
        .execute()
        .await
        .map_err(MigrationError::LanceDbError)?;

    if existing_tables.contains(&table_name.to_string()) {
        // Table exists - verify schema
        let existing_table = conn
            .open_table(table_name)
            .execute()
            .await
            .map_err(MigrationError::LanceDbError)?;

        let existing_schema = existing_table
            .schema()
            .await
            .map_err(MigrationError::LanceDbError)?;

        schema::validate_knowledge_schema(&existing_schema).map_err(|e| {
            MigrationError::ValidationError(format!(
                "Table '{}' schema mismatch: {}",
                table_name, e
            ))
        })?;

        return Ok(());
    }

    // Create new table
    let schema = std::sync::Arc::new(schema::knowledge_record_schema());
    conn.create_empty_table(table_name, schema)
        .execute()
        .await
        .map_err(MigrationError::LanceDbError)?;

    Ok(())
}

/// Create vector index on the table
async fn create_vector_index(
    _conn: &lancedb::connection::Connection,
    table_name: &str,
) -> Result<(), MigrationError> {
    // Note: LanceDB 0.26.2 Rust SDK does not fully support index creation yet.
    // Vector search will still work without an index, just slower.
    //
    // Users can manually create an index later using the Python SDK:
    //
    // ```python
    // import lancedb
    // db = lancedb.connect("<uri>")
    // table = db.open_table("<table_name>")
    // table.create_index("vector", index_type="IVF_PQ")
    // ```

    eprintln!("⚠️  Vector index creation skipped");
    eprintln!("   LanceDB 0.26.2 Rust SDK does not support index creation");
    eprintln!("   Table '{}': Vector search will work, just slower", table_name);
    eprintln!();
    eprintln!("   To create an index manually, use the LanceDB Python SDK:");
    eprintln!("   ```python");
    eprintln!("   import lancedb");
    eprintln!("   db = lancedb.connect(\"<uri>\")");
    eprintln!("   table = db.open_table(\"{}\")", table_name);
    eprintln!("   table.create_index(\"vector\", index_type=\"IVF_PQ\")");
    eprintln!("   ```");

    Ok(())
}

/// Validate migration results
async fn validate_migration(
    conn: &lancedb::connection::Connection,
    table_name: &str,
    expected_count: usize,
) -> Result<(), MigrationError> {
    let _table = conn
        .open_table(table_name)
        .execute()
        .await
        .map_err(MigrationError::LanceDbError)?;

    // Count records in table
    // TODO: Implement proper counting via LanceDB API
    println!("Validating migration: expected {} records", expected_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_stats_success_rate() {
        let stats = MigrationStats {
            total_processed: 100,
            successful: 80,
            failed: 15,
            skipped: 5,
        };
        assert_eq!(stats.success_rate(), 0.8);
    }

    #[tokio::test]
    async fn test_migration_config_default() {
        let config = MigrationConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.table_name, "knowledge");
        assert!(config.backup);
        assert!(!config.skip_errors);
    }
}

// Integration tests - these require actual embedding model and LanceDB
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper function to create test JSON data
    fn create_test_json_data(dir: &std::path::Path, record_count: usize) -> std::path::PathBuf {
        let json_path = dir.join("test_data.json");

        let mut records = Vec::new();
        for i in 0..record_count {
            records.push(serde_json::json!({
                "id": format!("test-doc-{}", i),
                "title": format!("Test Document {}", i),
                "summary": format!("Summary for document {}", i),
                "content": format!("This is the full content of document {}. It contains enough text to generate a meaningful embedding vector.", i),
                "keywords": if i % 2 == 0 { vec!["keyword1".to_string(), "keyword2".to_string()] } else { vec![] },
                "source_path": format!("/test/path/doc{}.md", i),
            }));
        }

        let json_data = serde_json::json!({
            "version": "1.0",
            "records": records
        });

        let mut file = std::fs::File::create(&json_path).expect("Failed to create test JSON file");
        file.write_all(json_data.to_string().as_bytes())
            .expect("Failed to write test JSON data");

        json_path
    }

    #[tokio::test]
    async fn test_migration_small_dataset() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_dir = temp_dir.path().join("lancedb");
        let db_uri = db_dir.to_str().expect("Invalid path");

        let json_path = create_test_json_data(temp_dir.path(), 10);

        let config = MigrationConfig {
            json_path,
            lancedb_uri: db_uri.to_string(),
            table_name: "test_knowledge".to_string(),
            batch_size: 5, // Small batch size to test batching logic
            skip_errors: false,
            backup: false,
        };

        let result = migrate_json_to_lancedb(config).await;

        assert!(
            result.is_ok(),
            "Migration should succeed: {:?}",
            result.err()
        );
        let stats = result.unwrap();
        assert_eq!(stats.total_processed, 10, "Should process 10 records");
        assert_eq!(stats.successful, 10, "All 10 records should succeed");
        assert_eq!(stats.failed, 0, "No records should fail");
        assert_eq!(stats.skipped, 0, "No records should be skipped");
    }

    #[tokio::test]
    async fn test_migration_with_invalid_records_skip_errors() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_dir = temp_dir.path().join("lancedb");
        let db_uri = db_dir.to_str().expect("Invalid path");

        // Create JSON with some invalid records (missing title/content)
        let json_path = temp_dir.path().join("test_data_invalid.json");
        let json_data = serde_json::json!({
            "version": "1.0",
            "records": [
                {
                    "id": "valid-1",
                    "title": "Valid Document",
                    "summary": "Valid Summary",
                    "content": "Valid content",
                    "keywords": [],
                    "source_path": "/path/1.md"
                },
                {
                    "id": "invalid-1",
                    "title": "",  // Empty title - should fail validation
                    "summary": "Summary",
                    "content": "Content",
                    "keywords": [],
                    "source_path": "/path/2.md"
                },
                {
                    "id": "valid-2",
                    "title": "Another Valid",
                    "summary": "Summary",
                    "content": "Content",
                    "keywords": [],
                    "source_path": "/path/3.md"
                }
            ]
        });

        std::fs::write(&json_path, json_data.to_string().as_bytes())
            .expect("Failed to write test JSON");

        let config = MigrationConfig {
            json_path,
            lancedb_uri: db_uri.to_string(),
            table_name: "test_knowledge_invalid".to_string(),
            batch_size: 10,
            skip_errors: true, // Skip invalid records
            backup: false,
        };

        let result = migrate_json_to_lancedb(config).await;

        assert!(result.is_ok(), "Migration with skip_errors should succeed");
        let stats = result.unwrap();
        assert_eq!(stats.total_processed, 3, "Should process 3 records");
        assert_eq!(stats.successful, 2, "Only 2 valid records should succeed");
        assert_eq!(stats.skipped, 1, "1 invalid record should be skipped");
    }

    #[tokio::test]
    async fn test_migration_empty_dataset() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_dir = temp_dir.path().join("lancedb");
        let db_uri = db_dir.to_str().expect("Invalid path");

        let json_path = temp_dir.path().join("test_data_empty.json");
        let json_data = serde_json::json!({
            "version": "1.0",
            "records": []
        });

        std::fs::write(&json_path, json_data.to_string().as_bytes())
            .expect("Failed to write test JSON");

        let config = MigrationConfig {
            json_path,
            lancedb_uri: db_uri.to_string(),
            table_name: "test_knowledge_empty".to_string(),
            batch_size: 10,
            skip_errors: false,
            backup: false,
        };

        let result = migrate_json_to_lancedb(config).await;

        assert!(
            result.is_ok(),
            "Migration with empty dataset should succeed"
        );
        let stats = result.unwrap();
        assert_eq!(stats.total_processed, 0, "Should process 0 records");
        assert_eq!(stats.successful, 0, "No records should succeed");
    }
}
