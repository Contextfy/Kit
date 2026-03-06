# 变更：CLI Scout 命令显示 BM25 相关性分数

## Why

Issue #9 已经完成了 BM25 全文搜索的实现，`SearchResult` 结构体已经包含了 `score: f32` 字段。但是 CLI `scout` 命令无法向用户显示这个分数，原因是：

1. `retriever/mod.rs` 中的 `Brief` 结构体没有包含 `score` 字段
2. `KnowledgeStore::search()` 方法返回的 `Vec<KnowledgeRecord>` 没有分数信息
3. `Retriever::scout()` 方法无法将分数从 `SearchResult` 传递到 `Brief`
4. CLI `scout.rs` 命令无法访问分数进行显示

本变更将把 BM25 分数传播到整个检索管道，并在 CLI 输出中显示，让用户能够看到每个搜索结果的相关性。

## What Changes

- 在 `packages/core/src/retriever/mod.rs` 中的 `Brief` 结构体添加 `score: f32` 字段
- 修改 `KnowledgeStore::search()` 返回分数信息（返回 `Vec<(KnowledgeRecord, f32)>` 或创建新的结果结构体）
- 更新 `Retriever::scout()` 将搜索结果的分数传递给 `Brief`
- 更新 CLI `scout.rs` 命令，使用格式 `"Score: {:.2} | [title] content"` 显示分数
- 分数保留 2 位小数以提高可读性

### 额外优化 (Bonus Improvements)

- **CLI 终端颜色高亮**: 添加 `colored` 依赖，根据分数高低使用不同颜色（绿色/黄色/暗淡）提升可读性
- **Bridge 层对齐**: 修复 NAPI `Brief` 结构体的 Mock Drift，添加 `parent_doc_title` 和 `score` 字段（使用 `f64` 以兼容 NAPI）
- **代码质量**: 修复 server 层 clippy 警告（移除不必要的显式解引用）

**BREAKING**: None

## Impact

- **影响的 specs**:
  - `core-engine` (MODIFIED: Two-Stage Retrieval - Brief 结构体现在包含 score)
  - `cli` (MODIFIED: Scout 命令输出格式现在包含 BM25 分数和颜色高亮)
- **影响的代码**:
  - `packages/core/src/retriever/mod.rs` - 添加 score 字段到 Brief，更新 scout() 方法
  - `packages/core/src/storage/mod.rs` - 修改 search() 返回分数
  - `packages/cli/src/commands/scout.rs` - 在输出格式中显示分数和颜色高亮
  - `packages/cli/Cargo.toml` - 添加 `colored = "2.1"` 依赖
  - `packages/bridge/src/lib.rs` - NAPI Brief 结构体添加 parent_doc_title 和 score 字段
  - `packages/server/src/main.rs` - 修复 clippy 警告
- **用户体验影响**: 用户现在可以在搜索结果中看到相关性分数，帮助他们识别最相关的内容。颜色高亮使高分结果更醒目
