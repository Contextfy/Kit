# Change: 混合检索算法（Hybrid Search）

## Why

当前核心引擎已分别实现 BM25 全文搜索（`search`）和向量语义搜索（`vector_search`），但两者独立运行，无法结合关键词匹配和语义相似度的优势。为提升检索准确率，需要实现混合检索（Hybrid Search），将 BM25 分数和向量相似度按权重融合，提供更精准的排序结果。

## What Changes

- 在 `packages/core/src/storage/mod.rs` 中定义 `BriefWithScore` 结构体，包含 `record`、`bm25_score`、`vector_score`、`final_score` 字段
- 实现 `KnowledgeStore::hybrid_search(&self, query: &str, top_k: usize) -> Result<Vec<BriefWithScore>>` 方法
- 多路召回：分别调用 BM25 `search`（获取 Top-N）和 `vector_search`（获取 Top-N），N 取 `top_k * 2` 保证召回率
- BM25 分数归一化：使用 Min-Max 归一化 `(score - min) / (max - min)`，处理除零情况（当 abs(max - min) < f32::EPSILON 时，如果 bm25_score > 0 则取 1.0，否则取 0.0）
- 结果交并集：使用 `HashMap<String, BriefWithScore>` 按文档 ID 合并两路结果，缺失分数设为 0.0
- 加权融合：按 `final_score = 0.7 * normalized_bm25 + 0.3 * vector_score` 计算最终得分
- 安全排序：使用 `total_cmp` 结合 `record.id` 打破平局，降序排序后截取 `top_k`
- 优雅降级：当向量搜索失败或无模型时，平滑退化为纯 BM25 搜索
- 并发检索：使用 `tokio::join!` 并发执行 BM25 和向量搜索（如借用允许）
- 单元测试：验证归一化逻辑、权重计算、降级场景

**BREAKING**: KnowledgeStore::search() 方法签名变更，新增 limit: usize 参数。调用方必须传入该参数（如 100）。

## Impact

- Affected specs: core-engine
- Affected code:
  - packages/core/src/storage/mod.rs（新增 `hybrid_search` 方法和 `BriefWithScore` 结构体，修复 `search` 方法的 limit 契约）
  - packages/core/src/retriever/mod.rs（新增 `hybrid_scout` 方法，消除魔法数字，添加单元测试）
  - packages/bridge/src/lib.rs（Bridge 层接入核心检索实现，重构生命周期）

### Performance Implications

并发执行 BM25 和向量检索，1000 篇文档下的混合检索预期延迟应保持在 `< 500ms` 级别。
