//! # Contextfy Bridge
//!
//! This crate provides Node.js bindings for the Contextfy core library using NAPI-RS.
//!
//! ## Example
//!
//! ```javascript
//! const { ContextfyKit } = require('contextfy-bridge');
//! const kit = new ContextfyKit();
//! const results = await kit.scout('query');
//! ```

use contextfy_core::retriever::Retriever;
use contextfy_core::storage::KnowledgeStore;
use napi_derive::napi;
use std::sync::Arc;

/// Contextfy API module exported to Node.js.
#[napi]
pub mod contextfy {
    use super::{Arc, Brief, Details, KnowledgeStore, Retriever};

    /// Main API wrapper for Contextfy functionality.
    ///
    /// This struct provides the primary interface for interacting with the Contextfy
    /// knowledge base from Node.js.
    ///
    /// # Example
    ///
    /// ```javascript
    /// const { ContextfyKit } = require('contextfy-bridge');
    /// const kit = new ContextfyKit();
    /// ```
    #[napi]
    pub struct ContextfyKit {
        retriever: Retriever<'static>,
        // Store must be kept alive since retriever holds a reference to it
        // We use Arc to allow multiple references and 'static lifetime since we own the data
        _store: Arc<KnowledgeStore>,
    }

    impl Default for ContextfyKit {
        fn default() -> Self {
            Self::new()
        }
    }

    #[napi]
    impl ContextfyKit {
        /// Creates a new `ContextfyKit` instance.
        ///
        /// This initializes the knowledge store with the default data directory (`.contextfy/data`).
        /// The embedding model is disabled by default for faster initialization.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const kit = new ContextfyKit();
        /// ```
        #[napi(constructor)]
        pub fn new() -> Self {
            // Initialize the KnowledgeStore with default path
            // Note: In a production environment, this should be configurable
            // and properly handle async initialization
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime");

            let store = rt.block_on(async {
                KnowledgeStore::new(".contextfy/data", None)
                    .await
                    .expect("Failed to initialize KnowledgeStore")
            });

            let store_arc = Arc::new(store);

            // SAFETY: We extend the lifetime to 'static since we own the Arc
            // and the retriever will be stored alongside the Arc
            let retriever = unsafe {
                std::mem::transmute::<Retriever<'_>, Retriever<'static>>(
                    Retriever::new(&*store_arc)
                )
            };

            Self {
                retriever,
                _store: store_arc,
            }
        }

        /// Searches the knowledge base for matching records.
        ///
        /// This performs a BM25 keyword search over the knowledge base.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        ///
        /// # Returns
        ///
        /// Returns a vector of brief information about matching records.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const results = await kit.scout('Rust');
        /// console.log(results); // [{ id, title, summary, score }, ...]
        /// ```
        #[napi]
        pub async fn scout(&self, query: String) -> napi::Result<Vec<Brief>> {
            self.retriever
                .scout(&query)
                .await
                .map(|core_briefs| {
                    core_briefs
                        .into_iter()
                        .map(|core_brief| Brief {
                            id: core_brief.id,
                            title: core_brief.title,
                            parent_doc_title: core_brief.parent_doc_title,
                            summary: core_brief.summary,
                            score: core_brief.score as f64, // Convert f32 to f64 for NAPI
                        })
                        .collect()
                })
                .map_err(|e| napi::Error::from_reason(format!("Search failed: {}", e)))
        }

        /// Retrieves detailed information about a specific record.
        ///
        /// # Arguments
        ///
        /// * `id` - The unique identifier of the record
        ///
        /// # Returns
        ///
        /// Returns detailed information including the full content of the record.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const details = await kit.inspect('record-id');
        /// console.log(details.content);
        /// ```
        #[napi]
        pub async fn inspect(&self, id: String) -> napi::Result<Details> {
            self.retriever
                .inspect(&id)
                .await
                .map(|details_opt| {
                    match details_opt {
                        Some(details) => Details {
                            id: details.id,
                            title: details.title,
                            content: details.content,
                        },
                        None => Details {
                            id,
                            title: String::new(),
                            content: String::new(),
                        },
                    }
                })
                .map_err(|e| napi::Error::from_reason(format!("Failed to retrieve record: {}", e)))
        }
    }
}

/// Re-exports the main ContextfyKit type for convenience.
pub use contextfy::ContextfyKit;

/// Brief information about a knowledge record.
///
/// This struct is returned by search operations and contains summary information
/// about a matching record.
///
/// # Fields
///
/// * `id` - Unique identifier for the record
/// * `title` - Title of the record
/// * `parent_doc_title` - Title of the parent document
/// * `summary` - Brief summary of the content (first 200 characters)
/// * `score` - BM25 relevance score
#[napi(object)]
pub struct Brief {
    /// Unique identifier for the record
    pub id: String,
    /// Title of the record
    pub title: String,
    /// Title of the parent document
    pub parent_doc_title: String,
    /// Brief summary of the content (first 200 characters)
    pub summary: String,
    /// BM25 relevance score (f64 for NAPI compatibility)
    pub score: f64,
}

/// Detailed information about a knowledge record.
///
/// This struct contains the complete content of a record and is returned by
/// the inspect operation.
///
/// # Fields
///
/// * `id` - Unique identifier for the record
/// * `title` - Title of the record
/// * `content` - Full content of the record
#[napi(object)]
pub struct Details {
    /// Unique identifier for the record
    pub id: String,
    /// Title of the record
    pub title: String,
    /// Full content of the record
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contextfy_kit_default() {
        let _kit = contextfy::ContextfyKit::default();
        // Verifies that Default implementation works without panic
    }

    #[test]
    fn test_contextfy_kit_new() {
        let _kit = contextfy::ContextfyKit::new();
        // Verifies that constructor works without panic
    }

    #[test]
    fn test_reexport() {
        // This test verifies that the re-export works correctly
        let _kit: ContextfyKit = ContextfyKit::new();
    }
}

// Note: Integration tests for async methods (scout, inspect) require a Node.js runtime
// and should be placed in the tests/ directory with a proper test harness.
// For NAPI-RS packages, consider using the JavaScript test suite in the parent project.
