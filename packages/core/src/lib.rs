pub mod embeddings;
pub mod parser;
pub mod retriever;
pub mod search;
pub mod storage;

pub use embeddings::EmbeddingModel;
pub use parser::{parse_markdown, slice_by_headers, ParsedDoc, SlicedDoc, SlicedSection};
pub use retriever::{Brief, Details, Retriever};
pub use search::{
    create_index, create_schema, Indexer, SearchResult, Searcher, FIELD_CONTENT, FIELD_KEYWORDS,
    FIELD_SUMMARY, FIELD_TITLE,
};
pub use storage::{KnowledgeRecord, KnowledgeStore};
