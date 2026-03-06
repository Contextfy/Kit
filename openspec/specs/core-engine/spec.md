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
The core engine SHALL store parsed documents and their semantic chunks in a file-system-based JSON storage with atomic write guarantees, support for slice-level storage, AND integrate Tantivy full-text search index for BM25 scoring, AND extract keywords from code blocks for enhanced API search relevance. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证、支持切片级别存储，并集成 Tantivy 全文搜索索引以进行 BM25 评分，并从代码块提取关键词以增强 API 搜索相关性。

#### Scenario: 搜索存储的切片（BM25 搜索替换朴素匹配）
- **当**用户使用查询字符串搜索文档时
- **则**系统优先使用 Tantivy `Searcher` 执行 BM25 全文搜索
- **并且**返回结果包含 BM25 相关性分数
- **并且**结果按 BM25 分数降序排列（替代原有的分词匹配分数）
- **如果** Tantivy 索引不可用，回退到原有的基于分词的匹配逻辑
- **并且**返回匹配的切片记录列表（按相关性排序）

#### Scenario: 添加文档时提取关键词并同步更新索引
- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统解析文档内容并从代码块中提取关键词
- **并且**将提取的关键词存储到 `KnowledgeRecord.keywords` 字段
- **并且在写入 JSON 文件后，同步添加文档到 Tantivy 索引**
- **并且**为每个切片调用 `Indexer::add_doc(record)` 时包含 keywords 字段
- **并且**在所有切片添加后调用 `Indexer::commit()` 提交索引
- **如果**索引操作失败，记录警告但不影响 JSON 存储
- **并且**返回所有切片的 UUID 列表

#### Scenario: 初始化知识存储时创建索引
- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在 `{data_dir}/.tantivy` 目录创建或打开 Tantivy 索引
- **并且**初始化 `Indexer` 和 `Searcher` 实例
- **如果**索引初始化失败（如权限问题），记录警告并继续运行
- **并且**系统自动清理上次崩溃遗留的 `.temp-*` 临时目录

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

### Requirement: 中文 Markdown 兼容性验证
核心引擎 SHALL 验证其对中文技术文档和微软复杂 Markdown 标签的解析稳定性，确保在生产环境中不会因解析器 panic 导致服务中断。

#### Scenario: 解析包含中文字符的 Markdown 文档
- **当**系统解析包含 UTF-8 编码中文字符的 Markdown 文件时
- **则**pulldown-cmark parser 应成功解析文档结构
- **并且**正确提取标题、段落、代码块等元素
- **并且**不产生解析错误或 panic

#### Scenario: 处理微软复杂标签结构
- **当**系统解析包含复杂嵌套结构的 Markdown 文档时（例如多层级的列表、表格、代码块嵌套）
- **则**parser 应优雅处理这些结构
- **并且**提取的语义块保持结构完整性
- **并且**不会因标签嵌套深度导致栈溢出或性能问题

#### Scenario: 构建基岩版文档知识库
- **当**系统批量处理 26 篇 Minecraft Bedrock Script API 文档时
- **则**所有文档应成功解析并存储到知识库
- **并且**`contextfy build` 命令完成时显示成功处理的文档数量
- **并且**生成的切片可通过 `contextfy scout` 正常检索

### Requirement: Tantivy Index Schema Definition
The core engine SHALL define a Tantivy Schema for full-text search indexing with TEXT field configuration supporting tokenization for Chinese and English content. 核心引擎 SHALL 为全文搜索索引定义 Tantivy Schema，使用 TEXT 字段配置支持中英文内容的分词。

#### Scenario: 定义包含四个 TEXT 字段的 Schema
- **当**系统初始化 Tantivy Schema 时
- **则**Schema 必须包含以下字段，均配置为 TEXT 类型（支持分词）：
  - `title` - 用于索引文档标题
  - `summary` - 用于索引文档摘要
  - `content` - 用于索引文档完整内容
  - `keywords` - 用于索引文档关键词

#### Scenario: 验证 Schema 字段配置
- **当**系统查询 Schema 定义时
- **则**所有字段必须配置为 TEXT 记录类型（TEXT）
- **并且**字段必须启用分词支持（TOKENIZED）
- **并且**字段必须支持存储（STORED）以用于结果展示

### Requirement: Tantivy Index Initialization
The core engine SHALL provide a function to create and initialize a Tantivy Index with support for both in-memory and filesystem-backed storage modes. 核心引擎 SHALL 提供函数来创建和初始化 Tantivy 索引，支持内存和文件系统两种存储模式。

#### Scenario: 在内存中创建临时索引
- **当**系统调用索引创建函数且未指定目录路径时
- **则**系统在内存中创建临时 Tantivy Index
- **并且**Index 使用预定义的 Schema（包含 title, summary, content, keywords 字段）
- **并且**Index 可用于文档索引和搜索操作
- **并且**索引数据在程序退出后自动释放

#### Scenario: 在指定目录创建持久化索引
- **当**系统调用索引创建函数并指定有效目录路径时
- **则**系统在指定目录创建持久化 Tantivy Index
- **并且**Index 使用预定义的 Schema
- **并且**索引元数据和数据写入到指定目录
- **并且**目录已存在时可重新打开现有索引

#### Scenario: 处理无效目录路径
- **当**系统尝试在不存在的父目录下创建索引且父目录无法创建时
- **则**系统返回描述性错误
- **并且**错误信息包含具体的路径和失败原因

### Requirement: Tantivy Module Structure
The core engine SHALL expose a public `search` module containing the Tantivy Index initialization and Schema definition functions. 核心引擎 SHALL 公开 `search` 模块，包含 Tantivy 索引初始化和 Schema 定义函数。

#### Scenario: 导出 search 模块
- **当**外部代码使用 `contextfy_core::search` 时
- **则**模块必须公开可访问
- **并且**模块提供创建 Index 的公共函数
- **并且**模块提供获取或定义 Schema 的方法

#### Scenario: 集成到 core crate
- **当**用户编译 `contextfy-core` crate 时
- **则**`search` 模块包含在 `lib.rs` 的导出列表中
- **并且**外部依赖可通过 `use contextfy_core::search` 引入模块

### Requirement: BM25 全文搜索索引
The core engine SHALL provide an Indexer for adding documents to the Tantivy full-text search index with proper commit handling and error management, AND populate the keywords field with extracted code block identifiers. 核心引擎 SHALL 提供 Indexer 用于将文档添加到 Tantivy 全文搜索索引，具有正确的 commit 处理和错误管理，并使用提取的代码块标识符填充关键词字段。

#### Scenario: 将 KnowledgeRecord 添加到索引（包含关键词）
- **当**系统调用 `Indexer::add_doc(record)` 添加知识记录时
- **则**系统将 `KnowledgeRecord` 转换为 Tantivy `Document`
- **并且**将以下字段映射到 Schema 字段：
  - `record.title` → `title` 字段
  - `record.summary` → `summary` 字段
  - `record.content` → `content` 字段
  - `record.keywords` → `keywords` 字段（使用 Tantivy 原生多值字段插入：for 循环逐个 add_text）
- **并且**如果 `keywords` 字段为空数组，不添加任何值
- **并且**文档被添加到索引写入缓冲区
- **并且**返回 `Result<()>` 表示操作成功或失败

#### Scenario: 提交索引使文档可搜索
- **当**系统调用 `Indexer::commit()` 时
- **则**系统将所有缓冲的写入操作持久化到磁盘
- **并且**使新添加的文档立即可被搜索
- **并且**返回 `Result<()>` 表示操作成功或失败
- **如果** commit 失败，返回描述性错误信息

#### Scenario: 处理 Tantivy 错误
- **当**索引操作发生 Tantivy 错误时
- **则**系统将 `TantivyError` 转换为 `anyhow::Error`
- **并且**添加上下文信息（如文档 ID、操作类型）
- **并且**禁止使用 `unwrap()` 或 `expect()` 直接 panic

### Requirement: BM25 搜索查询
The core engine SHALL provide a Searcher for executing BM25 full-text search queries with relevance scoring and result ordering, AND apply high weight boost to the keywords field for exact API name matching. 核心引擎 SHALL 提供 Searcher 用于执行 BM25 全文搜索查询，具有相关性评分和结果排序，并为关键词字段应用高权重提升以实现精确 API 名称匹配。

#### Scenario: 执行基本搜索查询（包含关键词字段）
- **当**用户调用 `Searcher::search(query, limit)` 时
- **则**系统使用 `QueryParser` 解析查询字符串
- **并且**在 `title`、`summary`、`content`、`keywords` 四个字段中执行搜索
- **并且**`keywords` 字段具有 5.0 - 10.0 的权重提升
- **并且**使用 Jieba 分词器处理中文分词
- **并且**返回最多 `limit` 个匹配结果

#### Scenario: 返回带 BM25 分数的结果
- **当**搜索查询返回结果时
- **则**每个结果包含以下字段：
  - `id`: 记录的唯一标识符
  - `title`: 记录标题
  - `summary`: 记录摘要
  - `score`: BM25 相关性分数（f32）
- **并且**结果按 BM25 分数降序排列
- **并且**分数最高的结果排在第一位

#### Scenario: 处理空查询
- **当**用户传入空字符串或仅包含空白的查询时
- **则**系统返回空结果列表 `Vec::new()`
- **并且**不执行实际的搜索操作

#### Scenario: 处理查询解析错误
- **当**查询字符串无法被 QueryParser 解析时
- **则**系统返回 `Err(anyhow::Error)`
- **并且**错误信息包含原始查询字符串和解析失败原因

### Requirement: Tantivy 搜索性能基准
The core engine SHALL maintain search query latency under 100ms for knowledge bases containing 1000 documents. 核心引擎 SHALL 保持搜索查询延迟在包含 1000 个文档的知识库中低于 100ms。

#### Scenario: 1000 文档搜索性能
- **当**知识库包含 1000 个文档时
- **并且**用户执行搜索查询
- **则**查询延迟应 < 100ms
- **并且**延迟测量包括：查询解析 + 搜索执行 + 结果收集

### Requirement: 代码块关键词提取
The core engine SHALL extract identifiers from code blocks in Markdown documents and store them as keywords for enhanced search relevance. 核心引擎 SHALL 从 Markdown 文档的代码块中提取标识符并作为关键词存储，以增强搜索相关性。

#### Scenario: 从代码块提取函数名
- **当**系统解析包含代码块的 Markdown 文档时
- **并且**代码块包含函数定义（如 `function createItem()`, `def build_server():`, `fn process_data()`）
- **则**系统使用正则表达式提取函数名（如 `createItem`, `build_server`, `process_data`）
- **并且**将提取的函数名添加到 `KnowledgeRecord.keywords` 字段

#### Scenario: 从代码块提取类/类型名
- **当**系统解析包含代码块的 Markdown 文档时
- **并且**代码块包含类或类型定义（如 `class BlockCustomComponent`, `struct Config`, `interface User`）
- **则**系统使用正则表达式提取类/类型名（如 `BlockCustomComponent`, `Config`, `User`）
- **并且**将提取的类/类型名添加到 `KnowledgeRecord.keywords` 字段

#### Scenario: 提取 snake_case 标识符
- **当**系统解析包含代码块的 Markdown 文档时
- **并且**代码块包含 snake_case 标识符（如 `create_item`, `process_data`, `map`, `new`, `range`）
- **则**系统使用正则表达式提取这些标识符
- **并且**将提取的标识符添加到 `KnowledgeRecord.keywords` 字段
- **并且**常用 API 名称（如 `map`, `new`, `range`）不会被误杀

#### Scenario: 过滤编程语言关键字
- **当**系统提取代码块标识符时
- **则**系统过滤掉常见的编程语言关键字（如 `fn`, `let`, `const`, `if`, `else`, `return`, `class`, `def` 等）
- **并且**过滤掉长度小于 3 个字符的标识符
- **并且**对提取的关键词进行去重

#### Scenario: 使用缓存优化正则表达式性能
- **当**系统执行关键词提取时
- **则**系统使用 `std::sync::OnceLock` 或 `lazy_static` 缓存正则表达式
- **并且**避免在循环中重复编译相同的正则表达式
- **并且**确保关键词提取不会显著影响解析性能

#### Scenario: 处理不包含代码块的文档
- **当**系统解析不包含代码块的 Markdown 文档时
- **则**`KnowledgeRecord.keywords` 字段为空数组
- **并且**不产生错误或警告
- **并且**文档正常存储和索引

### Requirement: 关键词搜索权重增强
The core engine SHALL apply a high search weight boost to the keywords field in BM25 search queries to ensure exact API name matches rank at the top. 核心引擎 SHALL 在 BM25 搜索查询中为关键词字段应用高权重提升，确保精确的 API 名称匹配排在最前面。

#### Scenario: 应用关键词字段权重提升
- **当**系统初始化 `Searcher` 时
- **则**系统将 `keywords` 字段添加到 `QueryParser` 的搜索字段列表
- **并且**使用 `set_field_boost()` API 为 `keywords` 字段设置高权重（5.0 - 10.0）
- **并且**确保关键词字段的权重显著高于 `content` 字段（默认权重 1.0）

#### Scenario: 验证关键词匹配排名优先
- **当**用户搜索特定的 API 名称（如 "createItem"）时
- **并且**文档 A 在代码块中包含 `function createItem() {}`（关键词字段包含 "createItem"）
- **并且**文档 B 在普通文本中提及 "createItem"（仅在 content 字段）
- **则**文档 A 的 BM25 分数应显著高于文档 B
- **并且**文档 A 应排在搜索结果的第一位

