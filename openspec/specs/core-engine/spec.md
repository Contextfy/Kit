# core-engine Specification

## Purpose
TBD - created by archiving change initialize-project. Update Purpose after archive.
## Requirements
### Requirement: Markdown Parsing
The core engine SHALL parse markdown files and extract structured information for indexing with support for semantic chunking. 核心引擎 SHALL 解析 markdown 文件并提取用于索引的结构化信息，并支持语义分块。

#### Scenario: 解析有效的 markdown 文件（分块模式）
- **当**用户提供有效 markdown 文件的路径并启用分块模式时
- **则**系统解析文件并返回包含以下内容的 `ParsedDoc` 结构体：
  - `id`：文档的唯一 UUID
  - `path`：原始文件路径字符串
  - `title`：文档中找到的第一个 H1 标题
  - `chunks`：按顺序排列的语义块列表，每个块包含独立的 `id`、`title`、`summary`、`content`
  - `chunk_count`：语义块的总数量

#### Scenario: 解析没有 H1 标题的 markdown（分块模式）
- **当**用户提供没有 H1 标题的 markdown 文件时
- **则**系统使用文件名（不含扩展名）作为标题

#### Scenario: 优雅地处理解析错误
- **当**提供的文件路径不存在或不可读时
- **则**系统返回描述性错误，指示具体失败原因

### Requirement: Knowledge Storage
The core engine SHALL store parsed documents and their semantic chunks in a file-system based JSON storage with atomic write guarantees and support for slice-level storage. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证并支持切片级别存储。

#### Scenario: 存储切片文档（切片模式）
- **当**用户将包含多个切片的 `ParsedDoc` 添加到知识存储时
- **则**系统为每个切片创建独立的 `KnowledgeRecord`，每个记录包含：
  - `id`：切片的唯一 UUID
  - `title`：切片的标题（H2 标题）
  - `parent_doc_title`：父文档的标题（H1 标题或文件名）
  - `summary`：切片内容的前 200 个字符
  - `content`：切片的完整内容
  - `source_path`：原始文档文件路径
- **并且**使用原子性写入（临时文件 → 原子移动模式）确保数据完整性
- **并且**返回所有切片的 UUID 列表

#### Scenario: 存储无切片文档（回退模式）
- **当**用户将不包含切片的 `ParsedDoc` 添加到知识存储时
- **则**系统将整个文档作为单条记录存储（向后兼容）
- **并且**记录包含：
  - `title`：文档的完整标题
  - `parent_doc_title`：与 `title` 相同（保持一致性）
  - `source_path` 字段用于追溯原始文件

#### Scenario: 创建新的知识存储
- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统：
  - 在指定目录中创建存储文件夹
  - 自动清理上次崩溃遗留的 `.temp-*` 临时目录
  - 准备接受文档

#### Scenario: 搜索存储的切片
- **当**用户使用查询字符串搜索文档时
- **则**系统在 `title` 和 `summary` 字段上执行文本匹配
- **并且**跳过临时文件和目录（防御性检查）
- **并且**返回匹配的切片记录列表

#### Scenario: 原子性写入失败回滚
- **当**写入过程中发生错误（磁盘满、权限问题等）
- **则**系统：
  - 删除所有已提交的文件（如果进入提交阶段）
  - 清理临时目录
  - 返回详细错误信息
- **确保**不会留下孤儿数据或不完整的文件

#### Scenario: 崩溃恢复
- **当**系统在写入过程中崩溃并重启
- **则**系统启动时自动扫描并清理所有 `.temp-*` 目录
- **确保**不会积累垃圾数据

### Requirement: Two-Stage Retrieval
The core engine SHALL provide scout and inspect operations for efficient context retrieval with support for chunk-level results and parent document context. 核心引擎 SHALL 提供 scout 和 inspect 操作以进行高效的上下文检索，支持块级结果和父文档上下文。

#### Scenario: 侦察相关的文档和块
- **当**用户使用搜索字符串调用 `scout(query)` 时
- **则**系统返回 `Brief` 结构体列表，每个包含：
  - `id`：记录的唯一标识符
  - `title`：记录标题（切片标题或文档标题）
  - `parent_doc_title`：父文档的标题
  - `summary`：内容摘要（前 200 个字符）

#### Scenario: 通过 ID 检视块内容
- **当**用户使用 scout 结果中的子块 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回：
  - 该块的完整 `content` 字段
  - 该块的标题（切片的 section_title 或文档的 title）

#### Scenario: 通过 ID 检视父文档
- **当**用户使用父文档 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回所有子块的列表，每个子块包含完整的 `content` 字段

#### Scenario: 处理无效文档 ID
- **当**用户使用不存在的 UUID 调用 `inspect(id)` 时
- **则**系统返回错误，指示未找到文档

#### Scenario: CLI scout 命令显示父文档信息
- **当**用户执行 CLI scout 命令时
- **则**系统显示搜索结果，格式为：
  - `[parent_doc] section_title` - 对于切片文档
  - `document_title` - 对于非切片文档
  - `ID: {id}`
  - `Summary: {summary}`

### Requirement: Incremental Build Support
The core engine SHALL track file hashes to skip unchanged documents during rebuild. 核心引擎应跟踪文件哈希，以便在重建时跳过未更改的文档。

#### Scenario: 检测未更改的文件
- **当**用户重建知识库且文件未被修改时
- **则**系统比较当前文件哈希与存储的哈希，并跳过重新处理文件

#### Scenario: 处理已更改的文件
- **当**用户重建知识库且文件已被修改时
- **则**系统解析、更新并存储文档的新版本

### Requirement: Semantic Chunking
The core engine SHALL split markdown documents into chunks using H2 headers as semantic boundaries. 核心引擎 SHALL 使用 H2 标题作为语义边界将 markdown 文档分割为块。

#### Scenario: 使用 H2 标题分割文档
- **When** 用户解析包含多个 H2 标题的 markdown 文档时
- **Then** 系统将文档分割为多个语义块，每个块包含：
  - `id`：块的唯一 UUID
  - `parent_id`：父文档的 UUID（整个文档的 ID）
  - `title`：H2 标题文本作为块标题
  - `summary`：基于内容结构的智能摘要（首段或代码块，最多 1000 字符）
  - `content`：从该 H2 标题到下一个 H2 标题（或文档结尾）的完整内容
  - `position`：块在文档中的顺序索引（从 0 开始）

#### Scenario: 处理没有 H2 标题的文档
- **When** 用户解析不包含任何 H2 标题的 markdown 文档时
- **Then** 系统将整个文档作为单个块处理，`parent_id` 指向自身

#### Scenario: 保留文档级别元数据
- **When** 文档被分割为多个块时
- **Then** 系统创建父文档记录，包含：
  - `id`：父文档的唯一 UUID
  - `title`：文档的 H1 标题或文件名
  - `summary`：所有块摘要的拼接（最多 500 个字符）
  - `chunk_count`：子块的数量
  - `is_parent`：设置为 `true`

### Requirement: Source Path Tracking
The system SHALL track the original file path for each stored slice record. 系统 SHALL 为每个存储的切片记录跟踪原始文件路径。

#### Scenario: 存储带源路径的切片
- **当**切片被存储到知识库时
- **则**记录包含 `source_path` 字段，存储原始文件路径
- **并且**该字段可搜索和检索

### Requirement: 智能摘要提取
The core engine SHALL extract summaries using the first semantic paragraph with special handling for code blocks, ensuring complete code signatures are preserved. 核心引擎 SHALL 使用首个语义段落提取摘要，并对代码块进行特殊处理，确保完整的代码签名被保留。

#### Scenario: 从普通 markdown 内容提取摘要
- **Given** 一个具有标准段落结构的 markdown 章节或文档
- **When** 提取摘要时
- **Then** 系统返回完整的第一个段落（直到第一个 `\n\n` 双换行符的所有文本）
- **And** 摘要保留完整的句子和代码块
- **And** 摘要去除首尾空白字符

#### Scenario: 保留以代码块开始的完整签名
- **Given** 一个以代码块开始的内容（例如函数签名）
- **When** 提取摘要时
- **Then** 系统包含**整个代码块**，从开始的 ``` 到关闭的 ```
- **And** 即使代码块内包含换行符，也不会在代码块中间截断
- **And** 摘要包含完整的函数/类签名及其返回类型
- **Example**: 输入 `"```rust\npub fn foo() -> Bar\n```\n\n说明..."` → 摘要包含完整的三行代码块

#### Scenario: 处理代码块内的双换行符
- **Given** 一个以代码块开始，且代码块内部包含双换行符的内容
- **When** 提取摘要时
- **Then** 系统不会在代码块内部的 `\n\n` 处截断
- **And** 摘要持续直到找到代码块的关闭标记 ```
- **And** 摘要包含完整的代码块内容

#### Scenario: 处理无段落分隔的内容
- **Given** 没有双换行符的 markdown 内容（例如单一长段落或代码片段）
- **When** 提取摘要时
- **Then** 系统回退到现有行为（截取前 200 字符）
- **And** 尝试在最后一个句子结束标点（`.`、`!`、`?`）处截断（如果存在）
- **And** 如果不存在句子结束标点，则在 200 字符处截断
- **And** 摘要去除首尾空白字符

#### Scenario: 处理超长段落（Wall of Text 保护）
- **Given** 一个超过 1000 字符的单一段落（例如粘贴的日志或从不换行的文本）
- **When** 提取摘要时
- **Then** 系统在 1000 字符处强制截断
- **And** 尝试在截断点附近的最后一个完整句子（`.`、`!`、`?`）处断开
- **And** 如果找不到句子结束符，直接在 1000 字符处截断并添加 `...` 后缀
- **And** 摘要不会超过 1000 字符（防止撑爆 UI 或数据库字段溢出）

#### Scenario: 处理空内容或极短内容
- **Given** 空内容或少于 50 字符的内容
- **When** 提取摘要时
- **Then** 系统按原样返回内容，不进行截断
- **And** 去除首尾空白字符
- **And** 不发生错误

