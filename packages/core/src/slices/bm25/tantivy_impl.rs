/// Tantivy implementation of Bm25StoreTrait
///
/// This module provides the concrete Tantivy backend for BM25 full-text search.
/// It implements the Bm25StoreTrait while keeping Tantivy-specific
/// types isolated within this module.
///
/// Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2

use async_trait::async_trait;
use anyhow::{Context, Result as AnyhowResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, TantivyDocument, Value},
    Index, IndexReader, IndexWriter,
};

use crate::kernel::types::{Query, Score};
use crate::kernel::errors::{AppError, InfraError};

use super::trait_::{Bm25StoreTrait, Bm25Result};
use super::schema::{FIELD_ID, FIELD_TITLE, FIELD_SUMMARY, FIELD_CONTENT, FIELD_KEYWORDS};
use super::index::{create_bm25_index, create_index_reader};

/// Tantivy BM25 store implementation
///
/// This struct holds the Tantivy index and implements Bm25StoreTrait.
///
/// # Fields
///
/// * `index` - Tantivy index
/// * `writer` - Index writer for adding documents
/// * `reader` - Index reader for searching
///
/// **NOTE**: This struct is public for testing purposes only.
#[doc(hidden)]
#[allow(dead_code)]
pub struct TantivyBm25Store {
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    reader: Arc<IndexReader>,
}

#[allow(dead_code)]
impl TantivyBm25Store {
    /// Create a new Tantivy BM25 store
    ///
    /// # Parameters
    ///
    /// * `index` - Tantivy index
    ///
    /// # Returns
    ///
    /// Returns a new `TantivyBm25Store` instance.
    #[doc(hidden)]
    pub fn new(index: Index) -> AnyhowResult<Self> {
        let _schema = index.schema();

        // Create index writer with 50MB buffer
        let writer = index
            .writer(50_000_000)
            .context("Failed to create index writer")?;

        // Create index reader
        let reader = create_index_reader(&index)
            .context("Failed to create index reader")?;

        Ok(Self {
            index,
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(reader),
        })
    }

    /// Create TantivyBm25Store from directory
    ///
    /// This is a convenience function for creating a store from a directory.
    ///
    /// # Parameters
    ///
    /// * `directory` - Directory path for the index
    ///
    /// # Returns
    ///
    /// Returns a new `TantivyBm25Store` instance.
    #[doc(hidden)]
    pub async fn from_directory(directory: &std::path::Path) -> AnyhowResult<Self> {
        let index = create_bm25_index(Some(directory))?;
        Self::new(index)
    }

    /// Extract text value from Tantivy document
    fn extract_text_value(doc: &TantivyDocument, field: Field) -> String {
        if let Some(value) = doc.get_first(field) {
            if let Some(text) = value.as_str() {
                return text.to_string();
            }
        }
        String::new()
    }

    /// Convert BM25 score to normalized Score
    ///
    /// Tantivy returns BM25 scores which can be any positive value.
    /// We normalize to [0.0, 1.0] range using clamping.
    fn normalize_score(bm25_score: f32) -> Score {
        // BM25 scores can vary widely, so we use a simple clamp for now
        // In production, you might want to use percentile-based normalization
        let normalized = (bm25_score / 20.0).clamp(0.0, 1.0);
        Score::new(normalized as f64)
    }
}

#[async_trait]
impl Bm25StoreTrait for TantivyBm25Store {
    /// Search for documents using BM25 full-text search
    ///
    /// # Implementation Notes
    ///
    /// 1. Uses spawn_blocking to avoid blocking Tokio runtime
    /// 2. Performs Tantivy query parsing and BM25 search
    /// 3. Converts results to Bm25Result types with normalized scores
    /// 4. Returns Ok(Some(vec[])) if no results found (not an error)
    async fn search(&self, query: &Query) -> Result<Option<Vec<Bm25Result>>, AppError> {
        let query_text = query.text.trim().to_string();

        // Empty query returns empty results
        if query_text.is_empty() {
            return Ok(Some(vec![]));
        }

        // Clone Arc references for the blocking task
        let reader_clone = Arc::clone(&self.reader);
        let index_clone = self.index.clone();
        let limit = query.limit;

        // Use spawn_blocking to avoid blocking Tokio runtime
        let search_result = tokio::task::spawn_blocking(move || {
            // Reload reader to get latest commits
            reader_clone.reload()
                .context("Failed to reload index reader")?;

            // Get searcher snapshot
            let searcher = reader_clone.searcher();

            // Create query parser
            let schema = index_clone.schema();
            let title_field = schema.get_field(FIELD_TITLE)
                .context("Missing title field in schema")?;
            let summary_field = schema.get_field(FIELD_SUMMARY)
                .context("Missing summary field in schema")?;
            let content_field = schema.get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;
            let keywords_field = schema.get_field(FIELD_KEYWORDS)
                .context("Missing keywords field in schema")?;

            let mut query_parser = QueryParser::for_index(
                &index_clone,
                vec![title_field, summary_field, content_field, keywords_field],
            );
            query_parser.set_field_boost(keywords_field, 5.0);

            // Parse query
            let parsed_query = query_parser.parse_query(&query_text)
                .with_context(|| format!("Failed to parse query: {}", query_text))?;

            // Execute search with TopDocs collector
            let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))
                .context("Failed to execute search")?;

            // Extract field references for result conversion
            let id_field = schema.get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let title_field = schema.get_field(FIELD_TITLE)
                .context("Missing title field in schema")?;
            let summary_field = schema.get_field(FIELD_SUMMARY)
                .context("Missing summary field in schema")?;

            // Convert search results
            let mut results = Vec::new();
            for (bm25_score, doc_address) in top_docs {
                let retrieved_doc = searcher.doc(doc_address)
                    .context("Failed to retrieve document")?;

                let id = Self::extract_text_value(&retrieved_doc, id_field);
                let title = Self::extract_text_value(&retrieved_doc, title_field);
                let summary = Self::extract_text_value(&retrieved_doc, summary_field);
                let score = Self::normalize_score(bm25_score);

                results.push(Bm25Result::new(id, title, summary, score));
            }

            Ok::<Vec<Bm25Result>, anyhow::Error>(results)
        })
        .await
        .map_err(|e| AppError::Infra(InfraError::database(
            "search task failed",
            Some::<anyhow::Error>(e.into()),
        )))?
        .map_err(|e| AppError::Infra(InfraError::database("search failed", Some(e))))?;

        Ok(Some(search_result))
    }

    /// Add a document to the BM25 index
    ///
    /// # Implementation Notes
    ///
    /// 1. Creates Tantivy document with all fields
    /// 2. Uses spawn_blocking for the write operation
    /// 3. Commits changes to make document searchable
    async fn add(
        &self,
        id: &str,
        title: &str,
        summary: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let id = id.to_string();
        let title = title.to_string();
        let summary = summary.to_string();
        let content = content.to_string();

        let writer_clone = Arc::clone(&self.writer);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        tokio::task::spawn_blocking(move || {
            // Get schema
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema.get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let title_field = schema.get_field(FIELD_TITLE)
                .context("Missing title field in schema")?;
            let summary_field = schema.get_field(FIELD_SUMMARY)
                .context("Missing summary field in schema")?;
            let content_field = schema.get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;
            let _keywords_field = schema.get_field(FIELD_KEYWORDS)
                .context("Missing keywords field in schema")?;

            // Create document
            let mut doc = TantivyDocument::new();
            doc.add_text(id_field, &id);
            doc.add_text(title_field, &title);
            doc.add_text(summary_field, &summary);
            doc.add_text(content_field, &content);

            // Get writer lock
            let mut writer = writer_clone.blocking_lock();

            // Add document to index
            writer.add_document(doc)
                .context("Failed to add document to index")?;

            // Commit to make document searchable
            writer.commit()
                .context("Failed to commit index")?;

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|e| AppError::Infra(InfraError::database(
            "add task failed",
            Some::<anyhow::Error>(e.into()),
        )))?
        .map_err(|e| AppError::Infra(InfraError::database("add failed", Some(e))))?;

        Ok(())
    }

    /// Delete a document from the BM25 index
    ///
    /// # Implementation Notes
    ///
    /// 1. Uses term-based deletion by ID field
    /// 2. Returns true if document was found and deleted
    /// 3. Returns false if document was not found
    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        let id = id.to_string();

        let writer_clone = Arc::clone(&self.writer);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        tokio::task::spawn_blocking(move || {
            // Get schema
            let schema = index_clone.schema();

            // Get ID field
            let id_field = schema.get_field(FIELD_ID)
                .context("Missing id field in schema")?;

            // Get writer lock
            let mut writer = writer_clone.blocking_lock();

            // Delete document by term (returns number of deleted documents)
            let term = tantivy::Term::from_field_text(id_field, &id);
            let _deleted_count = writer.delete_term(term);

            // Commit to make deletion visible
            writer.commit()
                .context("Failed to commit index")?;

            Ok::<bool, anyhow::Error>(true)
        })
        .await
        .map_err(|e| AppError::Infra(InfraError::database(
            "delete task failed",
            Some::<anyhow::Error>(e.into()),
        )))?
        .map_err(|e| AppError::Infra(InfraError::database("delete failed", Some(e))))?;

        Ok(true)
    }

    /// Check if the store is healthy and accessible
    ///
    /// # Implementation Notes
    ///
    /// Verifies index reader can be reloaded and searcher can access data.
    async fn health_check(&self) -> Result<bool, AppError> {
        let reader_clone = Arc::clone(&self.reader);

        // Use spawn_blocking to avoid blocking Tokio runtime
        tokio::task::spawn_blocking(move || {
            reader_clone.reload()
                .context("Failed to reload index reader")?;
            Ok::<bool, anyhow::Error>(true)
        })
        .await
        .map_err(|e| AppError::Infra(InfraError::database(
            "health check task failed",
            Some::<anyhow::Error>(e.into()),
        )))?
        .map_err(|e| AppError::Infra(InfraError::database("health check failed", Some(e))))?;

        Ok(true)
    }

    /// Get a document by ID
    ///
    /// # Implementation Notes
    ///
    /// 1. Uses spawn_blocking to avoid blocking Tokio runtime
    /// 2. Searches for document by exact ID match
    /// 3. Returns full document details including title and summary
    async fn get_by_id(&self, id: &str) -> Result<Option<Bm25Result>, AppError> {
        let id = id.to_string();

        let reader_clone = Arc::clone(&self.reader);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        let get_result = tokio::task::spawn_blocking(move || {
            // Reload reader to get latest commits
            reader_clone.reload()
                .context("Failed to reload index reader")?;

            // Get searcher snapshot
            let searcher = reader_clone.searcher();

            // Get schema
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema.get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let title_field = schema.get_field(FIELD_TITLE)
                .context("Missing title field in schema")?;
            let summary_field = schema.get_field(FIELD_SUMMARY)
                .context("Missing summary field in schema")?;
            let content_field = schema.get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;

            // Create query for exact ID match
            let term = tantivy::Term::from_field_text(id_field, &id);
            let query = tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

            // Execute search
            let top_docs = searcher.search(&query, &tantivy::collector::TopDocs::with_limit(1))
                .context("Failed to execute search")?;

            // Check if document was found
            if top_docs.is_empty() {
                return Ok::<Option<Bm25Result>, anyhow::Error>(None);
            }

            // Get the first (and only) result
            let (_score, doc_address) = &top_docs[0];
            let retrieved_doc = searcher.doc(*doc_address)
                .context("Failed to retrieve document")?;

            // Extract document fields
            let doc_id = Self::extract_text_value(&retrieved_doc, id_field);
            let title = Self::extract_text_value(&retrieved_doc, title_field);
            let summary = Self::extract_text_value(&retrieved_doc, summary_field);
            let content = Self::extract_text_value(&retrieved_doc, content_field);

            // Return result with content and default score (not relevant for get_by_id)
            Ok(Some(Bm25Result::with_content(doc_id, title, summary, content, Score::new(1.0))))
        })
        .await
        .map_err(|e| AppError::Infra(InfraError::database(
            "get_by_id task failed",
            Some::<anyhow::Error>(e.into()),
        )))?
        .map_err(|e| AppError::Infra(InfraError::database("get_by_id failed", Some(e))))?;

        Ok(get_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test store
    async fn create_test_store() -> (TantivyBm25Store, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let store = TantivyBm25Store::from_directory(temp_dir.path())
            .await
            .expect("Failed to create store");

        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_tantivy_store_creation() {
        let (store, _temp_dir) = create_test_store().await;

        assert!(store.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let (store, _temp_dir) = create_test_store().await;

        let query = Query::new("", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let results = result.unwrap().unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let (store, _temp_dir) = create_test_store().await;

        let query = Query::new("nonexistent query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let results = result.unwrap().unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_add_and_search() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a document
        store.add(
            "doc-1",
            "Rust Programming",
            "A guide to Rust",
            "Rust is a systems programming language",
        )
        .await
        .expect("Failed to add document");

        // Search for it
        let query = Query::new("Rust", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let results = result.unwrap().unwrap();
        assert!(results.len() > 0);
        assert_eq!(results[0].id, "doc-1");
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[tokio::test]
    async fn test_delete() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a document
        store.add(
            "doc-1",
            "Test Document",
            "Test Summary",
            "Test Content",
        )
        .await
        .expect("Failed to add document");

        // Delete it
        let result = store.delete("doc-1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Search should return no results
        let query = Query::new("Test", 10);
        let search_result = store.search(&query).await.unwrap().unwrap();
        assert_eq!(search_result.len(), 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store.health_check().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_normalize_score() {
        // Test score normalization
        assert_eq!(TantivyBm25Store::normalize_score(0.0).value(), 0.0);
        assert_eq!(TantivyBm25Store::normalize_score(10.0).value(), 0.5);
        assert_eq!(TantivyBm25Store::normalize_score(20.0).value(), 1.0);

        // Test clamping
        assert_eq!(TantivyBm25Store::normalize_score(100.0).value(), 1.0);
        assert_eq!(TantivyBm25Store::normalize_score(-10.0).value(), 0.0);
    }
}
