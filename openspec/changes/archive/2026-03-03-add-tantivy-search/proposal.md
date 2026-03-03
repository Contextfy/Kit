# 变更：添加 Tantivy 全文搜索基础设施

## 为什么

当前核心引擎使用基础的 JSON 文件存储和内存中文本匹配进行搜索。根据 `openspec/project.md` 文档，项目架构指定使用 **Tantivy** 进行 BM25 关键词搜索，但该功能尚未实现。为了支持两阶段检索模式（侦察/检视）并启用支持中文分词的生产级全文搜索，我们需要添加 Tantivy 基础设施层。

此变更是未来混合检索（向量搜索 + BM25）的基础性先决条件，将启用带正确索引和排序的高效关键词搜索。

## 变更内容

- 在 `packages/core/Cargo.toml` 中添加 `tantivy` 依赖
- 创建新的 `packages/core/src/search/` 模块（通过 `lib.rs` 导出）
- 实现 Tantivy 索引初始化，支持内存和文件系统两种存储模式
- 定义文档 Schema，包含 TEXT 字段：`title`、`summary`、`content`、`keywords`（均支持分词）
- 添加单元测试以验证索引创建和 Schema 正确性
- 使用临时目录进行测试隔离（通过 `tempfile` crate）

**范围限制（架构红线）：**
- **禁止**修改现有的 `storage/mod.rs` - 这是一个新的并行能力
- **禁止**实现 CLI 级别的查询命令 - 这仅是基础设施
- **禁止**集成到现有检索流程 - 那是未来的变更
- **仅**实现底层的 Index 结构体和 Schema 定义

## 影响范围

- **影响的规范**：`core-engine`（新增需求）
- **影响的代码**：
  - `packages/core/Cargo.toml` - 添加 tantivy 依赖
  - `packages/core/src/lib.rs` - 导出新的 search 模块
  - `packages/core/src/search/mod.rs` - 新文件，包含索引初始化
  - `packages/core/src/search/schema.rs` - 新文件，包含 Schema 定义（或合并到 mod.rs）

- **性能**：为未来快速 BM25 评分打基础（目标：Scout 延迟 < 20ms）
- **测试**：单元测试必须验证在临时目录中创建索引
- **兼容性**：纯新增，对现有 API 无破坏性变更
