use crate::KnowledgeStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Details {
    pub id: String,
    pub title: String,
    pub content: String,
}

pub struct Retriever<'a> {
    store: &'a KnowledgeStore,
}

impl<'a> Retriever<'a> {
    pub fn new(store: &'a KnowledgeStore) -> Self {
        Retriever { store }
    }

    pub async fn scout(&self, query: &str) -> Result<Vec<Brief>> {
        let records = self.store.search(query).await?;
        Ok(records
            .into_iter()
            .map(|r| Brief {
                id: r.id,
                title: r.title,
                summary: r.summary,
            })
            .collect())
    }

    pub async fn inspect(&self, id: &str) -> Result<Option<Details>> {
        let record = self.store.get(id).await?;

        match record {
            Some(r) => Ok(Some(Details {
                id: r.id,
                title: r.title,
                content: r.content,
            })),
            None => Ok(None),
        }
    }
}
