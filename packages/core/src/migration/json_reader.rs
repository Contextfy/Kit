//! JSON data structures and reader for migration

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::MigrationError;

/// Legacy JSON data format
///
/// This represents the expected structure of JSON files created by
/// earlier versions of Contextfy/Kit.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonData {
    /// Version of the JSON format
    #[serde(default = "default_version")]
    pub version: String,

    /// Knowledge records
    pub records: Vec<JsonRecord>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// A single knowledge record from legacy JSON format
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRecord {
    /// Unique identifier (UUID or hash)
    pub id: String,

    /// Document title
    pub title: String,

    /// Brief summary for search results
    pub summary: String,

    /// Full document content
    pub content: String,

    /// Keywords/tags (optional, may be empty)
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Source file path
    pub source_path: String,

    /// Creation timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last modified timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl JsonRecord {
    /// Validate that required fields are non-empty
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.id.trim().is_empty() {
            return Err(MigrationError::MissingField {
                field: "id".to_string(),
                record_id: self.id.clone(),
            });
        }

        if self.title.trim().is_empty() {
            return Err(MigrationError::MissingField {
                field: "title".to_string(),
                record_id: self.id.clone(),
            });
        }

        if self.content.trim().is_empty() {
            return Err(MigrationError::MissingField {
                field: "content".to_string(),
                record_id: self.id.clone(),
            });
        }

        Ok(())
    }
}

/// Reader for JSON files with batch support
///
/// This reader supports:
/// - Single JSON files
/// - Directories containing multiple JSON files
/// - Batched iteration for memory-efficient processing
pub struct JsonReader {
    data: JsonData,
    batch_size: usize,
    current_position: usize,
}

impl JsonReader {
    /// Create a new JSON reader from a file path
    pub async fn from_path<P: AsRef<Path>>(
        path: P,
        batch_size: usize,
    ) -> Result<Self, MigrationError> {
        let path = path.as_ref();

        // Read the file
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| MigrationError::JsonReadError {
                    path: path.to_path_buf(),
                    source: Box::new(e),
                })?;

        // Parse JSON
        let data: JsonData = serde_json::from_str(&content)?;

        Ok(Self {
            data,
            batch_size,
            current_position: 0,
        })
    }

    /// Create a reader from pre-parsed JSON data
    pub fn from_data(data: JsonData, batch_size: usize) -> Self {
        Self {
            data,
            batch_size,
            current_position: 0,
        }
    }

    /// Get total number of records
    pub fn total_records(&self) -> usize {
        self.data.records.len()
    }

    /// Check if there are more batches to process
    pub fn has_more_batches(&self) -> bool {
        self.current_position < self.data.records.len()
    }

    /// Get the next batch of records
    pub fn next_batch(&mut self) -> Option<Vec<JsonRecord>> {
        if !self.has_more_batches() {
            return None;
        }

        let end = std::cmp::min(
            self.current_position + self.batch_size,
            self.data.records.len(),
        );

        let batch = self.data.records[self.current_position..end].to_vec();
        self.current_position = end;

        Some(batch)
    }

    /// Create an iterator over batches
    pub fn batches(mut self) -> impl Iterator<Item = Vec<JsonRecord>> {
        std::iter::from_fn(move || self.next_batch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_record_validation_valid() {
        let record = JsonRecord {
            id: "test-id".to_string(),
            title: "Test Title".to_string(),
            summary: "Test Summary".to_string(),
            content: "Test Content".to_string(),
            keywords: vec!["tag1".to_string()],
            source_path: "/path/to/file.md".to_string(),
            created_at: None,
            updated_at: None,
        };

        assert!(record.validate().is_ok());
    }

    #[test]
    fn test_json_record_validation_missing_title() {
        let record = JsonRecord {
            id: "test-id".to_string(),
            title: "   ".to_string(), // Empty after trim
            summary: "Test Summary".to_string(),
            content: "Test Content".to_string(),
            keywords: vec![],
            source_path: "/path/to/file.md".to_string(),
            created_at: None,
            updated_at: None,
        };

        let result = record.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            MigrationError::MissingField { field, .. } => {
                assert_eq!(field, "title");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_json_reader_batches() {
        let records = (0..250)
            .map(|i| JsonRecord {
                id: format!("id-{}", i),
                title: format!("Title {}", i),
                summary: format!("Summary {}", i),
                content: format!("Content {}", i),
                keywords: vec![],
                source_path: format!("/path/{}.md", i),
                created_at: None,
                updated_at: None,
            })
            .collect();

        let data = JsonData {
            version: "1.0".to_string(),
            records,
        };

        let mut reader = JsonReader::from_data(data, 100);

        assert_eq!(reader.total_records(), 250);
        assert!(reader.has_more_batches());

        // First batch: 100 records
        let batch1 = reader.next_batch().unwrap();
        assert_eq!(batch1.len(), 100);

        // Second batch: 100 records
        let batch2 = reader.next_batch().unwrap();
        assert_eq!(batch2.len(), 100);

        // Third batch: 50 records (remaining)
        let batch3 = reader.next_batch().unwrap();
        assert_eq!(batch3.len(), 50);

        // No more batches
        assert!(reader.next_batch().is_none());
        assert!(!reader.has_more_batches());
    }
}
