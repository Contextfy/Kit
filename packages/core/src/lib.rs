pub mod bridge;
pub mod embeddings;
pub mod facade;
pub mod kernel;
pub mod parser;
pub mod slices;
pub mod storage;

pub use bridge::{BridgeApi, BridgeError};
pub use embeddings::EmbeddingModel;
pub use facade::{build_hybrid_orchestrator, DocumentDetails, SearchEngine};
pub use kernel::{AppError, DomainError, Hit, InfraError, Query, Score};
pub use parser::{parse_markdown, slice_by_headers, ParsedDoc, SlicedDoc, SlicedSection};
pub use storage::KnowledgeRecord;

// Slice exports (Phase 3)
// NOTE: Only export traits, not concrete implementations
// Concrete types like LanceDbStore should not be exposed to upper layers
pub use slices::vector::VectorStoreTrait;
pub use slices::bm25::{Bm25StoreTrait, Bm25Result};
pub use slices::hybrid::{RrfOrchestrator, RrfResult};
