//! Core domain types for the knowledge engine
//!
//! This module defines minimal, stable types that are shared across all slices.
//! These types contain NO infrastructure-specific payloads (no Arrow arrays,
//! no LanceDB vectors, no Tantivy documents).

use serde::{Deserialize, Serialize};

/// A normalized search query
///
/// Contains only the essential query information needed for retrieval.
/// Infrastructure-specific query parameters should be handled in respective slices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Query {
    /// The query text for semantic or lexical search
    pub text: String,

    /// Maximum number of results to return
    pub limit: usize,
}

impl Query {
    /// Create a new search query
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            limit,
        }
    }
}

/// A relevance score for search results
///
/// Represents a normalized score in the range [0.0, 1.0].
/// Higher scores indicate better relevance.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Score(pub f64);

impl Score {
    /// Maximum possible score (perfect match)
    pub const MAX: Self = Self(1.0);

    /// Minimum possible score (no relevance)
    pub const MIN: Self = Self(0.0);

    /// Create a new score, clamping to valid range [0.0, 1.0]
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the underlying float value
    pub fn value(self) -> f64 {
        self.0
    }

    /// Check if this score indicates meaningful relevance
    pub fn is_relevant(self) -> bool {
        self.0 > 0.0
    }
}

/// A minimal search result hit
///
/// **MANDATORY CONSTRAINT**: This type MUST remain minimal and infrastructure-agnostic.
/// It contains ONLY stable identifier and score fields.
///
/// # Anti-Patterns (DO NOT DO THIS):
/// - ❌ Add `raw_vector: Vec<f8>` to embed LanceDB payloads
/// - ❌ Add `arrow_batch: arrow::array::RecordBatch` for Arrow data
/// - ❌ Add `tantivy_doc: tantivy::Document` for Tantivy data
/// - ❌ Add engine-specific metadata (e.g., "lancedb_distance", "bm25_tf_idf")
///
/// # Valid Fields:
/// - `id`: Stable identifier (can be a UUID, string path, etc.)
/// - `score`: Normalized relevance score
///
/// Additional stable fields (e.g., `title`, `summary`) may be added ONLY if they
/// are consistent across ALL retrieval engines (vector, BM25, hybrid).
///
/// Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 2
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hit {
    /// Stable identifier for the matched document
    pub id: String,

    /// Normalized relevance score [0.0, 1.0]
    pub score: Score,
}

impl Hit {
    /// Create a new hit with the given ID and score
    pub fn new(id: impl Into<String>, score: Score) -> Self {
        Self {
            id: id.into(),
            score,
        }
    }

    /// Create a hit from a raw score value
    pub fn with_raw_score(id: impl Into<String>, score: f64) -> Self {
        Self::new(id, Score::new(score))
    }
}

impl PartialOrd for Hit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Higher scores should be considered "greater" (better ranking)
        self.score.partial_cmp(&other.score)
    }
}

/// A parsed AST chunk from a Python sidecar process
///
/// Represents a code symbol (function, class, method, etc.) extracted by
/// the Python `cocoindex` tool and transmitted via IPC (JSONL format).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AstChunk {
    /// Absolute or relative path to the source file
    pub file_path: String,

    /// Symbol name (e.g., "MyClass", "my_function")
    pub symbol_name: String,

    /// Node type (e.g., "function", "class", "method", "variable")
    pub node_type: String,

    /// AST content representation (could be source code snippet or structured data)
    pub ast_content: String,

    /// List of dependencies (other symbols this chunk references)
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl AstChunk {
    /// Create a new AST chunk
    pub fn new(
        file_path: impl Into<String>,
        symbol_name: impl Into<String>,
        node_type: impl Into<String>,
        ast_content: impl Into<String>,
        dependencies: Vec<String>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            symbol_name: symbol_name.into(),
            node_type: node_type.into(),
            ast_content: ast_content.into(),
            dependencies,
        }
    }

    /// Create an AST chunk with no dependencies
    pub fn without_dependencies(
        file_path: impl Into<String>,
        symbol_name: impl Into<String>,
        node_type: impl Into<String>,
        ast_content: impl Into<String>,
    ) -> Self {
        Self::new(file_path, symbol_name, node_type, ast_content, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_clamping() {
        assert_eq!(Score::new(1.5).value(), 1.0);
        assert_eq!(Score::new(-0.5).value(), 0.0);
        assert_eq!(Score::new(0.5).value(), 0.5);
    }

    #[test]
    fn test_score_relevance() {
        assert!(Score::new(0.5).is_relevant());
        assert!(!Score::new(0.0).is_relevant());
    }

    #[test]
    fn test_hit_ordering() {
        let hit1 = Hit::new("doc1", Score::new(0.9));
        let hit2 = Hit::new("doc2", Score::new(0.5));
        let hit3 = Hit::new("doc3", Score::new(0.7));

        // Higher scores should be considered "greater"
        assert!(hit1 > hit3); // 0.9 > 0.7 → hit1 is better
        assert!(hit3 > hit2); // 0.7 > 0.5 → hit3 is better
    }

    #[test]
    fn test_query_creation() {
        let query = Query::new("test query", 10);
        assert_eq!(query.text, "test query");
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_hit_creation() {
        let hit = Hit::with_raw_score("doc1", 0.85);
        assert_eq!(hit.id, "doc1");
        assert_eq!(hit.score.value(), 0.85);
    }

    #[test]
    fn test_score_validation() {
        // Valid scores
        assert!(Score::new(0.5).is_relevant());
        assert!(Score::new(1.0).is_relevant());

        // Invalid scores
        assert!(!Score::new(0.0).is_relevant());
        assert!(!Score::new(-0.1).is_relevant());
    }

    #[test]
    fn test_query_validation() {
        // Valid queries
        let q1 = Query::new("test", 10);
        assert_eq!(q1.text, "test");
        assert_eq!(q1.limit, 10);

        // Empty query should be handled at domain level
        let q2 = Query::new("", 10);
        assert!(q2.text.is_empty());
    }

    #[test]
    fn test_ast_chunk_creation() {
        let chunk = AstChunk::new(
            "/path/to/file.py",
            "MyClass",
            "class",
            "class MyClass:\n    pass",
            vec!["OtherClass".to_string()],
        );

        assert_eq!(chunk.file_path, "/path/to/file.py");
        assert_eq!(chunk.symbol_name, "MyClass");
        assert_eq!(chunk.node_type, "class");
        assert_eq!(chunk.ast_content, "class MyClass:\n    pass");
        assert_eq!(chunk.dependencies, vec!["OtherClass"]);
    }

    #[test]
    fn test_ast_chunk_without_dependencies() {
        let chunk = AstChunk::without_dependencies(
            "/path/to/file.py",
            "my_function",
            "function",
            "def my_function():\n    pass",
        );

        assert_eq!(chunk.file_path, "/path/to/file.py");
        assert_eq!(chunk.symbol_name, "my_function");
        assert_eq!(chunk.node_type, "function");
        assert!(chunk.dependencies.is_empty());
    }

    #[test]
    fn test_ast_chunk_serialization() {
        let chunk = AstChunk::new(
            "test.py",
            "foo",
            "function",
            "pass",
            vec!["bar".to_string()],
        );

        // Test serialization
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"file_path\":\"test.py\""));
        assert!(json.contains("\"symbol_name\":\"foo\""));

        // Test deserialization
        let deserialized: AstChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, chunk);
    }

    #[test]
    fn test_ast_chunk_default_dependencies() {
        // JSON without dependencies field should default to empty array
        let json = r#"{"file_path":"test.py","symbol_name":"foo","node_type":"function","ast_content":"pass"}"#;
        let chunk: AstChunk = serde_json::from_str(json).unwrap();

        assert!(chunk.dependencies.is_empty());
    }
}
