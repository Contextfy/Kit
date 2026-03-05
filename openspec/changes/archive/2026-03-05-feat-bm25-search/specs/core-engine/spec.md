## ADDED Requirements

### Requirement: BM25 全文搜索索引
The core engine SHALL provide an Indexer for adding documents to the Tantivy full-text search index with proper commit handling and error management. 核心引擎 SHALL 提供 Indexer 用于将文档添加到 Tantivy 全文搜索索引，具有正确的 commit 处理和错误管理。

#### Scenario: 将 KnowledgeRecord 添加到索引
- **当**系统调用 `Indexer::add_doc(record)` 添加知识记录时
- **则**系统将 `KnowledgeRecord` 转换为 Tantivy `Document`
- **并且**将以下字段映射到 Schema 字段：
  - `record.title` → `title` 字段
  - `record.summary` → `summary` 字段
  - `record.content` → `content` 字段
  - `record.keywords` → `keywords` 字段（如果存在）
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
The core engine SHALL provide a Searcher for executing BM25 full-text search queries with relevance scoring and result ordering. 核心引擎 SHALL 提供 Searcher 用于执行 BM25 全文搜索查询，具有相关性评分和结果排序。

#### Scenario: 执行基本搜索查询
- **当**用户调用 `Searcher::search(query, limit)` 时
- **则**系统使用 `QueryParser` 解析查询字符串
- **并且**在 `title`、`summary`、`content` 三个字段中执行搜索
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

## MODIFIED Requirements

### Requirement: Knowledge Storage
The core engine SHALL store parsed documents and their semantic chunks in a file-system-based JSON storage with atomic write guarantees, support for slice-level storage, AND integrate Tantivy full-text search index for BM25 scoring. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证、支持切片级别存储，并集成 Tantivy 全文搜索索引以进行 BM25 评分。

#### Scenario: 搜索存储的切片（BM25 搜索替换朴素匹配）
- **当**用户使用查询字符串搜索文档时
- **则**系统优先使用 Tantivy `Searcher` 执行 BM25 全文搜索
- **并且**返回结果包含 BM25 相关性分数
- **并且**结果按 BM25 分数降序排列（替代原有的分词匹配分数）
- **如果** Tantivy 索引不可用，回退到原有的基于分词的匹配逻辑
- **并且**返回匹配的切片记录列表（按相关性排序）

#### Scenario: 添加文档时同步更新索引
- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统在写入 JSON 文件后，同步添加文档到 Tantivy 索引
- **并且**为每个切片调用 `Indexer::add_doc(record)`
- **并且**在所有切片添加后调用 `Indexer::commit()` 提交索引
- **如果**索引操作失败，记录警告但不影响 JSON 存储
- **并且**返回所有切片的 UUID 列表

#### Scenario: 初始化知识存储时创建索引
- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在 `{data_dir}/.tantivy` 目录创建或打开 Tantivy 索引
- **并且**初始化 `Indexer` 和 `Searcher` 实例
- **如果**索引初始化失败（如权限问题），记录警告并继续运行
- **并且**系统自动清理上次崩溃遗留的 `.temp-*` 临时目录
