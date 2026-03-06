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

#### Scenario: 添加文档时提取关键词并同步更新索引
- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统解析文档内容并从代码块中提取关键词
- **并且**将提取的关键词存储到 `KnowledgeRecord.keywords` 字段
- **并且在写入 JSON 文件后，同步添加文档到 Tantivy 索引
- **并且**为每个切片调用 `Indexer::add_doc(record)` 时包含 keywords 字段
- **并且**在所有切片添加后调用 `Indexer::commit()` 提交索引
- **如果**索引操作失败，记录警告但不影响 JSON 存储
- **并且**返回所有切片的 UUID 列表

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

### Requirement: BM25 搜索查询
The core engine SHALL provide a Searcher for executing BM25 full-text search queries with relevance scoring and result ordering, AND apply high weight boost to the keywords field for exact API name matching. 核心引擎 SHALL 提供 Searcher 用于执行 BM25 全文搜索查询，具有相关性评分和结果排序，并为关键词字段应用高权重提升以实现精确 API 名称匹配。

#### Scenario: 执行基本搜索查询（包含关键词字段）
- **当**用户调用 `Searcher::search(query, limit)` 时
- **则**系统使用 `QueryParser` 解析查询字符串
- **并且**在 `title`、`summary`、`content`、`keywords` 四个字段中执行搜索
- **并且**`keywords` 字段具有 5.0 - 10.0 的权重提升
- **并且**使用 Jieba 分词器处理中文分词
- **并且**返回最多 `limit` 个匹配结果
