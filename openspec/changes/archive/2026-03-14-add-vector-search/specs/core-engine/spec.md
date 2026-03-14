# core-engine Specification Deltas

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements

None
