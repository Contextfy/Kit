use anyhow::{Context, Result as AnyhowResult};
/// Tantivy implementation of Bm25StoreTrait
///
/// This module provides the concrete Tantivy backend for BM25 full-text search.
/// It implements the Bm25StoreTrait while keeping Tantivy-specific
/// types isolated within this module.
///
/// Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2
use async_trait::async_trait;
use std::sync::Arc;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, TantivyDocument, Value},
    Index, IndexReader, IndexWriter,
};
use tokio::sync::Mutex;

use crate::kernel::errors::{AppError, InfraError};
use crate::kernel::types::{AstChunk, Query, Score};

use super::index::{create_bm25_index, create_index_reader};
use super::schema::{FIELD_CONTENT, FIELD_DEPENDENCIES, FIELD_FILE_PATH, FIELD_ID, FIELD_NODE_TYPE, FIELD_SYMBOL_NAME};
use super::trait_::{Bm25Result, Bm25StoreTrait};

// TODO(BM25-Tuning): The hardcoded BM25_MAX_SCORE of 20.0 can compress/clip real BM25 scores
// on different corpora. Consider making this a configurable parameter via env vars,
// or implementing a percentile-based normalization (e.g., 95th/99th percentile of sample queries).
/// Maximum BM25 score for normalization
///
/// Tantivy returns BM25 scores which can be any positive value.
/// We normalize to [0.0, 1.0] range by dividing by this constant.
const BM25_MAX_SCORE: f32 = 20.0;

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
        let reader = create_index_reader(&index).context("Failed to create index reader")?;

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
    pub fn from_directory(directory: &std::path::Path) -> AnyhowResult<Self> {
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
        let normalized = (bm25_score / BM25_MAX_SCORE).clamp(0.0, 1.0);
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
    /// 4. Returns Ok(None) if no results found (not an error)
    async fn search(&self, query: &Query) -> Result<Option<Vec<Bm25Result>>, AppError> {
        let query_text = query.text.trim().to_string();

        // Empty query returns None (no results, not an error)
        if query_text.is_empty() {
            return Ok(None);
        }

        // Clone Arc references for the blocking task
        let reader_clone = Arc::clone(&self.reader);
        let index_clone = self.index.clone();
        let limit = query.limit;

        // Use spawn_blocking to avoid blocking Tokio runtime
        let search_result = tokio::task::spawn_blocking(move || {
            // Reload reader to get latest commits
            reader_clone
                .reload()
                .context("Failed to reload index reader")?;

            // Get searcher snapshot
            let searcher = reader_clone.searcher();

            // Create query parser
            let schema = index_clone.schema();
            let symbol_name_field = schema
                .get_field(FIELD_SYMBOL_NAME)
                .context("Missing symbol_name field in schema")?;
            let content_field = schema
                .get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;
            let dependencies_field = schema
                .get_field(FIELD_DEPENDENCIES)
                .context("Missing dependencies field in schema")?;
            let file_path_field = schema
                .get_field(FIELD_FILE_PATH)
                .context("Missing file_path field in schema")?;
            let node_type_field = schema
                .get_field(FIELD_NODE_TYPE)
                .context("Missing node_type field in schema")?;

            let mut query_parser = QueryParser::for_index(
                &index_clone,
                vec![
                    symbol_name_field,
                    content_field,
                    dependencies_field,
                    file_path_field,
                    node_type_field,
                ],
            );

            // **Field Weights**: symbol_name^5.0, dependencies^2.0, content^1.0
            query_parser.set_field_boost(symbol_name_field, 5.0);
            query_parser.set_field_boost(dependencies_field, 2.0);
            query_parser.set_field_boost(content_field, 1.0);

            // Parse query
            let parsed_query = query_parser
                .parse_query(&query_text)
                .with_context(|| format!("Failed to parse query: {}", query_text))?;

            // Execute search with TopDocs collector
            let top_docs = searcher
                .search(&parsed_query, &TopDocs::with_limit(limit))
                .context("Failed to execute search")?;

            // Extract field references for result conversion
            let id_field = schema
                .get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let symbol_name_field = schema
                .get_field(FIELD_SYMBOL_NAME)
                .context("Missing symbol_name field in schema")?;
            let file_path_field = schema
                .get_field(FIELD_FILE_PATH)
                .context("Missing file_path field in schema")?;

            // Convert search results
            let mut results = Vec::new();
            for (bm25_score, doc_address) in top_docs {
                let retrieved_doc = searcher
                    .doc(doc_address)
                    .context("Failed to retrieve document")?;

                let id = Self::extract_text_value(&retrieved_doc, id_field);
                let title = Self::extract_text_value(&retrieved_doc, symbol_name_field);
                let summary = Self::extract_text_value(&retrieved_doc, file_path_field);
                let score = Self::normalize_score(bm25_score);

                results.push(Bm25Result::new(id, title, summary, score));
            }

            Ok::<Vec<Bm25Result>, anyhow::Error>(results)
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "search task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("search failed", Some(e))))?;

        // Return None if no results found, Some(results) otherwise
        if search_result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(search_result))
        }
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
        keywords: &str,
    ) -> Result<(), AppError> {
        let id = id.to_string();
        let title = title.to_string();
        let summary = summary.to_string();
        let content = content.to_string();
        let keywords = keywords.to_string();

        let writer_clone = Arc::clone(&self.writer);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        tokio::task::spawn_blocking(move || {
            // Get schema
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema
                .get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let symbol_name_field = schema
                .get_field(FIELD_SYMBOL_NAME)
                .context("Missing symbol_name field in schema")?;
            let file_path_field = schema
                .get_field(FIELD_FILE_PATH)
                .context("Missing file_path field in schema")?;
            let node_type_field = schema
                .get_field(FIELD_NODE_TYPE)
                .context("Missing node_type field in schema")?;
            let content_field = schema
                .get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;
            let dependencies_field = schema
                .get_field(FIELD_DEPENDENCIES)
                .context("Missing dependencies field in schema")?;

            // Create document (mapping old API to new schema)
            let mut doc = TantivyDocument::new();
            doc.add_text(id_field, &id);
            doc.add_text(symbol_name_field, &title);  // title → symbol_name
            doc.add_text(file_path_field, &summary);  // summary → file_path
            doc.add_text(node_type_field, "file");     // Default node_type
            doc.add_text(content_field, &content);

            // keywords → dependencies (split by whitespace)
            for keyword in keywords.split_whitespace() {
                doc.add_text(dependencies_field, keyword);
            }

            // Get writer lock
            let mut writer = writer_clone.blocking_lock();

            // Upsert: Delete existing document with same ID first (if exists)
            // This prevents duplicate documents when updating an existing entry
            let term = tantivy::Term::from_field_text(id_field, &id);
            writer.delete_term(term);

            // Add new document to index
            writer
                .add_document(doc)
                .context("Failed to add document to index")?;

            // Commit to make document searchable
            writer.commit().context("Failed to commit index")?;

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "add task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("add failed", Some(e))))?;

        Ok(())
    }

    /// Delete a document from the BM25 index
    ///
    /// # Implementation Notes
    ///
    /// 1. Pre-checks if document exists before attempting deletion
    /// 2. Uses term-based deletion by ID field
    /// 3. Returns true if document was found and deleted
    /// 4. Returns false if document was not found (idempotent)
    async fn delete(&self, id: &str) -> Result<bool, AppError> {
        let id = id.to_string();

        let writer_clone = Arc::clone(&self.writer);
        let reader_clone = Arc::clone(&self.reader);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        tokio::task::spawn_blocking(move || {
            // Get schema
            let schema = index_clone.schema();

            // Get ID field
            let id_field = schema
                .get_field(FIELD_ID)
                .context("Missing id field in schema")?;

            // CRITICAL: Pre-check if document exists before deletion
            // writer.delete_term() returns Opstamp, not deletion count
            // We must search first to determine if document exists
            reader_clone
                .reload()
                .context("Failed to reload index reader")?;

            let searcher = reader_clone.searcher();
            let term = tantivy::Term::from_field_text(id_field, &id);
            let term_query = tantivy::query::TermQuery::new(
                term.clone(),
                tantivy::schema::IndexRecordOption::Basic,
            );

            // Search for the document (limit to 1 result)
            let top_docs = searcher
                .search(&term_query, &tantivy::collector::TopDocs::with_limit(1))
                .context("Failed to search for document")?;

            // If document doesn't exist, return false (idempotent)
            if top_docs.is_empty() {
                return Ok::<bool, anyhow::Error>(false);
            }

            // Document exists, proceed with deletion
            let mut writer = writer_clone.blocking_lock();

            // Delete document by term
            writer.delete_term(term);

            // Commit to make deletion visible
            writer.commit().context("Failed to commit index")?;

            // Document was found and deleted
            Ok::<bool, anyhow::Error>(true)
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "delete task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("delete failed", Some(e))))
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
            reader_clone
                .reload()
                .context("Failed to reload index reader")?;
            Ok::<bool, anyhow::Error>(true)
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "health check task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
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
            reader_clone
                .reload()
                .context("Failed to reload index reader")?;

            // Get searcher snapshot
            let searcher = reader_clone.searcher();

            // Get schema
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema
                .get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let symbol_name_field = schema
                .get_field(FIELD_SYMBOL_NAME)
                .context("Missing symbol_name field in schema")?;
            let file_path_field = schema
                .get_field(FIELD_FILE_PATH)
                .context("Missing file_path field in schema")?;
            let content_field = schema
                .get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;

            // Create query for exact ID match
            let term = tantivy::Term::from_field_text(id_field, &id);
            let query =
                tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

            // Execute search
            let top_docs = searcher
                .search(&query, &tantivy::collector::TopDocs::with_limit(1))
                .context("Failed to execute search")?;

            // Check if document was found
            if top_docs.is_empty() {
                return Ok::<Option<Bm25Result>, anyhow::Error>(None);
            }

            // Get the first (and only) result
            let (_score, doc_address) = &top_docs[0];
            let retrieved_doc = searcher
                .doc(*doc_address)
                .context("Failed to retrieve document")?;

            // Extract document fields
            let doc_id = Self::extract_text_value(&retrieved_doc, id_field);
            let title = Self::extract_text_value(&retrieved_doc, symbol_name_field);
            let summary = Self::extract_text_value(&retrieved_doc, file_path_field);
            let content = Self::extract_text_value(&retrieved_doc, content_field);

            // Return result with content and default score (not relevant for get_by_id)
            Ok(Some(Bm25Result::with_content(
                doc_id,
                title,
                summary,
                content,
                Score::new(1.0),
            )))
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "get_by_id task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("get_by_id failed", Some(e))))?;

        Ok(get_result)
    }

    /// Get multiple documents by IDs (batch version)
    ///
    /// More efficient than calling get_by_id multiple times as it batches queries.
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Option<Bm25Result>>, AppError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids = ids.to_vec();
        let reader_clone = Arc::clone(&self.reader);
        let index_clone = self.index.clone();

        // Use spawn_blocking to avoid blocking Tokio runtime
        let get_results = tokio::task::spawn_blocking(move || {
            // Reload reader to get latest commits
            reader_clone
                .reload()
                .context("Failed to reload index reader")?;

            // Get searcher snapshot
            let searcher = reader_clone.searcher();

            // Get schema
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema
                .get_field(FIELD_ID)
                .context("Missing id field in schema")?;
            let symbol_name_field = schema
                .get_field(FIELD_SYMBOL_NAME)
                .context("Missing symbol_name field in schema")?;
            let file_path_field = schema
                .get_field(FIELD_FILE_PATH)
                .context("Missing file_path field in schema")?;
            let content_field = schema
                .get_field(FIELD_CONTENT)
                .context("Missing content field in schema")?;

            let mut results = Vec::with_capacity(ids.len());

            // Batch query: use BooleanQuery to match multiple IDs
            let mut terms: Vec<Box<dyn tantivy::query::Query>> = Vec::new();
            for id in &ids {
                let term = tantivy::Term::from_field_text(id_field, id);
                terms.push(Box::new(tantivy::query::TermQuery::new(
                    term,
                    tantivy::schema::IndexRecordOption::Basic,
                )));
            }

            // Use OR query to match any of the IDs
            let boolean_query = tantivy::query::BooleanQuery::union(terms);
            let top_docs = searcher
                .search(
                    &boolean_query,
                    &tantivy::collector::TopDocs::with_limit(ids.len()),
                )
                .context("Failed to execute batch search")?;

            // Build a map of ID -> document for quick lookup
            let mut doc_map: std::collections::HashMap<String, Bm25Result> =
                std::collections::HashMap::new();

            for (_score, doc_address) in top_docs {
                let retrieved_doc = searcher
                    .doc(doc_address)
                    .context("Failed to retrieve document")?;

                let doc_id = Self::extract_text_value(&retrieved_doc, id_field);
                let title = Self::extract_text_value(&retrieved_doc, symbol_name_field);
                let summary = Self::extract_text_value(&retrieved_doc, file_path_field);
                let content = Self::extract_text_value(&retrieved_doc, content_field);

                doc_map.insert(
                    doc_id.clone(),
                    Bm25Result::with_content(doc_id, title, summary, content, Score::new(1.0)),
                );
            }

            // Return results in the same order as input IDs
            for id in &ids {
                results.push(doc_map.get(id).cloned());
            }

            Ok::<Vec<Option<Bm25Result>>, anyhow::Error>(results)
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "get_by_ids task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("get_by_ids failed", Some(e))))?;

        Ok(get_results)
    }

    /// Batch add AST chunks to the BM25 index
    ///
    /// # Performance Requirements
    ///
    /// - **Defense Line**: Single transaction - All chunks in ONE `writer.commit()` call
    /// - **NEVER** call `writer.commit()` in a loop
    /// - **Dependencies**: Add each dependency as a separate text field value
    ///
    /// # Parameters
    ///
    /// * `chunks` - AST chunk list
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Batch add successful
    /// * `Err(AppError)` - Batch add failed
    async fn add_batch(&self, chunks: Vec<AstChunk>) -> Result<(), AppError> {
        if chunks.is_empty() {
            return Ok(());
        }

        let writer_clone = Arc::clone(&self.writer);
        let index_clone = self.index.clone();

        tokio::task::spawn_blocking(move || {
            let schema = index_clone.schema();

            // Get field references
            let id_field = schema.get_field(FIELD_ID).context("Missing id field")?;
            let file_path_field = schema.get_field(FIELD_FILE_PATH).context("Missing file_path field")?;
            let symbol_name_field = schema.get_field(FIELD_SYMBOL_NAME).context("Missing symbol_name field")?;
            let node_type_field = schema.get_field(FIELD_NODE_TYPE).context("Missing node_type field")?;
            let content_field = schema.get_field(FIELD_CONTENT).context("Missing content field")?;
            let dependencies_field = schema.get_field(FIELD_DEPENDENCIES).context("Missing dependencies field")?;

            let mut writer = writer_clone.blocking_lock();

            // **Defense Line**: Single transaction batch add
            for chunk in chunks {
                // Upsert: Delete existing document with same ID first
                let term = tantivy::Term::from_field_text(id_field, &chunk.id);
                writer.delete_term(term);

                // Create document
                let mut doc = TantivyDocument::new();
                doc.add_text(id_field, &chunk.id);
                doc.add_text(file_path_field, &chunk.file_path);
                doc.add_text(symbol_name_field, &chunk.symbol_name);
                doc.add_text(node_type_field, &chunk.node_type);
                doc.add_text(content_field, &chunk.content);

                // Dependencies: Multi-value field - add each dependency separately
                for dep in &chunk.dependencies {
                    doc.add_text(dependencies_field, dep);
                }

                writer.add_document(doc)
                    .context("Failed to add document to batch")?;
            }

            // **Defense Line**: Single commit - NEVER in a loop
            writer.commit().context("Failed to commit batch")?;

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|e| {
            AppError::Infra(InfraError::database(
                "add_batch task failed",
                Some::<anyhow::Error>(e.into()),
            ))
        })?
        .map_err(|e| AppError::Infra(InfraError::database("add_batch failed", Some(e))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test store
    async fn create_test_store() -> (TantivyBm25Store, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let store =
            TantivyBm25Store::from_directory(temp_dir.path()).expect("Failed to create store");

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
        // Empty query should return None (not Some(vec![]))
        let results = result.unwrap();
        assert!(results.is_none(), "Empty query should return None");
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let (store, _temp_dir) = create_test_store().await;

        let query = Query::new("nonexistent query", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        // No results should return None (not Some(vec![]))
        let results = result.unwrap();
        assert!(
            results.is_none(),
            "Search with no matches should return None"
        );
    }

    #[tokio::test]
    async fn test_add_and_search() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a document
        store
            .add(
                "doc-1",
                "Rust Programming",
                "A guide to Rust",
                "Rust is a systems programming language",
                "",
            )
            .await
            .expect("Failed to add document");

        // Search for it
        let query = Query::new("Rust", 10);
        let result = store.search(&query).await;

        assert!(result.is_ok());
        let results = result.unwrap().unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-1");
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[tokio::test]
    async fn test_delete() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a document
        store
            .add("doc-1", "Test Document", "Test Summary", "Test Content", "")
            .await
            .expect("Failed to add document");

        // Delete it
        let result = store.delete("doc-1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Search should return None (no results)
        let query = Query::new("Test", 10);
        let search_result = store.search(&query).await.unwrap();
        assert!(
            search_result.is_none(),
            "Search should return None after deletion"
        );
    }

    #[tokio::test]
    async fn test_delete_not_found_returns_false() {
        let (store, _temp_dir) = create_test_store().await;

        // Try to delete a document that doesn't exist
        let result = store.delete("missing-id").await;
        assert!(
            result.is_ok(),
            "delete should not error for non-existent document"
        );

        let deleted = result.unwrap();
        assert!(
            !deleted,
            "delete should return false when document does not exist"
        );
    }

    #[tokio::test]
    async fn test_health_check() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store.health_check().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_get_by_id_success() {
        let (store, _temp_dir) = create_test_store().await;

        // Add a document
        store
            .add(
                "doc-1",
                "Rust Programming",
                "A comprehensive guide to Rust",
                "Rust is a systems programming language focused on safety and performance",
                "",
            )
            .await
            .expect("Failed to add document");

        // Get the document by ID
        let result = store.get_by_id("doc-1").await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());

        let doc = result.unwrap();
        assert_eq!(doc.id, "doc-1");
        assert_eq!(doc.title, "Rust Programming");
        assert_eq!(doc.summary, "A comprehensive guide to Rust");
        assert_eq!(
            doc.content,
            Some(
                "Rust is a systems programming language focused on safety and performance"
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (store, _temp_dir) = create_test_store().await;

        // Try to get a non-existent document
        let result = store.get_by_id("non-existent-id").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
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

    #[tokio::test]
    async fn test_get_by_ids_order_preserved() {
        let (store, _temp_dir) = create_test_store().await;

        // Add 3 documents
        store
            .add("doc-1", "Title 1", "Summary 1", "Content 1", "")
            .await
            .expect("Failed to add doc-1");
        store
            .add("doc-2", "Title 2", "Summary 2", "Content 2", "")
            .await
            .expect("Failed to add doc-2");
        store
            .add("doc-3", "Title 3", "Summary 3", "Content 3", "")
            .await
            .expect("Failed to add doc-3");

        // Request documents in non-sequential order
        let results = store
            .get_by_ids(&["doc-3".to_string(), "doc-1".to_string()])
            .await
            .expect("Failed to get documents by IDs");

        // Verify order is preserved (matches request order, not sorted)
        assert_eq!(results.len(), 2, "Should return 2 results");
        assert_eq!(
            results[0].as_ref().unwrap().id,
            "doc-3",
            "First result should be doc-3"
        );
        assert_eq!(
            results[1].as_ref().unwrap().id,
            "doc-1",
            "Second result should be doc-1"
        );
    }
}
