## ADDED Requirements

### Requirement: Markdown Parsing
The core engine SHALL parse markdown files and extract structured information for indexing. 核心引擎应解析 markdown 文件并提取用于索引的结构化信息。

#### Scenario: 解析有效的 markdown 文件
- **当**用户提供有效 markdown 文件的路径时
- **则**系统解析文件并返回包含以下内容的 `ParsedDoc` 结构体：
  - `path`：原始文件路径字符串
  - `title`：文档中找到的第一个 H1 标题
  - `summary`：文档内容的前 200 个字符
  - `content`：完整的 markdown 内容字符串

#### Scenario: 解析没有 H1 标题的 markdown
- **当**用户提供没有 H1 标题的 markdown 文件时
- **则**系统使用文件名（不含扩展名）作为标题

#### Scenario: 优雅地处理解析错误
- **当**提供的文件路径不存在或不可读时
- **则**系统返回描述性错误，指示具体失败原因

### Requirement: Knowledge Storage
The core engine SHALL store parsed documents in LanceDB with a simple schema for retrieval. 核心引擎应使用简单的 schema 将解析的文档存储在 LanceDB 中以便检索。

#### Scenario: 存储解析的文档
- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统为记录生成唯一 UUID 并将其存储在 LanceDB 中，包含字段：`id`、`title`、`summary`、`content`

#### Scenario: 创建新的知识存储
- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在指定目录中创建或打开 LanceDB 实例并准备接受文档

#### Scenario: 搜索存储的文档
- **当**用户使用查询字符串搜索文档时
- **则**系统在 `title` 和 `summary` 字段上执行简单文本匹配，并返回按相关性排序的匹配文档

### Requirement: Two-Stage Retrieval
The core engine SHALL provide scout and inspect operations for efficient context retrieval. 核心引擎应提供 scout 和 inspect 操作以进行高效的上下文检索。

#### Scenario: 侦察相关文档
- **当**用户使用搜索字符串调用 `scout(query)` 时
- **则**系统返回 `Brief` 结构体列表，仅包含顶级匹配文档的 `id`、`title` 和 `summary`

#### Scenario: 通过 ID 检视文档
- **当**用户使用 scout 结果中的文档 UUID 调用 `inspect(id)` 时
- **则**系统检索并返回包含完整 `content` 字段的完整 `Details` 结构体

#### Scenario: 处理无效文档 ID
- **当**用户使用不存在的 UUID 调用 `inspect(id)` 时
- **则**系统返回错误，指示未找到文档

### Requirement: Incremental Build Support
The core engine SHALL track file hashes to skip unchanged documents during rebuild. 核心引擎应跟踪文件哈希，以便在重建时跳过未更改的文档。

#### Scenario: 检测未更改的文件
- **当**用户重建知识库且文件未被修改时
- **则**系统比较当前文件哈希与存储的哈希，并跳过重新处理文件

#### Scenario: 处理已更改的文件
- **当**用户重建知识库且文件已被修改时
- **则**系统解析、更新并存储文档的新版本
