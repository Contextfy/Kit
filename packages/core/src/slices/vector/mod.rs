//! Vector storage slice
//!
//! This module provides vector storage abstraction with LanceDB backend.
//! It isolates LanceDB-specific types from the kernel layer, allowing the
//! core engine to remain backend-agnostic.
//!
//! ## Architecture
//!
//! - **trait_.rs**: VectorStoreTrait - backend-agnostic interface
//! - **schema.rs**: Arrow schema definitions for LanceDB (private)
//! - **connection.rs**: LanceDB connection and table management (private)
//! - **lancedb_impl.rs**: Concrete LanceDB implementation (private)
//!
//! ## Usage Pattern
//!
//! ```ignore
//! use contextfy_core::slices::vector::VectorStoreTrait;
//! use contextfy_core::kernel::types::Query;
//!
//! // External code depends only on the trait, not concrete implementation
//! async fn search_vector(store: &dyn VectorStoreTrait, query: &Query) {
//!     let results = store.search(query).await?;
//!     // Process results...
//! }
//! ```
//!
//! **MANDATORY CONSTRAINT**: Only the trait is publicly exported.
//! Concrete implementations (LanceDbStore) and helper functions are private
//! to prevent infrastructure leakage across module boundaries.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

pub mod trait_;

// Concrete implementations and helpers are private to prevent infrastructure leakage
pub(crate) mod connection;
pub(crate) mod lancedb_impl;
pub(crate) mod schema;

// **MANDATORY**: Only export the trait, NOT concrete implementations
// Concrete types like LanceDbStore must not be accessible from outside this slice
pub use trait_::VectorStoreTrait;
