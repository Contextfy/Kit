## MODIFIED Requirements

### Requirement: Two-Stage Retrieval
The core engine SHALL provide scout and inspect operations for efficient context retrieval with support for chunk-level results and parent document context. 核心引擎 SHALL 提供 scout 和 inspect 操作以进行高效的上下文检索，支持块级结果和父文档上下文。

#### Scenario: 侦察相关的文档和块
- **当**用户使用搜索字符串调用 `scout(query)` 时
- **则**系统返回 `Brief` 结构体列表，每个包含：
  - `id`：记录的唯一标识符
  - `title`：记录标题（切片标题或文档标题）
  - `parent_doc_title`：父文档的标题
  - `summary`：内容摘要（前 200 个字符）
  - `score`：BM25 相关性分数（f32）

#### Scenario: 通过 ID 检视文档内容
- **当**用户使用 scout 结果中的 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回：
  - 该文档的完整 `content` 字段
  - 该文档的 `title` 和 `id`
- **如果**文档存在，返回 `Some(Details)`
- **如果**文档不存在，返回 `None`

#### Scenario: CLI scout 命令显示父文档信息和分数
- **当**用户执行 CLI scout 命令时
- **则**系统显示搜索结果，格式为：
  - `Score: {score:.2} | [parent_doc] section_title` - 对于切片文档
  - `Score: {score:.2} | document_title` - 对于非切片文档
  - `ID: {id}`
  - `Summary: {summary}`
