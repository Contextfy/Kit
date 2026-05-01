//! BM25 full-text search slice
//!
//! This module provides BM25 full-text search abstraction with Tantivy backend.
//! It isolates Tantivy-specific types from the kernel layer, allowing the
//! core engine to remain backend-agnostic.
//!
//! ## Architecture
//!
//! - **trait_.rs**: Bm25StoreTrait - backend-agnostic interface
//! - **schema.rs**: Tantivy schema definitions (private)
//! - **index.rs**: Tantivy index creation and management (private)
//! - **tantivy_impl.rs**: Concrete Tantivy implementation (private)
//!
//! ## Usage Pattern
//!
//! ```ignore
//! use contextfy_core::slices::bm25::{Bm25StoreTrait, Bm25Result};
//! use contextfy_core::kernel::types::Query;
//!
//! // External code depends only on the trait, not concrete implementation
//! async fn search_bm25(store: &dyn Bm25StoreTrait, query: &Query) {
//!     let results = store.search(query).await?;
//!     // Process results...
//! }
//! ```
//!
//! **MANDATORY CONSTRAINT**: Only the trait and result type are publicly exported.
//! Concrete implementations (TantivyBm25Store) and helper functions are private
//! to prevent infrastructure leakage across module boundaries.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

pub mod trait_;

// Concrete implementations and helpers are private to prevent infrastructure leakage
mod schema;
// index is pub(crate) for facade factory access
pub(crate) mod index;
// tantivy_impl is pub(crate) for facade factory access
pub(crate) mod tantivy_impl;

// **MANDATORY**: Only export the trait and result type, NOT concrete implementations
// Concrete types like TantivyBm25Store must not be accessible from outside this slice
pub use trait_::{Bm25Result, Bm25StoreTrait};
