# Change: 完成存储层替换 - LanceDB 混合检索架构

## Why

在 Issue #19 和 #20 中，我们已经完成了 LanceDB 的依赖引入和旧 JSON 数据的迁移。然而，**核心运行链路**仍然需要验证和确认以下关键点：

1. **架构完整性**：确认新的 LanceDB + Tantivy 混合检索架构已经完全取代旧的 JSON 存储系统
2. **混合检索验证**：确保 `search()` 方法正确实现混合检索（BM25 + 向量）并使用 RRF 策略合并结果
3. **API 兼容性**：验证现有调用方（CLI、Server）无需修改即可使用新架构
4. **资源管理**：确认 LanceDB 连接和 Embedding 模型在应用启动时正确初始化并通过 `Arc` 共享

**关键发现**：经过代码调查，新架构**已经基本实现**，但需要正式确认和文档化当前状态。

## What Changes

### 1. 确认当前架构状态

经过详细的代码调查，**存储层替换已完成**，当前架构如下：

```
┌─────────────────────────────────────────────┐
│           调用方 (CLI/Server)                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│         SearchEngine (Facade)               │
│  - search(query, limit)                     │
│  - add(id, title, summary, content, keywords)│
│  - get_document(id)                         │
│  - delete(id)                               │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│      HybridOrchestrator (混合检索)          │
│  - 并行执行 BM25 + 向量搜索                 │
│  - RRF (Reciprocal Rank Fusion) 合并结果    │
│  - 优雅的错误降级                           │
└────────┬──────────────────────┬─────────────┘
         │                      │
    ▼    │                      ▼
┌──────────────┐        ┌──────────────┐
│ LanceDBStore │        │TantivyBm25Store│
│  (向量搜索)  │        │  (BM25搜索)   │
└──────────────┘        └──────────────┘
```

### 2. 核心实现确认

#### 2.1 VectorStoreTrait (向量存储抽象)

**位置**: `packages/core/src/slices/vector/trait_.rs`

```rust
#[async_trait]
pub trait VectorStoreTrait: Send + Sync {
    /// 向量搜索
    async fn search(&self, query: &Query) -> Result<Option<Vec<Hit>>, AppError>;

    /// 添加文档
    async fn add(&self, id: &str, text: &str, metadata: Option<&serde_json::Value>)
        -> Result<(), AppError>;

    /// 删除文档
    async fn delete(&self, id: &str) -> Result<bool, AppError>;

    /// 健康检查
    async fn health_check(&self) -> Result<bool, AppError>;
}
```

**实现**: `LanceDbStore` (`packages/core/src/slices/vector/lancedb_impl.rs`)
- 使用 LanceDB 执行向量相似度搜索
- 支持批量 Embedding 生成（通过共享 `EmbeddingModel`）
- L2 距离归一化到 [0.0, 1.0] 分数

#### 2.2 Bm25StoreTrait (BM25 全文搜索抽象)

**位置**: `packages/core/src/slices/bm25/trait_.rs`

**实现**: `TantivyBm25Store` (`packages/core/src/slices/bm25/tantivy_impl.rs`)
- 使用 Tantivy 索引执行 BM25 全文搜索
- 支持关键词字段提升权重（5.0 - 10.0）
- 返回完整文档详情（title, summary, content）

#### 2.3 HybridOrchestrator (混合检索编排器)

**位置**: `packages/core/src/slices/hybrid/orchestrator.rs`

**核心方法**:

```rust
impl HybridOrchestrator {
    /// 混合搜索 - 并行执行 BM25 和向量搜索，使用 RRF 合并结果
    pub async fn search(&self, query: &Query) -> Result<Vec<Hit>, AppError> {
        // 1. 并行执行两个搜索
        let (vector_result, bm25_result) = tokio::join!(
            self.vector_store.search(query),
            self.bm25_store.search(query)
        );

        // 2. 处理错误降级
        match (vector_hits, bm25_hits) {
            (Ok(v), Ok(b)) => {
                // 两者都成功 - 使用 RRF 合并
                let fused = self.rrf.fuse_two(v, b)?;
                Ok(fused)
            }
            (Ok(v), Err(_)) => {
                // 向量成功，BM25 失败 - 只返回向量结果
                Ok(v)
            }
            (Err(_), Ok(b)) => {
                // BM25 成功，向量失败 - 只返回 BM25 结果
                Ok(b)
            }
            (Err(_), Err(_)) => {
                // 两者都失败 - 返回错误
                Err(...)
            }
        }
    }

    /// 添加文档到两个存储（带事务性保证）
    pub async fn add(&self, id: &str, title: &str, summary: &str,
                     content: &str, keywords: Option<&str>)
        -> Result<(), AppError> {
        // 并行添加，如果失败则回滚
        ...
    }

    /// 删除文档（返回详细结果）
    pub async fn delete(&self, id: &str) -> DeleteResult {
        // 并行删除，保留各自结果
        ...
    }
}
```

**RRF 算法实现**:
- 公式: `rrf_score(d) = Σ 1 / (k + rank_d)` 其中 k=60
- 合并两个排序列表，重新计算相关性分数
- 支持多个结果源的融合

#### 2.4 SearchEngine (高级 Facade)

**位置**: `packages/core/src/facade.rs`

**核心 API**:

```rust
impl SearchEngine {
    /// 创建搜索引擎（初始化所有后端）
    pub async fn new(
        index_dir: Option<&Path>,      // Tantivy 索引目录
        lancedb_uri: &str,              // LanceDB 连接 URI
        table_name: &str,               // LanceDB 表名
    ) -> Result<Self>;

    /// 混合搜索
    pub async fn search(&self, query_text: &str, limit: usize)
        -> Result<Vec<Hit>>;

    /// 添加文档
    pub async fn add(&self, id: &str, title: &str, summary: &str,
                     content: &str, keywords: Option<&str>)
        -> Result<()>;

    /// 获取文档详情
    pub async fn get_document(&self, id: &str)
        -> Result<Option<DocumentDetails>>;

    /// 批量获取文档
    pub async fn get_documents(&self, ids: &[String])
        -> Result<Vec<Option<DocumentDetails>>>;

    /// 删除文档
    pub async fn delete(&self, id: &str) -> DeleteResult;

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool>;
}
```

### 3. 资源管理策略

#### 3.1 EmbeddingModel 单例模式

**位置**: `packages/core/src/facade.rs:42-61`

```rust
fn shared_embedding_model() -> Result<Arc<EmbeddingModel>> {
    static EMBEDDING_MODEL_CELL: OnceLock<Mutex<Option<Arc<EmbeddingModel>>>> = OnceLock::new();

    let cell = EMBEDDING_MODEL_CELL.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock()?;

    if let Some(model) = guard.as_ref() {
        // 已初始化，返回克隆的 Arc
        Ok(Arc::clone(model))
    } else {
        // 首次初始化（下载并加载 BGE-small-en 模型）
        let model = EmbeddingModel::new().map(Arc::new)?;
        *guard = Some(model.clone());
        Ok(model)
    }
}
```

**优势**:
- 模型只加载一次（1-5 分钟冷启动）
- 后续调用返回克隆的 Arc（微秒级）
- 所有 `LanceDbStore` 实例共享同一个模型

#### 3.2 LanceDB 连接管理

**初始化流程**:
1. 连接到 LanceDB 数据库 (`connect(lancedb_uri)`)
2. 创建或打开表 (`create_table_if_not_exists`)
3. 创建 `LanceDbStore` 并共享连接
4. 连接对象由 `LanceDbStore` 持有，无需额外管理

### 4. API 兼容性验证

**CLI 使用示例** (已验证):

`packages/cli/src/commands/build.rs:48-54`:
```rust
let engine = SearchEngine::new(
    Some(std::path::Path::new(".contextfy/data/bm25_index")),
    ".contextfy/data/lancedb",
    "knowledge",
).await?;

// 添加文档
engine.add(&id, &title, &summary, &content, None).await?;
```

`packages/cli/src/commands/scout.rs:29-34`:
```rust
let engine = SearchEngine::new(
    Some(std::path::Path::new(".contextfy/data/bm25_index")),
    ".contextfy/data/lancedb",
    "knowledge",
).await?;

// 混合搜索
let hits = engine.search(&query, 10).await?;

// 获取文档详情
let docs = engine.get_documents(&ids).await?;
```

**结论**: ✅ API 完全兼容，无需修改调用方代码

## Impact

- **Affected specs**: core-engine (确认存储层架构)
- **Affected code**: 无需修改（架构已就位）
- **BREAKING**: None
- **文档更新**: 需要更新架构文档以反映新设计

## Tasks

### Phase 1: 验证与确认 (1-2 天)

- [ ] 1.1 运行现有测试套件，确认所有测试通过
- [ ] 1.2 手动测试 CLI 命令 (`build`, `scout`, `serve`)
- [ ] 1.3 验证混合检索质量（使用已知查询对比结果）
- [ ] 1.4 检查日志输出，确认 RRF 合并被正确执行

### Phase 2: 文档更新 (1 天)

- [ ] 2.1 更新 `docs/Architecture.md`，描述新的三层架构
- [ ] 2.2 更新 `README.md`，移除对 JSON 存储的引用
- [ ] 2.3 添加"混合检索原理"文档，解释 RRF 算法
- [ ] 2.4 更新 API 文档，标注 `SearchEngine` 为主要入口

### Phase 3: 性能优化 (可选，2-3 天)

- [ ] 3.1 添加性能基准测试（对比旧 JSON 方案）
- [ ] 3.2 优化 LanceDB 向量索引参数（IVF-PQ）
- [ ] 3.3 优化 Embedding 批处理大小
- [ ] 3.4 添加查询缓存层（可选）

### Phase 4: 清理与归档 (1 天)

- [ ] 4.1 归档 Issue #21
- [ ] 4.2 更新 CHANGELOG.md
- [ ] 4.3 关闭相关的旧 Issues (#19, #20)

## Migration Strategy

**无需迁移** - 新架构已经就位并在使用中。

唯一需要的是：
1. **验证**: 确认所有功能正常工作
2. **文档化**: 更新文档以反映新架构
3. **性能测试**: 对比新旧方案的性能

## Open Questions

1. **旧数据迁移**: Issue #20 的迁移工具是否已经测试和验证？
   - 建议: 运行迁移工具并验证数据完整性

2. **性能基准**: 新架构相比旧 JSON 方案的性能如何？
   - 建议: 运行基准测试并记录结果

3. **生产就绪**: LanceDB 在生产环境中的稳定性如何？
   - 建议: 在测试环境中运行压力测试

## Testing Strategy

### 单元测试 (已有)

- `VectorStoreTrait` 测试 (`packages/core/src/slices/vector/lancedb_impl.rs:431-619`)
- `Bm25StoreTrait` 测试 (`packages/core/src/slices/bm25/tantivy_impl.rs`)
- `HybridOrchestrator` 测试 (`packages/core/src/slices/hybrid/orchestrator.rs:416-873`)
- `SearchEngine` 测试 (`packages/core/src/facade.rs:362-443`)

### 集成测试 (待添加)

```rust
#[tokio::test]
async fn test_end_to_end_hybrid_search() {
    // 1. 创建测试搜索引擎
    let engine = SearchEngine::new(...).await.unwrap();

    // 2. 添加测试文档
    engine.add("doc1", "Rust", "A systems language", "...", None).await.unwrap();

    // 3. 执行混合搜索
    let hits = engine.search("systems programming", 10).await.unwrap();

    // 4. 验证结果
    assert!(!hits.is_empty());
    assert_eq!(hits[0].id, "doc1");
}
```

### 性能测试 (待添加)

```rust
#[tokio::test]
async fn benchmark_hybrid_search_latency() {
    let engine = SearchEngine::new(...).await.unwrap();

    let start = Instant::now();
    let hits = engine.search("test query", 10).await.unwrap();
    let latency = start.elapsed();

    assert!(latency < Duration::from_millis(100), "Hybrid search should be < 100ms");
}
```

## Success Criteria

- ✅ 所有现有测试通过
- ✅ CLI 命令 (`build`, `scout`) 正常工作
- ✅ 混合检索返回预期结果
- ✅ 文档已更新
- ✅ 无 BREAKING 变更引入
- ✅ 性能可接受（查询延迟 < 100ms）

## Notes

**重要发现**: 本提案确认了**存储层替换已完成**，主要工作是验证和文档化，而非新的实现。

这是因为：
1. Issue #19 (#18) 已完成 LanceDB 依赖集成
2. Issue #20 已完成 JSON 到 LanceDB 的数据迁移工具
3. Issue #18 已完成混合检索和 RRF 实现
4. 当前代码库已经完全使用新架构

因此，Issue #21 的实际工作范围是：
- **验证**现有实现的正确性
- **文档化**新架构
- **测试**功能完整性
- **可选的性能优化**

## References

- `openspec/changes/archive/2026-05-04-add-lancedb-dependency/proposal.md` - LanceDB 集成
- `openspec/changes/archive/2026-05-04-migrate-json-to-lancedb/proposal.md` - 数据迁移
- `openspec/changes/archive/2026-03-16-add-hybrid-search/proposal.md` - 混合检索
- `packages/core/src/facade.rs` - SearchEngine 实现
- `packages/core/src/slices/hybrid/orchestrator.rs` - HybridOrchestrator 实现
