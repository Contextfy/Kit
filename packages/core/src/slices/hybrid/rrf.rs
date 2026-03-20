//! Reciprocal Rank Fusion (RRF) orchestrator
//!
//! This module implements RRF for combining results from multiple retrieval methods.
//! RRF is a simple yet effective method for fusion that doesn't require score normalization.
//!
//! ## Algorithm
//!
//! RRF combines ranked lists by computing:
//! ```text
//! score(d) = sum(weight_i / (k + rank_i(d)))
//! ```
//!
//! where:
//! - `d` is a document
//! - `weight_i` is the weight for ranker i (default: 1.0)
//! - `rank_i(d)` is the rank of document d in ranker i's result list (1-indexed)
//! - `k` is a constant (default: 60)
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md`

use std::collections::HashMap;

use crate::kernel::types::{Hit, Score};
use crate::kernel::errors::{AppError, DomainError};

/// RRF fusion result with combined score
#[derive(Debug, Clone)]
pub struct RrfResult {
    /// Document ID
    pub id: String,
    /// Combined RRF score
    pub score: Score,
}

impl RrfResult {
    /// Create a new RRF result
    pub fn new(id: String, score: Score) -> Self {
        Self { id, score }
    }

    /// Convert to kernel Hit type
    pub fn to_hit(self) -> Hit {
        Hit::new(self.id, self.score)
    }
}

/// RRF fusion orchestrator
///
/// Combines results from multiple retrieval methods using Reciprocal Rank Fusion.
pub struct RrfOrchestrator {
    /// RRF constant k (default: 60)
    k: i32,
}

impl RrfOrchestrator {
    /// Create a new RRF orchestrator
    ///
    /// # Parameters
    ///
    /// * `k` - RRF constant (default: 60)
    ///
    /// # Returns
    ///
    /// Returns a new `RrfOrchestrator` instance.
    pub fn new(k: i32) -> Self {
        Self { k }
    }

    /// Create with default k=60
    ///
    /// This is a convenience method. For Default trait support, use
    /// `RrfOrchestrator::default()` or `RrfOrchestrator::default_k()`.
    pub fn default_k() -> Self {
        Self::new(60)
    }

    /// Fuse results from multiple rankers using RRF
    ///
    /// # Parameters
    ///
    /// * `results` - Vector of result lists from different rankers
    /// * `weights` - Optional weights for each ranker (default: all 1.0)
    ///
    /// # Returns
    ///
    /// Returns fused and sorted results.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No result lists provided
    /// - Weights length doesn't match results length
    pub fn fuse(
        &self,
        results: Vec<Vec<Hit>>,
        weights: Option<Vec<f64>>,
    ) -> Result<Vec<RrfResult>, AppError> {
        // Validate input
        if results.is_empty() {
            return Err(AppError::Domain(DomainError::Other(
                "No result lists provided for fusion".to_string(),
            )));
        }

        if let Some(ref w) = weights {
            if w.len() != results.len() {
                return Err(AppError::Domain(DomainError::Other(
                    "Weights length must match results length".to_string(),
                )));
            }
        }

        // Use default weights if not provided
        let weights = weights.unwrap_or_else(|| vec![1.0; results.len()]);

        // Accumulate RRF scores
        let mut scores: HashMap<String, f64> = HashMap::new();

        for (ranker_results, weight) in results.iter().zip(weights.iter()) {
            for (rank, hit) in ranker_results.iter().enumerate() {
                let rank = rank + 1; // 1-indexed rank
                let rrf_score = weight / (self.k as f64 + rank as f64);

                scores
                    .entry(hit.id.clone())
                    .and_modify(|s| *s += rrf_score)
                    .or_insert(rrf_score);
            }
        }

        // Convert to results and sort by score descending
        let mut fused_results: Vec<RrfResult> = scores
            .into_iter()
            .map(|(id, score)| RrfResult::new(id, Score::new(score)))
            .collect();

        fused_results.sort_by(|a, b| {
            b.score
                .value()
                .partial_cmp(&a.score.value())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        Ok(fused_results)
    }

    /// Fuse two result lists with equal weights
    ///
    /// This is a convenience function for the common case of fusing two rankers.
    ///
    /// # Parameters
    ///
    /// * `results1` - Results from first ranker
    /// * `results2` - Results from second ranker
    ///
    /// # Returns
    ///
    /// Returns fused and sorted results.
    ///
    /// # Errors
    ///
    /// Returns error only if no ranked lists are provided. Note that since
    /// `fuse_two` always provides exactly two lists to the underlying `fuse`
    /// method, this function will never actually return an error. Empty result
    /// lists are valid inputs and will produce an empty result.
    pub fn fuse_two(&self, results1: Vec<Hit>, results2: Vec<Hit>) -> Result<Vec<RrfResult>, AppError> {
        self.fuse(vec![results1, results2], None)
    }
}

impl Default for RrfOrchestrator {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_hit(id: &str, score: f64) -> Hit {
        Hit::new(id.to_string(), Score::new(score))
    }

    #[test]
    fn test_rrf_result_creation() {
        let result = RrfResult::new("test-id".to_string(), Score::new(0.5));

        assert_eq!(result.id, "test-id");
        assert_eq!(result.score.value(), 0.5);
    }

    #[test]
    fn test_rrf_result_to_hit() {
        let result = RrfResult::new("test-id".to_string(), Score::new(0.8));
        let hit = result.to_hit();

        assert_eq!(hit.id, "test-id");
        assert_eq!(hit.score.value(), 0.8);
    }

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = RrfOrchestrator::new(60);
        assert_eq!(orchestrator.k, 60);
    }

    #[test]
    fn test_orchestrator_default() {
        let orchestrator = RrfOrchestrator::default();
        assert_eq!(orchestrator.k, 60);
    }

    #[test]
    fn test_fuse_empty_results() {
        let orchestrator = RrfOrchestrator::default();
        let result = orchestrator.fuse(vec![], None);

        assert!(result.is_err());
    }

    #[test]
    fn test_fuse_single_ranker() {
        let orchestrator = RrfOrchestrator::default();

        let results = vec![vec![
            create_hit("doc-1", 0.9),
            create_hit("doc-2", 0.8),
            create_hit("doc-3", 0.7),
        ]];

        let fused = orchestrator.fuse(results, None).unwrap();

        // Results should be in the same order for single ranker
        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].id, "doc-1");
        assert_eq!(fused[1].id, "doc-2");
        assert_eq!(fused[2].id, "doc-3");
    }

    #[test]
    fn test_fuse_two_rankers() {
        let orchestrator = RrfOrchestrator::default();

        // Ranker 1: doc-1, doc-2, doc-3
        let results1 = vec![
            create_hit("doc-1", 0.9),
            create_hit("doc-2", 0.8),
            create_hit("doc-3", 0.7),
        ];

        // Ranker 2: doc-2, doc-1, doc-4
        let results2 = vec![
            create_hit("doc-2", 0.95),
            create_hit("doc-1", 0.85),
            create_hit("doc-4", 0.75),
        ];

        let fused = orchestrator.fuse_two(results1, results2)
            .expect("fuse_two should succeed");

        // doc-1 should be highest (rank 1 in R1, rank 2 in R2)
        // doc-2 should be second (rank 2 in R1, rank 1 in R2)
        assert_eq!(fused.len(), 4);

        // doc-1: 1/(60+1) + 1/(60+2) = 0.01639 + 0.01613 = 0.03252
        // doc-2: 1/(60+2) + 1/(60+1) = 0.01613 + 0.01639 = 0.03252
        // Should be equal, but tie-break by ID
        assert_eq!(fused[0].id, "doc-1");
        assert_eq!(fused[1].id, "doc-2");
        assert_eq!(fused[2].id, "doc-3"); // Only in R1
        assert_eq!(fused[3].id, "doc-4"); // Only in R2
    }

    #[test]
    fn test_fuse_with_weights() {
        let orchestrator = RrfOrchestrator::new(60);

        let results1 = vec![create_hit("doc-1", 0.9), create_hit("doc-2", 0.8)];
        let results2 = vec![create_hit("doc-2", 0.95), create_hit("doc-1", 0.85)];

        // Weight first ranker higher
        let weights = vec![2.0, 1.0];
        let fused = orchestrator.fuse(vec![results1, results2], Some(weights)).unwrap();

        // doc-2 should be higher with weights (rank 2 in R1*2.0 + rank 1 in R2*1.0)
        // vs doc-1 (rank 1 in R1*2.0 + rank 2 in R2*1.0)
        // Actually doc-1 should still be higher due to rank 1 in higher-weighted ranker
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].id, "doc-1");
        assert_eq!(fused[1].id, "doc-2");
    }

    #[test]
    fn test_fuse_weight_mismatch() {
        let orchestrator = RrfOrchestrator::default();

        let results = vec![
            vec![create_hit("doc-1", 0.9)],
            vec![create_hit("doc-2", 0.8)],
        ];

        // Only one weight for two result lists
        let weights = vec![1.0];
        let result = orchestrator.fuse(results, Some(weights));

        assert!(result.is_err());
    }

    #[test]
    fn test_rrf_score_calculation() {
        let orchestrator = RrfOrchestrator::new(60);

        let results1 = vec![
            create_hit("doc-1", 0.9),  // rank 1
            create_hit("doc-2", 0.8),  // rank 2
        ];

        let results2 = vec![
            create_hit("doc-3", 0.95), // rank 1
            create_hit("doc-1", 0.85), // rank 2
        ];

        let fused = orchestrator.fuse_two(results1, results2)
            .expect("fuse_two should succeed");

        // doc-1: rank 1 in R1 + rank 2 in R2
        // score = 1/(60+1) + 1/(60+2) = 0.01639 + 0.01613 = 0.03252
        let doc1_score = fused.iter().find(|r| r.id == "doc-1").unwrap().score.value();
        assert!((doc1_score - 0.0325).abs() < 0.0001);

        // doc-2: rank 2 in R1 only
        // score = 1/(60+2) = 0.01613
        let doc2_score = fused.iter().find(|r| r.id == "doc-2").unwrap().score.value();
        assert!((doc2_score - 0.0161).abs() < 0.0001);

        // doc-3: rank 1 in R2 only
        // score = 1/(60+1) = 0.01639
        let doc3_score = fused.iter().find(|r| r.id == "doc-3").unwrap().score.value();
        assert!((doc3_score - 0.0164).abs() < 0.0001);
    }
}
