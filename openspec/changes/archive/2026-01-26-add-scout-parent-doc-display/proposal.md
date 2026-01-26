# 变更：scout 命令显示父文档来源

## 为什么

scout 命令目前只显示切片文档的小节标题（H2），用户难以理解搜索结果的完整上下文。用户需要看到切片属于哪个父文档，以便更好地导航和理解内容。

## 变更内容

- 在 `KnowledgeRecord` 结构体中添加 `parent_doc_title` 字段
- 更新 `KnowledgeStore::add()` 在创建切片时存储父文档标题
- 更新 `Brief` 结构体以包含 `parent_doc_title`
- 更新 CLI scout 命令显示格式："[parent_doc] section_title"

**破坏性变更**：无 - 仅添加新字段和显示增强

## 影响范围

- 影响的规范：core-engine
- 影响的代码：
  - `packages/core/src/storage/mod.rs` - KnowledgeRecord 结构体和存储逻辑
  - `packages/core/src/retriever/mod.rs` - Brief 结构体
  - `packages/cli/src/commands/scout.rs` - 输出显示格式
