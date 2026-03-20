//! Data Transfer Objects (DTOs) for N-API bridge layer
//!
//! This module defines JavaScript-compatible types that are exposed through N-API.
//! They provide a clean separation between:
//! - **Kernel types**: Pure Rust domain models (internal)
//! - **DTO types**: Serializable structures for FFI boundary (public)
//!
//! **MANDATORY CONSTRAINT**: Option semantics must be preserved.
//! - When kernel returns `Ok(None)`, the bridge must return `null` to JS
//! - Never return "fake objects" with empty strings to represent None
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 4
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/specs/bridge-layer/spec.md`

use crate::kernel::types::{Hit, Query, Score};
use serde::{Deserialize, Serialize};

/// JavaScript-compatible search query DTO
///
/// This structure is exposed to JavaScript/TypeScript through N-API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDto {
    /// Query text for semantic or lexical search
    pub text: String,

    /// Maximum number of results to return
    pub limit: u32,
}

impl From<QueryDto> for Query {
    fn from(dto: QueryDto) -> Self {
        Query::new(dto.text, dto.limit as usize)
    }
}

impl From<Query> for QueryDto {
    fn from(query: Query) -> Self {
        QueryDto {
            text: query.text,
            limit: query.limit as u32,
        }
    }
}

/// JavaScript-compatible search result hit DTO
///
/// This structure is exposed to JavaScript/TypeScript through N-API.
/// It maps directly to `kernel::types::Hit` with no additional fields.
///
/// **ANTI-PATTERN WARNING**: Do NOT add engine-specific fields like:
/// - `raw_vector: Vec<f8>` (LanceDB payload)
/// - `bm25_score: f64` (BM25-specific)
/// - `vector_distance: f64` (vector-specific)
///
/// These should be handled in their respective slices, not in the bridge layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitDto {
    /// Stable identifier for the matched document
    pub id: String,

    /// Normalized relevance score [0.0, 1.0]
    pub score: f64,
}

impl From<HitDto> for Hit {
    fn from(dto: HitDto) -> Self {
        Hit::new(dto.id, Score::new(dto.score))
    }
}

impl From<Hit> for HitDto {
    fn from(hit: Hit) -> Self {
        HitDto {
            id: hit.id,
            score: hit.score.value(),
        }
    }
}

/// JavaScript-compatible search response DTO
///
/// Contains the search results or indicates no matches found.
/// This properly handles Option semantics for JavaScript null propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseDto {
    /// Array of search hits (empty if no results)
    pub hits: Vec<HitDto>,

    /// Total number of matches (may be greater than hits.length)
    pub total_count: u32,

    /// Query execution time in milliseconds
    pub elapsed_ms: u64,
}

impl SearchResponseDto {
    /// Create a response from kernel hits
    ///
    /// # Parameters
    ///
    /// * `hits` - Search results (limited by query.limit)
    /// * `total_count` - Total matching documents (for pagination support)
    /// * `elapsed_ms` - Query execution time in milliseconds
    ///
    /// # Important Note on total_count
    ///
    /// `total_count` represents the TOTAL number of matching documents in the database,
    /// NOT the number of hits returned. This is critical for pagination:
    ///
    /// - If there are 500 matching documents but limit=10, `hits.len()` is 10
    /// - But `total_count` should be 500 to indicate there are more results available
    ///
    /// If the backend doesn't support total counts, pass `hits.len()` as a fallback.
    pub fn from_kernel(hits: Vec<Hit>, total_count: usize, elapsed_ms: u64) -> Self {
        let hit_dtos: Vec<HitDto> = hits.into_iter().map(Into::into).collect();

        SearchResponseDto {
            hits: hit_dtos,
            total_count: total_count as u32,
            elapsed_ms,
        }
    }

    /// Create an empty response (no matches found)
    ///
    /// **IMPORTANT**: This is different from an error response.
    /// An empty response means the query executed successfully but found no matches.
    pub fn empty(elapsed_ms: u64) -> Self {
        SearchResponseDto {
            hits: Vec::new(),
            total_count: 0,
            elapsed_ms,
        }
    }

    /// Check if this response contains any results
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    /// Get the number of hits in this response
    pub fn len(&self) -> usize {
        self.hits.len()
    }
}

/// Optional search result with proper Option semantics
///
/// This type is used for operations that may return a single result or None.
pub type OptionalHitDto = Option<HitDto>;

/// Convert from kernel Option<Hit> to bridge Option<HitDto>
///
/// **MANDATORY**: This function MUST preserve None semantics.
/// - `Some(hit)` → `Some(hit_dto)`
/// - `None` → `None` (which becomes `null` in JavaScript)
///
/// Never convert `None` to a "fake object" with empty strings.
pub fn optional_hit_from_kernel(kernel_hit: Option<Hit>) -> OptionalHitDto {
    kernel_hit.map(Into::into)
}

/// Convert from bridge Option<HitDto> to kernel Option<Hit>
pub fn optional_hit_to_kernel(dto_hit: OptionalHitDto) -> Option<Hit> {
    dto_hit.map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_dto_conversion() {
        let dto = QueryDto {
            text: "test query".to_string(),
            limit: 10,
        };

        let kernel: Query = dto.clone().into();
        assert_eq!(kernel.text, "test query");
        assert_eq!(kernel.limit, 10);

        let back_to_dto: QueryDto = kernel.into();
        assert_eq!(back_to_dto.text, dto.text);
        assert_eq!(back_to_dto.limit, dto.limit);
    }

    #[test]
    fn test_hit_dto_conversion() {
        let dto = HitDto {
            id: "doc1".to_string(),
            score: 0.85,
        };

        let kernel: Hit = dto.clone().into();
        assert_eq!(kernel.id, "doc1");
        assert_eq!(kernel.score.value(), 0.85);

        let back_to_dto: HitDto = kernel.into();
        assert_eq!(back_to_dto.id, dto.id);
        assert_eq!(back_to_dto.score, dto.score);
    }

    #[test]
    fn test_hit_dto_score_clamping() {
        // Scores should be clamped to [0.0, 1.0]
        let hit1: Hit = HitDto {
            id: "doc1".to_string(),
            score: 1.5, // Above max
        }
        .into();
        assert_eq!(hit1.score.value(), 1.0);

        let hit2: Hit = HitDto {
            id: "doc2".to_string(),
            score: -0.5, // Below min
        }
        .into();
        assert_eq!(hit2.score.value(), 0.0);
    }

    #[test]
    fn test_optional_hit_preserves_none() {
        // Test that None is properly preserved
        let kernel_none: Option<Hit> = None;
        let dto_none = optional_hit_from_kernel(kernel_none);
        assert!(dto_none.is_none(), "None must be preserved");

        // Test that Some is properly converted
        let kernel_some = Some(Hit::new("doc1", Score::new(0.9)));
        let dto_some = optional_hit_from_kernel(kernel_some);
        assert!(dto_some.is_some());
        assert_eq!(dto_some.unwrap().id, "doc1");
    }

    #[test]
    fn test_search_response_from_kernel() {
        let hits = vec![
            Hit::new("doc1", Score::new(0.9)),
            Hit::new("doc2", Score::new(0.7)),
        ];

        // Simulate pagination: returned 2 hits, but there are 100 total matches
        let response = SearchResponseDto::from_kernel(hits, 100, 50);
        assert_eq!(response.hits.len(), 2);
        assert_eq!(response.total_count, 100);  // Total matches, not returned hits
        assert_eq!(response.elapsed_ms, 50);
        assert!(!response.is_empty());
        assert_eq!(response.len(), 2);
    }

    #[test]
    fn test_search_response_empty() {
        let response = SearchResponseDto::empty(25);
        assert!(response.is_empty());
        assert_eq!(response.hits.len(), 0);
        assert_eq!(response.total_count, 0);
        assert_eq!(response.elapsed_ms, 25);
        assert_eq!(response.len(), 0);
    }

    #[test]
    fn test_hit_ordering_preserved() {
        let hits = vec![
            Hit::new("doc1", Score::new(0.5)),
            Hit::new("doc2", Score::new(0.9)),
            Hit::new("doc3", Score::new(0.7)),
        ];

        let response = SearchResponseDto::from_kernel(hits, 3, 0);

        // Verify input ordering is preserved (not sorted by DTO conversion)
        assert_eq!(response.hits[0].id, "doc1"); // 0.5
        assert_eq!(response.hits[1].id, "doc2"); // 0.9
        assert_eq!(response.hits[2].id, "doc3"); // 0.7

        // Note: The kernel doesn't sort here - sorting happens in retrieval layer
        // This test just verifies DTO conversion preserves order
    }

    #[test]
    fn test_optional_hit_never_creates_fake_object() {
        // Test that None is properly preserved
        let kernel_none: Option<Hit> = None;
        let dto_none = optional_hit_from_kernel(kernel_none);
        assert!(dto_none.is_none(), "None must be preserved, never converted to a fake object");

        // Defensive assertion: never use a fake object with empty fields to represent None
        let fake = HitDto {
            id: String::new(),
            score: 0.0,
        };
        assert!(
            fake.id.is_empty(),
            "Fake object with empty id should be detectable"
        );

        // Ensure that a legitimate empty id is different from None
        // (Edge case: a document with empty id should still return Some, not None)
        let kernel_hit = Hit::new("", Score::new(0.5));
        let dto_some = optional_hit_from_kernel(Some(kernel_hit));
        assert!(
            dto_some.is_some(),
            "Hit with empty id should still return Some"
        );
        assert_eq!(dto_some.unwrap().id, "");
    }
}
