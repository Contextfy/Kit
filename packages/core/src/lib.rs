pub mod bridge;
pub mod embeddings;
pub mod facade;
pub mod kernel;
pub mod parser;
// Private implementation module - only specific types are re-exported below
mod slices;

pub use bridge::{BridgeApi, BridgeError};
pub use embeddings::EmbeddingModel;
pub use facade::{build_hybrid_orchestrator, DeleteResult, DocumentDetails, SearchEngine};
pub use kernel::{AppError, DomainError, Hit, InfraError, Query, Score};
pub use parser::{parse_markdown, slice_by_headers, ParsedDoc, SlicedDoc, SlicedSection};

// Slice exports (Phase 3)
// NOTE: Storage traits only - concrete implementations like LanceDbStore should not be exposed
// Algorithm/facade types like RrfOrchestrator are exceptions as they are domain services
pub use slices::vector::VectorStoreTrait;
pub use slices::bm25::{Bm25StoreTrait, Bm25Result};
pub use slices::hybrid::{RrfOrchestrator, RrfResult};
