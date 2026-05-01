# cli Specification

## ADDED Requirements

### Requirement: CLI scout 命令显示分数和颜色高亮

CLI SHALL display search results with BM25 relevance scores and terminal color highlighting. CLI SHALL 显示带有 BM25 相关性分数和终端颜色高亮的搜索结果。

#### Scenario: 显示带分数的搜索结果

- **WHEN** 用户执行 CLI scout 命令
- **THEN** 系统显示搜索结果，格式为：
  - `Score: {score:.2} | [parent_doc] section_title` - 对于切片文档
  - `Score: {score:.2} | document_title` - 对于非切片文档
  - `ID: {id}`
  - `Summary: {summary}`
- **AND** 分数显示保留 2 位小数

#### Scenario: 分数颜色高亮规则

- **WHEN** 显示搜索结果时
- **THEN** 系统根据分数应用颜色高亮：
  - **绿色** (green/bold): 分数 >= 0.75（高相关性）
  - **黄色** (yellow/bold): 0.5 <= 分数 < 0.75（中等相关性）
  - **灰色暗淡** (white/dimmed): 分数 < 0.5（低相关性）
- **AND** 使用 `colored` crate 的终端颜色功能
- **AND** 颜色格式化为 `format!("Score: {:.2}", score).color()`
