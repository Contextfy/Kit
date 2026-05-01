//! Migration error types

use std::path::PathBuf;

use crate::AppError;

/// Migration-specific errors
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// JSON file could not be read or parsed
    #[error("Failed to read JSON file '{path}': {source}")]
    JsonReadError {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// JSON structure is invalid or missing required fields
    #[error("Invalid JSON structure: {0}")]
    InvalidJsonStructure(String),

    /// Required field is missing or empty
    #[error("Missing or empty required field '{field}' in record {record_id}")]
    MissingField { field: String, record_id: String },

    /// Embedding generation failed
    #[error("Failed to generate embedding for record '{record_id}': {reason}")]
    EmbeddingFailed { record_id: String, reason: String },

    /// LanceDB operation failed
    #[error("LanceDB error: {0}")]
    LanceDbError(#[from] lancedb::Error),

    /// LanceDB connection failed
    #[error("Failed to connect to LanceDB at '{uri}': {reason}")]
    ConnectionError { uri: String, reason: String },

    /// Backup creation failed
    #[error("Failed to create backup of '{path}': {reason}")]
    BackupError { path: PathBuf, reason: String },

    /// Validation failed after migration
    #[error("Migration validation failed: {0}")]
    ValidationError(String),

    /// Batch processing encountered an error
    #[error("Batch processing failed at batch #{batch_number}: {reason}")]
    BatchError { batch_number: usize, reason: String },

    /// Configuration error
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<AppError> for MigrationError {
    fn from(err: AppError) -> Self {
        MigrationError::ConfigError(err.to_string())
    }
}

impl From<serde_json::Error> for MigrationError {
    fn from(err: serde_json::Error) -> Self {
        MigrationError::InvalidJsonStructure(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MigrationError::MissingField {
            field: "title".to_string(),
            record_id: "test-id".to_string(),
        };
        assert!(err.to_string().contains("title"));
        assert!(err.to_string().contains("test-id"));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let migration_err: MigrationError = io_err.into();
        assert!(matches!(migration_err, MigrationError::IoError(_)));
    }
}
