## ADDED Requirements

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

## MODIFIED Requirements

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
