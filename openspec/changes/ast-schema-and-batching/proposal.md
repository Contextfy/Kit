# Change: AST Schema 重塑与批量操作优化

## Why

当前存储引擎使用**文档级** Schema（`title`, `summary`, `content`），无法精确表示代码的 AST 结构，且逐条插入性能瓶颈严重（1000 文档构建 > 20s）。为支持代码语义检索并实现 Issue #22 的性能目标（构建 < 10s），需要将底层存储结构重塑为"AST 节点模型"并实现批量操作。

## What Changes

- **新增 `AstChunk` 结构体**：包含 `id`, `file_path`, `symbol_name`, `node_type`, `content`, `dependencies`, `vector` 7 个字段
- **LanceDB Schema 重构**：从文档字段映射到 AST 字段，`dependencies` 序列化为逗号分隔字符串（避开 Arrow ListArray）
- **Tantivy Schema 重构**：新增字段，`symbol_name` 获得 5.0 倍权重（精确符号检索）
- **批量操作接口**：`VectorStoreTrait::add_batch()` 和 `Bm25StoreTrait::add_batch()`
- **性能优化防线**：FastEmbed 批量调用、LanceDB 单次 RecordBatch 写入、Tantivy 单次事务提交

**BREAKING**:
- ❌ Schema 字段变更：`title, summary, keywords` → `file_path, symbol_name, node_type, dependencies`
- ✅ 向后兼容：Facade 层保留 `add()` 方法，内部映射到 `AstChunk`

## Impact

- Affected specs: core-engine
- Affected code:
  - packages/core/src/kernel/types.rs - 新增 `AstChunk`
  - packages/core/src/slices/vector/schema.rs - LanceDB Schema 重构
  - packages/core/src/slices/bm25/schema.rs - Tantivy Schema 重构
  - packages/core/src/slices/vector/lancedb_impl.rs - `add_batch` 实现
  - packages/core/src/slices/bm25/tantivy_impl.rs - `add_batch` 实现
  - packages/core/src/slices/{vector,bm25}/trait_.rs - Trait 扩展
  - packages/core/src/facade.rs, slices/hybrid/mod.rs - `add_batch` 方法
