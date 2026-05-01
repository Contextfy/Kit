//! LanceDB connection management
//!
//! This module handles connection lifecycle to LanceDB, including:
//! - Connection establishment
//! - Table creation and validation
//! - Schema migration support
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

use anyhow::{Context, Result};
use lancedb::connection::Connection as LanceConnection;
use std::sync::Arc;

use super::schema::{knowledge_record_schema, validate_knowledge_schema};

/// Connect to a LanceDB database
///
/// # Parameters
///
/// * `uri` - Database connection string (e.g., "lance://data/db" or local directory path)
///
/// # Returns
///
/// Returns a `Connection` object on success.
///
/// # Errors
///
/// Returns an error if:
/// - URI format is invalid
/// - Database is not accessible
/// - Connection times out
#[allow(dead_code)]
pub(crate) async fn connect(uri: &str) -> Result<LanceConnection> {
    lancedb::connect(uri)
        .execute()
        .await
        .context("Failed to connect to LanceDB")
}

/// Create a table if it doesn't exist
///
/// This function is idempotent - calling it multiple times with the same
/// table_name will succeed after the first creation.
///
/// # Parameters
///
/// * `conn` - LanceDB connection object
/// * `table_name` - Name of the table to create
///
/// # Returns
///
/// Returns `Ok(())` if:
/// - Table was created successfully
/// - Table already exists with matching schema
///
/// # Errors
///
/// Returns an error if:
/// - Failed to query existing tables
/// - Table exists but schema doesn't match
/// - Failed to create new table
#[allow(dead_code)]
pub(crate) async fn create_table_if_not_exists(
    conn: &LanceConnection,
    table_name: &str,
) -> Result<()> {
    // Get existing table names
    let existing_tables = conn
        .table_names()
        .execute()
        .await
        .context("Failed to get table names from database")?;

    // Check if table already exists
    if existing_tables.contains(&table_name.to_string()) {
        // Table exists - verify schema matches
        let existing_table = conn
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open existing table: {}", table_name))?;

        let existing_schema = existing_table
            .schema()
            .await
            .with_context(|| format!("Failed to get schema for existing table: {}", table_name))?;

        // Validate schema matches expected structure
        validate_knowledge_schema(&existing_schema).map_err(|e| {
            anyhow::anyhow!("Table '{}' exists but schema mismatch: {}", table_name, e)
        })?;

        return Ok(());
    }

    // Table doesn't exist - create new table
    let schema = Arc::new(knowledge_record_schema());
    let new_table = conn
        .create_empty_table(table_name, schema)
        .execute()
        .await
        .with_context(|| format!("Failed to create table: {}", table_name))?;

    // Verify table was created with correct schema
    let created_schema = new_table.schema().await.with_context(|| {
        format!(
            "Failed to get schema for newly created table: {}",
            table_name
        )
    })?;

    validate_knowledge_schema(&created_schema)
        .map_err(|e| anyhow::anyhow!("Created table '{}' has invalid schema: {}", table_name, e))?;

    Ok(())
}

/// Initialize a LanceDB database
///
/// This is a convenience function that combines connection and table creation,
/// returning the initialized connection for immediate use.
///
/// # Parameters
///
/// * `uri` - Database connection string
/// * `table_name` - Name of the table to ensure exists
///
/// # Returns
///
/// Returns `LanceConnection` when database is initialized successfully.
///
/// # Errors
///
/// Returns an error if:
/// - Connection fails
/// - Table creation or validation fails
///
/// # Example
///
/// ```text
/// // Note: This function is internal-only (#[doc(hidden)]).
/// // External code should use the public VectorStoreTrait instead.
/// // Example flow:
/// let conn = initialize("data/db", "knowledge").await?;
/// // Use conn directly without reconnecting
/// ```
#[allow(dead_code)]
pub(crate) async fn initialize(uri: &str, table_name: &str) -> Result<LanceConnection> {
    let conn = connect(uri).await?;
    create_table_if_not_exists(&conn, table_name).await?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test connection to temporary database
    #[tokio::test]
    async fn test_connect_temp_dir() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");

        let conn = connect(db_uri).await.expect("Failed to connect to LanceDB");

        // Verify connection succeeded
        assert!(!conn.database().to_string().is_empty());
    }

    /// Test table creation
    #[tokio::test]
    async fn test_create_table_if_not_exists() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_knowledge";

        let conn = connect(db_uri).await.expect("Failed to connect to LanceDB");

        // Create table
        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to create table");

        // Verify table exists
        let table_names = conn
            .table_names()
            .execute()
            .await
            .expect("Failed to get table names");
        assert!(
            table_names.contains(&table_name.to_string()),
            "Table '{}' should exist",
            table_name
        );
    }

    /// Test table creation is idempotent
    #[tokio::test]
    async fn test_create_table_idempotent() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_idempotent";

        let conn = connect(db_uri).await.expect("Failed to connect to LanceDB");

        // First creation
        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to create table (first time)");

        // Second creation (should not fail)
        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to validate existing table (second time)");

        // Verify table still exists
        let table_names = conn
            .table_names()
            .execute()
            .await
            .expect("Failed to get table names");
        assert!(table_names.contains(&table_name.to_string()));
    }

    /// Test initialize convenience function
    #[tokio::test]
    async fn test_initialize() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_init";

        // Initialize database and get connection
        let conn = initialize(db_uri, table_name)
            .await
            .expect("Failed to initialize database");

        // Verify table exists using the returned connection
        let table_names = conn
            .table_names()
            .execute()
            .await
            .expect("Failed to get table names");
        assert!(table_names.contains(&table_name.to_string()));
    }

    /// Test schema validation on created table
    #[tokio::test]
    async fn test_table_schema_validation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_schema";

        let conn = connect(db_uri).await.expect("Failed to connect");
        create_table_if_not_exists(&conn, table_name)
            .await
            .expect("Failed to create table");

        // Open table and verify schema
        let table = conn
            .open_table(table_name)
            .execute()
            .await
            .expect("Failed to open table");

        let schema = table.schema().await.expect("Failed to get schema");

        // Verify field count
        assert_eq!(schema.fields().len(), 7);

        // Verify vector field
        let vector_field = schema
            .field_with_name("vector")
            .expect("Schema should have 'vector' field");

        use arrow::datatypes::DataType;
        match vector_field.data_type() {
            DataType::FixedSizeList(field, size) => {
                assert_eq!(*size, super::super::schema::VECTOR_DIM);
                assert_eq!(field.data_type(), &DataType::Float32);
            }
            _ => panic!("Vector field should be FixedSizeList"),
        }
    }
}
