//! Hybrid retrieval orchestration slice
//!
//! This module provides hybrid retrieval orchestration using multiple methods.
//! It combines results from different retrieval backends (BM25, vector, etc.)
//! using fusion algorithms like RRF (Reciprocal Rank Fusion).
//!
//! ## Architecture
//!
//! - **rrf.rs**: Reciprocal Rank Fusion implementation for result fusion
//! - **orchestrator.rs**: High-level orchestration of multiple retrieval methods
//!
//! ## Usage Pattern
//!
//! ```ignore
//! use contextfy_core::slices::hybrid::{RrfOrchestrator, HybridOrchestrator};
//! use contextfy_core::kernel::types::Query;
//!
//! // Create hybrid orchestrator
//! let orchestrator = HybridOrchestrator::default_with_stores(vector_store, bm25_store);
//!
//! // Search using both BM25 and vector
//! let query = Query::new("search text", 10);
//! let results = orchestrator.search(&query).await?;
//! ```
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

pub mod rrf;
pub mod orchestrator;

// Re-export main types at the module level
pub use rrf::{RrfOrchestrator, RrfResult};
pub use orchestrator::HybridOrchestrator;
