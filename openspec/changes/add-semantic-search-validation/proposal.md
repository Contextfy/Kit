# Change: 语义搜索验证

## Why

Contextfy/Kit 最近完成了 LanceDB 向量存储和 RRF 混合检索架构的实现（Issue #21, PR #42），结合了 BM25 全文搜索和向量语义搜索。然而，我们缺乏定量的评估数据来验证混合检索是否真正提升了搜索质量。

纯 BM25 搜索存在以下局限：
1. **无法理解语义相似性**：如查询 "heal player" 无法匹配到 "applyDamage" 相关文档
2. **同义词匹配能力弱**：如 "create block" 与 "Block.create()" 之间缺乏语义关联
3. **依赖精确关键词**：用户必须使用准确的术语才能找到相关文档

混合检索（BM25 + Vector + RRF）理论上应该能通过语义相似度弥补这些不足，但需要通过实际测试数据来验证。

本次变更的目标是设计并实现一套评估框架，用于对比纯 BM25 搜索和混合检索的准确率，并生成可量化的评估报告。

## What Changes

### 1. 创建评估测试模块

在 `packages/core/tests/` 下创建 `semantic_evaluation_test.rs`：
- 设计包含 10-15 个语义查询的测试集
- 每个查询包含：
  - 查询文本（如 "heal player", "create block", "spawn entity"）
  - 期望匹配的目标文档 ID 列表（支持多个相关文档）
  - 期望的排名优先级（Top-1, Top-3, Top-5）

### 2. 测试数据来源

**选项 A：使用真实 Minecraft API 文档**
- 从 `docs/minecraft-bedrock/` 加载真实文档
- 优点：真实场景，测试结果更有说服力
- 缺点：需要手动标注相关性，工作量较大

**选项 B：构造 Mock 文档集**
- 在测试代码中构造包含目标关键字的 Mock 文档
- 优点：可控性强，易于标注相关性
- 缺点：可能无法反映真实场景

**推荐方案**：混合方案
- 对于核心测试用例（如 "heal player" → "applyDamage"），使用构造的 Mock 文档确保测试可控
- 对于扩展测试用例，使用部分真实文档增加测试覆盖度

**测试数据结构**（使用多级相关性评分）：
```rust
#[derive(Debug, Clone)]
pub struct ExpectedDoc {
    pub doc_id: String,
    pub relevance_score: u8, // 1, 2, 或 3
}

pub struct TestQuery {
    pub text: String,
    pub expected_docs: Vec<ExpectedDoc>,
}
```

### 3. 评估指标

实现以下指标计算：
- **Accuracy@K**: Top-K 结果中至少有一个相关文档的查询占比
  - Accuracy@1: Top-1 准确率
  - Accuracy@3: Top-3 准确率
  - Accuracy@5: Top-5 准确率
- **NDCG@K**: 归一化折损累积增益，考虑排名质量
  - **关键特性**：使用多级相关性评分（0-3分制）而非二元相关/不相关
  - **评分标准**：3=完美匹配(精确API), 2=高度相关(同类方法), 1=部分相关(概念周边)
  - **价值**：能充分展示混合检索的优势——BM25 可召回大量 1 分文档（字面匹配），但只有向量检索能把 3 分文档（语义匹配）顶到第一位
- **Hit Rate**: 相关文档出现在结果中的查询占比

### 4. 对比测试流程

```rust
async fn run_evaluation() {
    // 1. 准备测试数据
    let test_queries = load_test_queries();

    // 2. 初始化两个搜索引擎
    let bm25_engine = setup_bm25_only_engine().await;
    let hybrid_engine = setup_hybrid_engine().await;

    // 3. 对每个查询执行两种搜索
    for query in test_queries {
        let bm25_results = bm25_engine.search(&query.text, 10).await;
        let hybrid_results = hybrid_engine.search(&query.text, 10).await;

        // 4. 记录排名并计算指标
        record_rankings(&query, &bm25_results, &hybrid_results);
    }

    // 5. 生成对比报告
    generate_evaluation_report();
}
```

### 5. 报告生成

生成 Markdown 格式的评估报告 `docs/SEMANTIC_EVALUATION_REPORT.md`：
- 包含生成时间戳
- 摘要对比表（BM25 vs Hybrid）
- 每个查询的详细结果对比
- 指标分析和改进幅度
- 质量门禁判定

### 6. 接受标准

- ✅ 混合检索 Top-3 准确率 > 80%
- ✅ 语义查询能找到同义词相关文档
- ✅ 生成的评估报告格式完整，数据准确
- ✅ 测试代码覆盖率达到 70%+

**BREAKING**: None（纯测试添加，不修改生产代码）

## Impact

- Affected specs: core-engine（添加评估需求）
- Affected code:
  - `packages/core/tests/semantic_evaluation_test.rs`（新建）
  - `docs/SEMANTIC_EVALUATION_REPORT.md`（生成）
  - 可选：`packages/core/src/evaluation/`（新建评估模块，如果需要复用）
- Dependencies:
  - `tempfile`（用于临时测试目录，已存在）
  - `chrono`（用于生成时间戳，如未添加需添加）
- **BREAKING**: None（纯测试添加）
