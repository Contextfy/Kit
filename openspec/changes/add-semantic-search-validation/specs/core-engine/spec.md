## ADDED Requirements

### Requirement: Semantic Search Evaluation

The system SHALL provide an automated evaluation framework to validate the effectiveness of hybrid search (BM25 + Vector + RRF) compared to pure BM25 search.

The evaluation framework SHALL:

1. Support semantic query testing including:
   - Synonym queries (e.g., "heal player" → "applyDamage")
   - Action variants (e.g., "create block" → "Block.create()")
   - Multilingual queries (e.g., "方块" → "Block")
   - Functional descriptions (e.g., "spawn entity" → "Entity.create()")

2. Use multi-level relevance scoring for expected documents:
   - **Score 3**: Perfect match (exact API, e.g., "heal player" → "applyDamage")
   - **Score 2**: Highly relevant (same category method, e.g., "heal player" → "hurtEntity")
   - **Score 1**: Partially relevant (concept-related, e.g., "heal player" → "Entity")
   - **Score 0**: Not relevant

3. Calculate standard information retrieval metrics:
   - Accuracy@K (for K = 1, 3, 5)
   - NDCG@K (Normalized Discounted Cumulative Gain) with multi-level relevance
   - Hit Rate

4. Generate a Markdown evaluation report (`docs/SEMANTIC_EVALUATION_REPORT.md`) containing:
   - Generation timestamp
   - Summary comparison table (BM25 vs Hybrid)
   - Detailed per-query results (Top-3)
   - Metric analysis and improvement percentage
   - Quality gate verification

#### Scenario: Evaluation framework runs successfully

- **WHEN** the evaluation test is executed (`cargo test --test semantic_evaluation`)
- **THEN** the test completes within 60 seconds on warm-cache runs (model already downloaded)
- **AND** on cold-start runs (first execution), the test may take 1-5 minutes for BGE-small-en model download
- **AND** all test queries are executed against both BM25 and Hybrid search engines
- **AND** evaluation metrics are calculated correctly
- **AND** a Markdown report is generated at `docs/SEMANTIC_EVALUATION_REPORT.md`

#### Scenario: Hybrid search outperforms BM25

- **WHEN** semantic queries are executed
- **THEN** Hybrid search Top-3 accuracy SHALL be greater than BM25 Top-3 accuracy
- **AND** Hybrid search NDCG@3 SHALL be greater than BM25 NDCG@3
- **AND** semantic queries (e.g., "heal player") SHALL find synonym-related documents (e.g., "applyDamage")

#### Scenario: Quality gate verification

- **WHEN** the evaluation report is generated
- **THEN** the report SHALL include a quality gate section
- **AND** the quality gate SHALL verify Hybrid Top-3 accuracy ≥ 80%
- **AND** the report SHALL indicate "PASSED" if the quality gate is met
- **AND** the report SHALL indicate "FAILED" if the quality gate is not met

#### Scenario: Evaluation metrics calculation accuracy

- **WHEN** evaluation metrics are calculated
- **THEN** Accuracy@K SHALL equal (queries with relevant docs in Top-K) / (total queries)
- **AND** NDCG@K SHALL be calculated using the standard formula: DCG@K / IDCG@K
- **AND** metric calculations SHALL be validated by unit tests

#### Scenario: Test data reproducibility

- **WHEN** the evaluation test is run multiple times
- **THEN** the results SHALL be deterministic and reproducible
- **AND** the test data (queries and expected documents) SHALL be version-controlled
- **AND** test documents SHALL use Mock data or versioned real documents
