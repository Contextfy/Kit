# Change: add-vector-search

## Why

Issue #14 已经完成了向量化基础设施（`EmbeddingModel`、`cosine_similarity`、向量持久化），但缺少实际的向量检索接口。当前 `KnowledgeStore::search()` 仅支持 BM25 关键词搜索，无法理解语义相关性。

例如：查询 "how to create a red healing block" 应该找到 "Creating Custom Blocks" 和 "Redstone Components" 相关文档，即使不包含精确关键词。BM25 只能匹配关键词，而向量搜索能理解语义意图。

更重要的是，为了满足 1000 文档 < 500ms 的硬性性能要求，必须在 `KnowledgeStore` 中引入内存缓存层，避免检索时遍历磁盘 I/O。

## What Changes

- **新增** `KnowledgeStore.records` 内存缓存字段（`Arc<RwLock<HashMap<String, KnowledgeRecord>>>`）
- **新增** `KnowledgeStore::vector_search()` 方法，实现基于余弦相似度的语义搜索
- **修改** `KnowledgeStore::new()` 在启动时全量加载所有文档到内存缓存
- **修改** `KnowledgeStore::add()` 在写入磁盘后同步更新内存缓存（Write-Through 模式）
- **新增** 单元测试验证语义相似度排序正确性
- **新增** 性能测试验证 1000 文档查询延迟 < 500ms
- **BREAKING**: None

## Impact

### Affected specs

- core-engine

### Affected code

- `packages/core/src/storage/mod.rs` - `KnowledgeStore` 结构体和方法
- `packages/core/src/storage/mod.rs` 测试模块 - 新增测试用例

### Architecture changes

引入内存缓存层作为 `KnowledgeStore` 的核心状态管理机制：
- **Cold Start**: 启动时一次性加载所有 JSON 文档到内存
- **Write-Through**: 写入磁盘时同步更新缓存，保证一致性
- **RAM Scan**: 向量搜索直接遍历内存，避免磁盘 I/O
- **并发安全**: 使用 `Arc<RwLock<>>` 支持多读者单写者并发模式

### Performance implications

- 启动时间增加：需要一次性加载所有文档（但 1000 篇文档仅 10-50 MB，加载时间 < 100ms）
- 内存占用增加：每篇文档约 10-50 KB（包含向量），1000 篇文档约 10-50 MB
- 搜索性能大幅提升：从磁盘扫描（秒级）降低到内存扫描（毫秒级）
