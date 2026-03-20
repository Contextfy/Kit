//! Core domain types for the knowledge engine
//!
//! This module defines minimal, stable types that are shared across all slices.
//! These types contain NO infrastructure-specific payloads (no Arrow arrays,
//! no LanceDB vectors, no Tantivy documents).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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
        match self.score.partial_cmp(&other.score) {
            Some(Ordering::Greater) => Some(Ordering::Greater),  // self score > other score → self is better
            Some(Ordering::Less) => Some(Ordering::Less),         // self score < other score → self is worse
            Some(Ordering::Equal) => Some(Ordering::Equal),       // scores are equal → hits are equal
            None => None,
        }
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
        assert!(hit1 > hit3);  // 0.9 > 0.7 → hit1 is better
        assert!(hit3 > hit2);  // 0.7 > 0.5 → hit3 is better
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
}
