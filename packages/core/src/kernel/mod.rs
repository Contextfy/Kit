//! Shared kernel types and error model
//!
//! This module contains stable, infrastructure-agnostic types that are shared across all slices.
//! No Arrow, Tantivy, or LanceDB dependencies are allowed in this module.

pub mod errors;
pub mod types;

pub use errors::{AppError, DomainError, InfraError};
pub use types::{AstChunk, Hit, Query, Score};
