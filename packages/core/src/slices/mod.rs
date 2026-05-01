//! Storage slices
//!
//! This module contains isolated storage implementations for different
//! retrieval engines. Each slice is self-contained and exports a trait
//! for dependency injection.
//!
//! ## Module Structure
//!
//! - **vector/**: Vector storage abstraction with LanceDB backend
//! - **bm25/**: BM25 full-text search with Tantivy backend
//! - **hybrid/**: Hybrid retrieval orchestration (RRF fusion)
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

pub mod vector;
pub mod bm25;
pub mod hybrid;
