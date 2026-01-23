use crate::parser::ParsedDoc;
use anyhow::Result;
// use lancedb::connect;
// use arrow::array::{StringArray, StringBuilder};
// use arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
// use arrow::record_batch::RecordBatch;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

pub struct KnowledgeStore {
    data_dir: String,
}

impl KnowledgeStore {
    pub fn new(data_dir: &str) -> Result<Self> {
        fs::create_dir_all(data_dir)?;
        Ok(KnowledgeStore {
            data_dir: data_dir.to_string(),
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<KnowledgeRecord>> {
        let mut records = Vec::new();

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    if record.title.to_lowercase().contains(&query.to_lowercase())
                        || record
                            .summary
                            .to_lowercase()
                            .contains(&query.to_lowercase())
                    {
                        records.push(record);
                    }
                }
            }
        }

        Ok(records)
    }

    pub async fn get(&self, id: &str) -> Result<Option<KnowledgeRecord>> {
        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    if record.id == id {
                        return Ok(Some(record));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn add(&self, doc: &ParsedDoc) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let record = KnowledgeRecord {
            id: id.clone(),
            title: doc.title.clone(),
            summary: doc.summary.clone(),
            content: doc.content.clone(),
        };

        let json = serde_json::to_string_pretty(&record)?;
        fs::write(Path::new(&self.data_dir).join(format!("{}.json", id)), json)?;

        Ok(id)
    }
}
