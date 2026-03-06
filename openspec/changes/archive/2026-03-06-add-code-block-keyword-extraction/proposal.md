# Change: 代码块关键词提取与搜索权重增强

## Why

Issue #9 完成了 BM25 全文搜索基础设施，并在 `KnowledgeRecord` 和 Tantivy schema 中添加了 `keywords` 字段。但当前 `keywords` 字段为空（stubbed 为 `vec![]`），且没有任何提取逻辑。

当用户搜索代码块中的特定 API 名称、函数名或类名（如 "createItem"、"BlockCustomComponent"）时，当前实现将这些关键词与普通正文文本同等对待。这意味着仅在代码中包含这些术语的文档无法获得应有的搜索相关性，导致开发者难以高效查找 API 文档。

## What Changes

### 标识符提取（Parser 层）

- **实现基于正则的代码块关键词提取**：在文档解析期间添加从 Markdown 代码块提取标识符的函数
  - 提取函数名：如 `function createItem()`、`def build_server():`、`fn process_data()` 等模式
  - 提取类/类型名：如 `class BlockCustomComponent`、`struct Config`、`interface User` 等模式
  - 提取变量名（常见命名约定）：`CamelCase`、`PascalCase`、`snake_case`
  - 使用缓存正则模式（通过 `lazy_static` 或 `std::sync::OnceLock`）避免循环中重复编译
  - 对提取的关键词去重后存入 `KnowledgeRecord.keywords`

### 搜索权重增强（Searcher 层）

- **在 Tantivy 查询中提升关键词字段权重**：修改 `Searcher::new()` 使用字段权重提升
  - 将 `keywords_field` 添加到 `QueryParser` 的字段列表
  - 使用 `set_field_boost()` API 为 `FIELD_KEYWORDS` 分配高权重（5.0 - 10.0）
  - 确保精确的 API 名称匹配排在搜索结果最前面

### 集成（Storage 层）

- **文档索引期间填充关键词**：更新 `Indexer::add_doc()` 将提取的关键词写入索引
- **解析管道中传递关键词**：修改 `KnowledgeStore::add()` 在创建 `KnowledgeRecord` 时提取并包含关键词

**BREAKING**: None

## Impact

- **受影响的规范**：`core-engine`（MODIFIED: Knowledge Storage, BM25 Search）
- **受影响的代码**：
  - `packages/core/src/parser/mod.rs` - 添加代码块关键词提取函数
  - `packages/core/src/search/mod.rs` - 在 QueryParser 中为关键词字段添加权重提升
  - `packages/core/src/storage/mod.rs` - 将提取的关键词传递给 KnowledgeRecord 和 Indexer

- **性能影响**：解析期间的正则提取开销极小（使用缓存模式）；搜索性能不变（字段提升在查询时计算）
- **开发者体验**：显著改善 API 文档可发现性 - 精确的 API 名称搜索将始终在最前面返回最相关的结果
