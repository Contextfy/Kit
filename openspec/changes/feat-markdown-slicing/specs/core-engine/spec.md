## ADDED Requirements

### Requirement: Semantic Chunking
The core engine SHALL split markdown documents into chunks using H2 headers as semantic boundaries. 核心引擎 SHALL 使用 H2 标题作为语义边界将 markdown 文档分割为块。

#### Scenario: 使用 H2 标题分割文档
- **当**用户解析包含多个 H2 标题的 markdown 文档时
- **则**系统将文档分割为多个语义块，每个块包含：
  - `id`：块的唯一 UUID
  - `parent_id`：父文档的 UUID（整个文档的 ID）
  - `title`：H2 标题文本作为块标题
  - `summary`：块内容的前 200 个字符
  - `content`：从该 H2 标题到下一个 H2 标题（或文档结尾）的完整内容
  - `position`：块在文档中的顺序索引（从 0 开始）

#### Scenario: 处理没有 H2 标题的文档
- **当**用户解析不包含任何 H2 标题的 markdown 文档时
- **则**系统将整个文档作为单个块处理，`parent_id` 指向自身

#### Scenario: 保留文档级别元数据
- **当**文档被分割为多个块时
- **则**系统创建父文档记录，包含：
  - `id`：父文档的唯一 UUID
  - `title`：文档的 H1 标题或文件名
  - `summary`：所有块摘要的拼接（最多 500 个字符）
  - `chunk_count`：子块的数量
  - `is_parent`：设置为 `true`

## MODIFIED Requirements

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
The core engine SHALL store parsed documents and their semantic chunks in LanceDB with parent-child relationships for hierarchical retrieval. 核心引擎 SHALL 使用父子关系将解析的文档和语义块存储在 LanceDB 中以进行分层检索。

#### Scenario: 存储分块的文档
- **当**用户将包含多个块的 `ParsedDoc` 添加到知识存储时
- **则**系统存储：
  - 父文档记录，包含 `id`、`title`、`summary`、`chunk_count`、`is_parent=true`
  - 每个子块记录，包含 `id`、`parent_id`、`title`、`summary`、`content`、`position`、`is_parent=false`

#### Scenario: 创建新的知识存储
- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在指定目录中创建或打开 LanceDB 实例并准备接受文档

#### Scenario: 搜索存储的文档和块
- **当**用户使用查询字符串搜索文档时
- **则**系统在 `title` 和 `summary` 字段上执行文本匹配，同时搜索父文档和子块，并返回按相关性排序的匹配结果

### Requirement: Two-Stage Retrieval
The core engine SHALL provide scout and inspect operations for efficient context retrieval with support for chunk-level results and parent document context. 核心引擎 SHALL 提供 scout 和 inspect 操作以进行高效的上下文检索，支持块级结果和父文档上下文。

#### Scenario: 侦察相关的文档和块
- **当**用户使用搜索字符串调用 `scout(query)` 时
- **则**系统返回 `Brief` 结构体列表，可能包含：
  - 父文档条目：包含 `id`、`title`、`summary`、`chunk_count`、`is_parent=true`
  - 子块条目：包含 `id`、`parent_id`、`title`、`summary`、`position`、`is_parent=false`

#### Scenario: 通过 ID 检视块内容
- **当**用户使用 scout 结果中的子块 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回：
  - 该块的完整 `content` 字段
  - 父文档的元数据（`parent_id`、父文档 `title`）

#### Scenario: 通过 ID 检视父文档
- **当**用户使用父文档 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回所有子块的列表，每个子块包含完整的 `content` 字段

#### Scenario: 处理无效文档 ID
- **当**用户使用不存在的 UUID 调用 `inspect(id)` 时
- **则**系统返回错误，指示未找到文档
