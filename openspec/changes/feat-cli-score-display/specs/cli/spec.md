## MODIFIED Requirements

### Requirement: CLI scout 命令显示父文档信息
CLI SHALL display search results with parent document context and BM25 relevance scores. CLI SHALL 显示带有父文档上下文和 BM25 相关性分数的搜索结果。

#### Scenario: 显示带分数的搜索结果
- **当**用户执行 CLI scout 命令时
- **则**系统显示搜索结果，格式为：
  - `Score: {score:.2} | [parent_doc] section_title` - 对于切片文档
  - `Score: {score:.2} | document_title` - 对于非切片文档
  - `ID: {id}`
  - `Summary: {summary}`
- **并且**分数显示保留 2 位小数
