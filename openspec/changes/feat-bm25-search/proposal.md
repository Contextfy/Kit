# Change: Implement BM25 Full-Text Search

## Why

Issue #8 has completed the Tantivy infrastructure (Schema definition and index initialization), but the core engine is still using naive text matching search (the `.contains()` method in `storage/mod.rs:search()`). This temporary implementation cannot provide production-grade search quality, lacks professional relevance scoring (BM25), and cannot effectively handle Chinese word segmentation.

This change will activate the Tantivy engine to implement true BM25 search algorithm and lay the foundation for future hybrid retrieval (vector search + BM25).

## What Changes

### Core Functionality Implementation

- **Implement Indexer struct**: Provide `Indexer::new(index)` and `Indexer::add_doc(doc)` methods
  - Convert `KnowledgeRecord` to Tantivy `Document`
  - Handle `commit()` logic correctly to ensure data is searchable
  - Error handling: Convert `TantivyError` to standard system `Result`

- **Implement Searcher struct**: Provide `Searcher::new(index)` and `Searcher::search(query)` methods
  - Build Tantivy `QueryParser` to parse user search terms
  - **CRITICAL**: QueryParser MUST use the registered `jieba` tokenizer for Chinese word segmentation
  - Execute queries and extract built-in BM25 scores
  - Return result struct containing `score` field

- **Architecture replacement**: Modify `packages/core/src/storage/mod.rs`
  - Remove temporary `.contains()` or space-based naive matching implementation
  - Integrate Tantivy index in `KnowledgeStore` (using filesystem index)
  - Replace `search()` method to call `Searcher::search`

### Performance Requirements

- Search latency: With 1000 mock documents inserted, `Searcher::search` query latency < 100ms
- Result ordering: Search results MUST be ordered by BM25 score in descending order

### Quality Gate

1. Prohibit `unwrap()` abuse, all Tantivy errors must be correctly converted to `Result` for upward propagation
2. Ensure `commit()` is called after writing data, otherwise newly written data cannot be searched
3. Code must pass `cargo fmt`, `cargo clippy`, `cargo test`

**BREAKING**: None

## Impact

- **Affected specs**: `core-engine` (MODIFIED: Knowledge Storage, ADDED: BM25 Search)
- **Affected code**:
  - `packages/core/src/search/mod.rs` - Add Indexer and Searcher structs
  - `packages/core/src/storage/mod.rs` - Integrate Tantivy index, replace search() implementation
  - `packages/core/src/lib.rs` - Ensure new types are correctly exported

- **Performance impact**: Search quality significantly improved, latency optimized from O(n) to O(log n)
- **Compatibility**: Storage format remains unchanged (JSON), adding index files (.tantivy directory)
