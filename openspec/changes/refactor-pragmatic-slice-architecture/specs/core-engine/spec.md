# Changes

## MODIFIED Requirements

### Requirement: Core Engine Layering and Search Orchestration

The core engine SHALL adopt a slice-based architecture with strict boundaries between kernel, bridge, and retrieval slices.

#### Scenario: Shared kernel types stay infrastructure-agnostic

- **WHEN** developers define or modify core query/result contracts
- **THEN** types are placed in `kernel/` and contain only stable cross-slice fields
- **AND** no Arrow, Tantivy, or LanceDB types are referenced in kernel modules

#### Scenario: Vector infrastructure is isolated behind trait

- **WHEN** vector storage behavior is implemented
- **THEN** upper orchestration depends on `VectorStoreTrait` only
- **AND** LanceDB schema/connection/implementation are confined to `slices/vector/`

#### Scenario: BM25 and Hybrid orchestration are modularized

- **WHEN** search requests are executed
- **THEN** BM25 retrieval is executed within `slices/bm25/`
- **AND** fusion logic (RRF) is executed within `slices/hybrid/`
- **AND** legacy monolithic KnowledgeStore orchestration path is removed

### Requirement: Hit Model Stability

The core engine SHALL keep `kernel::types::Hit` minimal and stable across slices.

#### Scenario: Hit excludes engine-specific payloads

- **WHEN** retrieval results are propagated across slices
- **THEN** `Hit` contains only stable identifiers and scores
- **AND** vector raw payloads, Arrow batch objects, and Tantivy document objects are excluded
