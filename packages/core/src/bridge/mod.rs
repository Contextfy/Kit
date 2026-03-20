//! Native Bridge (FFI) layer
//!
//! This module provides the bridge between Rust core logic and JavaScript/TypeScript
//! through N-API. It enforces strict contracts for:
//!
//! - **Runtime Execution**: Unified sync/async FFI strategy with Tokio singleton guard
//! - **DTO Mapping**: Bidirectional conversion between kernel types and N-API objects
//! - **Error Mapping**: Layer-aware error translation that preserves root cause chains
//! - **Option Semantics**: Proper handling of None values without fake object fallbacks
//!
//! **MANDATORY CONSTRAINTS**:
//! 1. Never nest `block_on` calls - use runtime guard
//! 2. Never return fake objects with empty strings for None cases
//! 3. Never flatten errors with `.to_string()` - preserve chains
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rules 1, 3, 4

pub mod api;
pub mod dto;
pub mod error_map;
pub mod runtime_guard;

pub use api::BridgeApi;
pub use error_map::BridgeError;
