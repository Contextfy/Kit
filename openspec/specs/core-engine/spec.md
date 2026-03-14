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

The core engine SHALL store parsed documents and their semantic chunks in a file-system-based JSON storage with atomic write guarantees, support for slice-level storage, AND integrate Tantivy full-text search index for BM25 scoring, AND extract keywords from code blocks for enhanced API search relevance, AND generate and persist embeddings for slices with content or fallback records without slices using batch processing with graceful degradation, **AND maintain an in-memory cache of all records for high-performance lookups**. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证、支持切片级别存储，并集成 Tantivy 全文搜索索引以进行 BM25 评分，并从代码块提取关键词以增强 API 搜索相关性，并使用批处理为有内容的切片或无切片的 Fallback 记录生成和持久化向量，并支持优雅降级，**并维护所有记录的内存缓存以实现高性能查询**。

#### Scenario: 初始化知识存储时创建索引并加载缓存

- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在 `{data_dir}/.tantivy` 目录创建或打开 Tantivy 索引
- **并且**系统初始化 `Indexer` 和 `Searcher` 实例
- **并且**系统初始化或接收共享的 `EmbeddingModel` 实例
- **并且**系统**在内存中全量加载所有已存储的 `KnowledgeRecord` 到 `self.records` HashMap**
- **如果**索引初始化失败（如权限问题），记录警告并继续运行
- **如果**向量模型初始化失败，记录警告并继续运行（向量字段将为 `None`）
- **并且**系统自动清理上次崩溃遗留的 `.temp-*` 临时目录

#### Scenario: 添加文档时提取关键词并同步更新索引和缓存

- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统解析文档内容并从代码块中提取关键词
- **并且**系统将提取的关键词存储到 `KnowledgeRecord.keywords` 字段
- **并且在写入 JSON 文件后，同步添加文档到 Tantivy 索引**
- **并且**系统为每个切片调用 `Indexer::add_doc(record)` 时包含 keywords 字段
- **并且**系统在所有切片添加后调用 `Indexer::commit()` 提交索引
- **并且**系统收集所有切片的 `title` 和 `summary` 拼接文本**
- **并且**系统调用 `embed_batch()` 一次性生成所有切片的向量**
- **并且**系统将向量赋值给对应 `KnowledgeRecord` 的 `embedding` 字段**
- **并且**系统**在完成磁盘写入和索引更新后，将所有新生成的 `KnowledgeRecord` 插入 `self.records` 内存缓存**
- **如果**索引操作失败，记录警告但不影响 JSON 存储
- **如果**向量模型未注入，所有切片的 `embedding` 字段为 `None`（优雅降级，文档仍可通过 BM25 检索）
- **如果**向量生成失败，记录警告并将 `embedding` 设为 `None`（优雅降级，文档仍可通过 BM25 检索）
- **如果**向量数量与切片数量不匹配，打印警告并将所有切片的 `embedding` 设为 `None`（优雅降级，防止越界 panic）
- **并且**系统返回所有切片的 UUID 列表

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

### Requirement: BM25 搜索效果评估

The core engine SHALL provide an automated evaluation harness to quantify BM25 search effectiveness compared to naive text matching (M1), with reproducible test reports and quality gates. 核心引擎 SHALL 提供自动化评估脚手架以量化 BM25 搜索相比朴素文本匹配（M1）的效果，具有可复现的测试报告和质量门禁。

#### Scenario: 运行 A/B 搜索效果评估测试

- **当**开发者运行集成测试 `cargo test --test evaluation_test` 时
- **则**系统加载 18 篇硬编码的模拟 Minecraft 模组开发文档
- **并且**系统对预定义的 10 个查询同时执行 M1 朴素匹配和 BM25 搜索
- **并且**系统对比两者的 Top-3 结果与人工标注的 Ground Truth
- **并且**系统计算 Accuracy@3、NDCG@3、Hit Rate@3 三项指标
- **并且**系统在 `docs/` 目录生成 `BM25_EVALUATION_REPORT.md` 报告文件
- **并且**系统断言 BM25 的 Top-3 准确率必须 > 70%（质量门禁）

#### Scenario: M1 朴素匹配搜索实现（基线对比）

- **当**评估脚手架调用 `naive_match_search(query, documents)` 时
- **则**系统使用空格分词将查询拆分为多个 tokens
- **并且**系统检查每个 token 是否出现在文档的 `title` 字段（权重 2.0）或 `summary` 字段（权重 1.0）
- **并且**系统如果 title 包含所有查询 tokens，给予额外奖励分（+3.0）
- **并且**系统如果 title 包含至少一半查询 tokens，给予部分奖励分（+1.0）
- **并且**系统将分数归一化到 BM25 量级（乘以 10.0 系数）
- **并且**系统返回按分数降序排列的 `(document_id, score)` 元组列表

#### Scenario: BM25 搜索集成（测试环境）

- **当**评估脚手架初始化测试索引时
- **则**系统创建内存中的 Tantivy 索引（避免磁盘 I/O）
- **并且**系统使用现有 `Indexer` 将所有模拟文档添加到索引
- **并且**系统调用 `Indexer::commit()` 确保文档可搜索
- **当**评估脚手架调用 `bm25_search(query, limit)` 时
- **则**系统使用现有 `Searcher::search()` 执行 BM25 查询
- **并且**系统返回按 BM25 分数降序排列的 `(document_id, score)` 元组列表

#### Scenario: 评估指标计算

- **当**评估脚手架计算单个查询的 `accuracy_at_k(results, ground_truth, k)` 时
- **则**系统检查 Top-K 结果中是否有任何 Ground Truth 文档
- **并且**返回布尔值：有命中则为 1.0，否则为 0.0
- **当**评估脚手架计算 `ndcg_at_k(results, ground_truth, k)` 时
- **则**系统使用标准 NDCG 公式：`DCG / IDCG`
- **其中** `DCG = sum(relevance_i / log2(position + 1))`（relevance_i 为 1 if 在 ground truth 中 else 0，position 从 1 开始）
- **并且**`IDCG` 假设理想排序下所有相关文档排在最前面
- **并且**返回归一化的 NDCG 分数（0.0 到 1.0）
- **当**评估脚手架计算 `hit_rate_at_k(results, ground_truth, k)` 时
- **则**系统返回 Top-K 中是否有任何 Ground Truth 文档的布尔值（1.0 或 0.0）

#### Scenario: 测试报告生成格式

- **当**评估脚手架生成 `docs/BM25_EVALUATION_REPORT.md` 时
- **则**报告必须包含以下章节：
  - **摘要部分**：BM25 vs M1 的整体对比（Accuracy@3、NDCG@3、Hit Rate@3）
  - **详细对比表**：每个查询的 Top-3 结果对比（M1 结果、BM25 结果、Ground Truth）
  - **指标分析**：BM25 相比 M1 的改进百分比（如 "BM25 Accuracy 比 M1 提升 45%"）
  - **失败案例分析**：列出 BM25 表现不如 M1 的查询（如有）
- **并且**报告使用 Markdown 表格格式，方便在 GitHub 上渲染
- **并且**报告包含测试运行时间戳和文档数量信息

#### Scenario: 质量门禁断言

- **当**评估测试完成所有查询和指标计算后
- **则**系统断言 `bm25_accuracy >= m1_accuracy - 0.05`（BM25 Top-3 准确率应在 M1 基线的 5% 容差内）
- **并且**系统断言 `bm25_accuracy >= 0.70`（BM25 Top-3 准确率必须达到 70%）
- **如果**任一断言失败，测试 panic 并显示详细指标对比
- **并且**系统无论如何都会生成报告文件供人工审查

### Requirement: 文本向量化

The core engine SHALL provide a text embedding module that converts text into 384-dimensional float vectors using the BGE-small-en-v1.5 model via FastEmbed. 核心引擎 SHALL 提供文本向量化模块，使用 FastEmbed 的 BGE-small-en-v1.5 模型将文本转换为 384 维浮点向量。

#### Scenario: 初始化嵌入模型

- **当**系统调用 `EmbeddingModel::new()` 时
- **则**系统加载 BGE-small-en-v1.5 模型
- **并且**返回 `Result<EmbeddingModel>` 表示初始化成功或失败
- **如果**模型加载失败，返回描述性错误信息

#### Scenario: 将文本转换为向量

- **当**系统调用 `embed_text(text)` 方法时
- **则**系统将输入文本传递给 FastEmbed 模型
- **并且**返回 `Result<Vec<f32>>` 包含 384 维向量
- **并且**向量长度严格等于 384
- **如果**嵌入生成失败，返回 `Err(anyhow::Error)`

#### Scenario: 批量将文本转换为向量

- **当**系统调用 `embed_batch(texts)` 方法时
- **则**系统将输入文本列表传递给 FastEmbed 模型的批处理接口
- **并且**返回 `Result<Vec<Vec<f32>>>` 包含所有 384 维向量
- **并且**返回向量数量严格等于输入文本数量
- **并且**每个向量长度严格等于 384
- **如果**批量嵌入生成失败，返回 `Err(anyhow::Error)`

#### Scenario: 线程安全的模型共享

- **当** `EmbeddingModel` 被 `Arc` 包裹并在多线程环境中使用时
- **则**系统允许安全的并发调用 `embed_text` 和 `embed_batch` 方法
- **并且**模型实现 `Send + Sync` trait

#### Scenario: 向量化性能要求

- **当**系统对单条文本（少于 500 字符）执行向量化时
- **则**生成向量的时间应 < 100ms
- **并且**性能测试包含模型加载后的首次调用（冷启动）和后续调用（热启动）

#### Scenario: 批量向量化性能要求

- **当**系统对 N 条文本执行批量向量化时
- **则**总耗时应显著低于 N 次单独调用的总和
- **并且**利用 FastEmbed 的批处理优化

#### Scenario: 相同文本产生相同向量

- **当**系统对相同文本内容多次调用 `embed_text` 时
- **则**系统返回相同的向量（浮点数误差除外）
- **并且**向量维度保持一致

### Requirement: FastEmbed 依赖集成

The core engine SHALL include the fastembed crate as a dependency in packages/core/Cargo.toml. 核心引擎 SHALL 在 packages/core/Cargo.toml 中包含 fastembed 依赖。

#### Scenario: 依赖版本管理

- **当**开发者在 `packages/core/Cargo.toml` 中添加 fastembed 依赖时
- **则**使用最新稳定版本
- **并且**依赖版本在工作空间中保持一致

#### Scenario: 编译时验证依赖

- **当**系统执行 `cargo build -p contextfy-core` 时
- **则**fastembed 依赖被成功解析和下载
- **并且**编译成功无依赖冲突错误

### Requirement: 嵌入模块导出

The core engine SHALL export the embeddings module through lib.rs for external use. 核心引擎 SHALL 通过 lib.rs 导出 embeddings 模块供外部使用。

#### Scenario: 模块公开访问

- **当**外部代码使用 `contextfy_core::embeddings` 时
- **则**模块必须公开可访问
- **并且** `EmbeddingModel` 结构体可被实例化

#### Scenario: 模块在 lib.rs 中注册

- **当** `contextfy-core` crate 被编译时
- **则**`embeddings` 模块包含在 `lib.rs` 的 `pub mod` 声明中
- **并且**模块的公共 API 可被依赖该 crate 的代码使用

### Requirement: 向量持久化存储

The core engine SHALL persist document embeddings in the JSON-based storage to enable semantic similarity computation without re-embedding. 核心引擎 SHALL 在基于 JSON 的存储中持久化文档向量，以实现语义相似度计算而无需重新嵌入。

#### Scenario: 存储带向量的知识记录

- **当**系统将 `KnowledgeRecord` 写入 JSON 文件时
- **则**记录包含 `embedding` 字段，类型为 `Option<Vec<f32>>`
- **并且**如果向量存在，JSON 中包含完整的浮点数数组
- **并且**如果向量为 None，字段在 JSON 中可以不存在或为 null

#### Scenario: 反序列化旧版 JSON 兼容性

- **当**系统读取不包含 `embedding` 字段的旧版 JSON 文件时
- **则**反序列化成功，`embedding` 字段默认为 `None`
- **并且**不产生错误或警告
- **并且**记录可正常使用

### Requirement: 余弦相似度计算

The core engine SHALL provide a cosine similarity function for comparing two embedding vectors with proper normalization and divide-by-zero protection. 核心引擎 SHALL 提供余弦相似度函数用于比较两个嵌入向量，具有正确的归一化和除零保护。

#### Scenario: 计算相同向量的相似度

- **当**系统对两个相同的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 1.0（完全相似）
- **并且**不产生除零错误

#### Scenario: 计算正交向量的相似度

- **当**系统对两个正交（点积为 0）的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 0.5（归一化后的中间值）

#### Scenario: 计算相反向量的相似度

- **当**系统对两个相反（`b = -a`）的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 0.0（完全不相似）

#### Scenario: 处理零向量

- **当**系统遇到至少一个输入为零向量（所有元素为 0）时
- **则**返回 0.0（除零保护）
- **并且**不产生 panic 或数值错误

#### Scenario: 归一化到 0-1 范围

- **当**系统计算余弦相似度时
- **则**结果被归一化到 [0.0, 1.0] 范围
- **并且**使用公式：`(raw_cosine + 1.0) / 2.0`
- **其中** `raw_cosine = (a · b) / (||a|| * ||b||)`

### Requirement: 批量向量生成

The core engine SHALL provide batch embedding generation to efficiently process multiple texts in a single model inference call. 核心引擎 SHALL 提供批量向量生成以高效地在单次模型推理中处理多个文本。

#### Scenario: 批量生成向量

- **当**系统调用 `embed_batch(texts)` 时
- **则**系统一次性处理所有文本
- **并且**返回 `Vec<Vec<f32>>`，长度与输入一致
- **并且**每个向量维度均为 384
- **并且**利用 FastEmbed 的原生批处理能力

#### Scenario: 处理空输入

- **当**系统调用 `embed_batch(&[])` 时
- **则**返回空 `Vec::new()`
- **并且**不产生错误

#### Scenario: 批量处理性能优势

- **当**系统批量处理 N 条文本时
- **则**总耗时显著低于 N 次单独调用 `embed_text`
- **并且**减少模型推理开销

### Requirement: 内存缓存层

The core engine SHALL maintain an in-memory cache of all knowledge records using `Arc<RwLock<HashMap<String, KnowledgeRecord>>>` to enable O(1) lookups and high-performance vector similarity scans. 核心引擎 SHALL 使用 `Arc<RwLock<HashMap<String, KnowledgeRecord>>>` 维护所有知识记录的内存缓存，以实现 O(1) 查询和高效的向量相似度扫描。

#### Scenario: 启动时全量加载文档到内存缓存

- **当**用户调用 `KnowledgeStore::new(data_dir, embedding_model)` 初始化知识库时
- **则**系统在创建数据目录后，使用 `fs::read_dir` 遍历 `data_dir`
- **并且**系统跳过临时文件（`.temp-*`）和子目录
- **并且**系统反序列化所有 `.json` 文件为 `KnowledgeRecord`
- **并且**系统将所有记录插入 `self.records` HashMap（key 为 `record.id`）
- **并且**系统记录单个文件解析失败的警告，但不中断整体加载

#### Scenario: 写入时同步更新缓存（Write-Through）

- **当**用户调用 `KnowledgeStore::add()` 添加新文档时
- **则**系统在成功写入 JSON 文件并完成 Tantivy 索引后
- **并且**系统获取 `self.records` 的写锁（`.write().await`）
- **并且**系统将新生成的 `KnowledgeRecord` 插入 HashMap（key 为 `id`）
- **并且**系统确保缓存与磁盘数据的一致性

#### Scenario: 内存缓存并发安全性

- **当**多个并发任务同时读取 `self.records` 时
- **则**系统使用 `RwLock` 允许多个读者同时持有读锁
- **当**单个任务写入 `self.records` 时
- **则**系统独占写锁，阻塞所有读者和写者
- **并且**系统使用 `Arc` 允许跨线程安全共享

### Requirement: 向量语义搜索

The core engine SHALL provide vector-based semantic search capability that computes cosine similarity between query and document embeddings to retrieve semantically relevant content. 核心引擎 SHALL 提供基于向量的语义搜索能力，通过计算查询与文档向量之间的余弦相似度来检索语义相关的文档。

#### Scenario: 执行向量语义搜索（内存扫描）

- **当**用户调用 `KnowledgeStore::vector_search(query, limit)` 时
- **则**系统将查询文本转换为 384 维向量
- **并且**系统获取 `self.records` 的读锁（`.read().await`），在内存中遍历所有文档
- **并且**系统计算查询向量与每个文档向量的余弦相似度
- **并且**系统过滤掉 `embedding` 字段为 `None` 的文档
- **并且**系统按相似度降序排序结果
- **并且**系统返回前 `limit` 个 `(KnowledgeRecord, similarity_score)` 元组
- **并且**系统**不使用** `fs::read_dir` 遍历磁盘（纯内存操作）

#### Scenario: 向量模型未初始化时返回明确错误

- **当**用户调用 `vector_search(query, limit)` 但 `embedding_model` 为 `None` 时（且查询词不为空且 limit > 0）
- **则**系统返回 `Err(anyhow::Error)` 包含错误信息 "Embedding model not initialized"
- **并且**系统不执行任何搜索操作

#### Scenario: 向量搜索边界条件提前返回

- **当**用户调用 `vector_search(query, limit)` 且查询词为空或 limit 为 0 时
- **则**系统直接返回空结果 `Ok(Vec::new())`
- **并且**系统不检查 `embedding_model` 是否初始化
- **并且**系统不执行任何搜索操作

#### Scenario: 异步运行时安全的向量化调用

- **当**系统执行查询向量化时
- **则**系统使用 `tokio::task::spawn_blocking` 包裹 `embed_text` 调用
- **并且**系统避免阻塞 Tokio 异步运行时的 worker 线程
- **并且**系统等待向量生成完成后继续执行

#### Scenario: 安全的浮点数排序

- **当**系统对相似度分数进行降序排序时
- **则**系统使用 `sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.id.cmp(&b.0.id)))`
- **并且**系统避免 `f32` 未实现 `Ord` trait 导致的编译错误
- **并且**系统正确处理可能的 NaN 值（使用 `unwrap_or`）
- **并且**当相似度分数相同时，系统按记录 ID 进行次级排序以保证结果稳定

#### Scenario: 语义相似度排序验证

- **当**知识库包含以下文档：
  - 文档 A："cat and dog are pets"
  - 文档 B："car and bike are vehicles"
  - 文档 C："puppy plays in yard"
- **并且**用户查询 "dog"
- **则**系统返回的结果顺序应为：A（最高相似度）> C（"puppy" 与 "dog" 语义相似）> B（不相关）

### Requirement: 向量搜索性能要求

The core engine SHALL complete vector-based semantic search queries within 500ms for knowledge bases containing 1000 documents. 核心引擎 SHALL 在包含 1000 个文档的知识库中于 500ms 内完成基于向量的语义搜索查询。

#### Scenario: 1000 文档向量搜索性能基准（核心扫描路径）

- **当**知识库包含 1000 个带有向量的文档时
- **并且**每个文档的向量为 384 维浮点数组
- **并且**系统执行内存扫描以计算查询向量与所有文档向量的余弦相似度
- **则**核心扫描路径延迟应 < 500ms
- **并且**延迟测量包括：内存遍历 + 1000 次余弦相似度计算 + 排序
- **并且**系统使用内存扫描，**不涉及磁盘 I/O**
- **并且**性能测试通过测试专用方法 `vector_search_with_query_vector()` 跳过查询向量化阶段，仅测量核心扫描路径性能（避免 EmbeddingModel 推理时间干扰）

