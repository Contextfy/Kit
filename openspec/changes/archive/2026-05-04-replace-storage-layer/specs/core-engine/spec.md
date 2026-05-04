## ADDED Requirements

### Requirement: Knowledge Storage (Hybrid Architecture)

The core engine SHALL use **LanceDB for vector storage** AND **Tantivy for BM25 full-text search** with a **hybrid retrieval orchestrator** that combines results using Reciprocal Rank Fusion (RRF), AND provide a high-level SearchEngine facade for simple access to all storage operations. 核心引擎 SHALL 使用 **LanceDB 进行向量存储** 并且 **Tantivy 进行 BM25 全文搜索**，配备 **混合检索编排器** 使用倒数排名融合（RRF）合并结果，并提供高级 SearchEngine 门面以简化所有存储操作的访问。

#### Scenario: 初始化搜索引擎时创建 LanceDB 和 Tantivy 后端

- **当**用户调用 `SearchEngine::new(index_dir, lancedb_uri, table_name)` 时
- **则**系统连接到 LanceDB 数据库（`lancedb_uri`）
- **并且**系统打开或创建指定的表（`table_name`）
- **并且**系统创建或打开 Tantivy BM25 索引（`index_dir`，或内存模式）
- **并且**系统初始化共享的 `EmbeddingModel` 单例（首次调用时下载 BGE-small-en 模型）
- **并且**系统创建 `HybridOrchestrator` 并注入两个存储后端
- **并且**系统返回配置好的 `SearchEngine` 实例
- **如果**任何后端初始化失败，返回描述性错误

#### Scenario: 搜索时执行混合检索（BM25 + 向量）

- **当**用户调用 `SearchEngine::search(query_text, limit)` 时
- **则**系统创建 `Query` 对象（包含 query_text 和 limit）
- **并且**系统调用 `HybridOrchestrator::search()`
- **并且**编排器**并行执行**两个搜索：
  - 向量搜索：`LanceDbStore::search()` - 使用 LanceDB 向量索引
  - BM25 搜索：`TantivyBm25Store::search()` - 使用 Tantivy 倒排索引
- **并且**系统使用 RRF 算法合并两个结果集：
  - 公式：`rrf_score(d) = Σ 1 / (k + rank_d)`，其中 k=60
  - 对每个文档，合并来自两个来源的排名分数
- **并且**系统返回按 RRF 分数排序的 `Vec<Hit>`
- **如果**一个搜索失败，系统记录警告并返回另一个成功的结果（优雅降级）
- **如果**两个搜索都失败，系统返回错误

#### Scenario: 添加文档时同时写入两个存储

- **当**用户调用 `SearchEngine::add(id, title, summary, content, keywords)` 时
- **则**系统调用 `HybridOrchestrator::add()`
- **并且**编排器**并行添加**到两个存储：
  - 向量存储：生成 Embedding 向量（384 维），使用 LanceDB 存储向量 + 元数据
  - BM25 存储：使用 Tantivy 索引 title, summary, content, keywords 字段
- **并且**如果两个添加都成功，返回 `Ok(())`
- **如果**一个添加失败，系统**自动回滚**另一个存储中的文档（防止孤记录）
- **并且**系统返回遇到的第一个错误

#### Scenario: 删除文档时从两个存储移除

- **当**用户调用 `SearchEngine::delete(id)` 时
- **则**系统调用 `HybridOrchestrator::delete()`
- **并且**系统**并行删除**从两个存储
- **并且**系统返回 `DeleteResult`，包含：
  - `vector_deleted: Result<bool, AppError>` - 向量存储删除结果
  - `bm25_deleted: Result<bool, AppError>` - BM25 存储删除结果
- **并且**用户可以检查 `any_success()` 或 `both_success()` 来确认删除状态
- **并且**系统记录任何失败的删除操作

#### Scenario: 获取文档详情时从 BM25 存储读取

- **当**用户调用 `SearchEngine::get_document(id)` 时
- **则**系统调用 `TantivyBm25Store::get_by_id(id)`
- **并且**系统返回 `Option<DocumentDetails>`，包含：
  - `id`: 文档 ID
  - `title`: 文档标题
  - `summary`: 文档摘要
  - `content`: 完整内容
- **如果**文档不存在，返回 `Ok(None)`
- **如果**存储操作失败，返回错误

#### Scenario: 批量获取文档时优化查询性能

- **当**用户调用 `SearchEngine::get_documents(ids)` 时
- **则**系统调用 `TantivyBm25Store::get_by_ids(ids)`
- **并且**系统**批量查询**所有 ID
- **并且**系统返回 `Vec<Option<DocumentDetails>>`，顺序与输入 IDs 相同
- **并且**对于不存在的文档，对应位置为 `None`
- **如果**查询失败，返回错误

### Requirement: LanceDB Vector Storage

The core engine SHALL use LanceDB as the vector storage backend for semantic search. 核心引擎 SHALL 使用 LanceDB 作为向量存储后端以支持语义搜索。

#### Scenario: 连接到 LanceDB 数据库

- **当**系统调用 `connect(lancedb_uri)` 时
- **则**系统连接到指定的 LanceDB 数据库
- **如果**数据库不存在，系统自动创建
- **如果**连接失败，返回错误

#### Scenario: 创建或打开 LanceDB 表

- **当**系统调用 `create_table_if_not_exists(conn, table_name)` 时
- **则**系统检查表是否存在
- **如果**表存在，系统打开现有表
- **如果**表不存在，系统创建新表并使用 `KnowledgeRecord` schema
- **并且**schema 包含以下字段：
  - `id`: Utf8 (非空)
  - `title`: Utf8 (非空)
  - `summary`: Utf8 (非空)
  - `content`: Utf8 (非空)
  - `vector`: FixedSizeList(Float32, 384) (非空)
  - `keywords`: Utf8 (可空)
  - `source_path`: Utf8 (非空)
- **如果**表创建失败，返回错误

#### Scenario: 向量搜索返回带分数的结果

- **当**系统执行向量搜索时
- **则**系统返回 `Vec<Hit>`，每个包含：
  - `id`: 文档 ID
  - `score`: 相关性分数（L2 距离归一化到 [0.0, 1.0]）
- **并且**结果按分数降序排列（最相关在前）
- **如果**没有结果，返回空 Vec

### Requirement: Tantivy BM25 Storage

The core engine SHALL use Tantivy as the BM25 full-text search backend. 核心引擎 SHALL 使用 Tantivy 作为 BM25 全文搜索后端。

#### Scenario: Tantivy 索引支持四个可搜索字段

- **当**系统初始化 Tantivy 索引时
- **则**Schema 包含以下 TEXT 字段：
  - `title`: 文档标题（支持中文分词）
  - `summary`: 文档摘要（支持中文分词）
  - `content`: 文档内容（支持中文分词）
  - `keywords`: 文档关键词（支持中文分词，提升权重 5.0-10.0）

#### Scenario: BM25 搜索返回完整文档详情

- **当**系统执行 BM25 搜索时
- **则**系统返回 `Vec<Bm25Result>`，每个包含：
  - `id`: 文档 ID
  - `title`: 文档标题
  - `summary`: 文档摘要
  - `score`: BM25 相关性分数
- **并且**系统可以通过 `get_by_id()` 获取完整内容

## REMOVED Requirements

### Requirement: Knowledge Storage

**Reason**: JSON 存储已被 LanceDB + Tantivy 架构取代。旧数据通过 Issue #20 的迁移工具已迁移到 LanceDB。

**Migration**: 使用 `openspec/changes/archive/2026-05-04-migrate-json-to-lancedb` 中的迁移工具将旧 JSON 数据迁移到 LanceDB。

- 旧的基于文件系统的 JSON 存储已被移除
- 旧的 `cache.json` 文件不再使用
- 所有存储操作现在通过 `SearchEngine` -> `HybridOrchestrator` -> LanceDB/Tantivy 链路
