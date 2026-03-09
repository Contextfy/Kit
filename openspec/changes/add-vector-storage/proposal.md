# Change: 向量存储与余弦相似度计算

## Why

当前核心引擎已经通过 Issue #13 引入了 `EmbeddingModel`，但向量仅用于一次性计算，未持久化存储。为实现混合检索（Hybrid Search：BM25 + 向量相似度），需要将文档的向量嵌入持久化到 JSON 存储中，并提供高性能的余弦相似度计算能力。

这将为后续的语义搜索（Semantic Search）奠定基础，使系统能够理解查询与文档之间的语义相似度，而不仅仅是关键词匹配。

## What Changes

- 在 `packages/core/src/storage/mod.rs` 中为 `KnowledgeRecord` 添加 `embedding: Option<Vec<f32>>` 字段
- 创建 `packages/core/src/embeddings/math.rs` 模块，实现 `cosine_similarity(a, b) -> f32` 函数
- 在 `EmbeddingModel` 中新增 `embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>` 方法
- 在 `KnowledgeStore::add()` 中集成向量生成：拼接 `title` 和 `summary` 并调用 `embed_batch`
- 为向量计算添加单元测试（余弦相似度测试、批处理测试）
- 确保 `KnowledgeRecord` 的 `embedding` 字段使用 `#[serde(default)]` 以兼容旧版 JSON 数据

**数学严谨性**：
- 余弦相似度公式：$\text{similarity} = \frac{A \cdot B}{\|A\| \|B\|}$
- 归一化映射到 [0, 1] 范围：`mapped_sim = (raw_cosine + 1.0) / 2.0`
- 除零保护：当分母为 0 时返回 0.0

**批量处理防线**：
- 必须先收集所有切片的拼接文本到 `Vec<&str>`
- 一次性调用 `embed_batch`，绝不能在循环中逐个生成

**异步安全防线**：
- 在 `async fn add()` 中调用 `embed_text` 和 `embed_batch` 必须使用 `tokio::task::spawn_blocking`
- 避免阻塞 Tokio 异步 worker 线程

**BREAKING**: `KnowledgeStore::new()` 新增必传的 `Option<Arc<EmbeddingModel>>` 参数（向后兼容，调用方可传 None）

## Impact

- Affected specs: core-engine
- Affected code:
  - packages/core/src/storage/mod.rs, packages/core/src/embeddings/mod.rs
  - Affected callers (updated to pass `embedding_model` parameter):
    - packages/cli/src/commands/build.rs
    - packages/cli/src/commands/scout.rs
    - packages/server/src/main.rs
