# Design: 语义搜索验证

## Context

Contextfy/Kit 项目最近实现了混合检索架构（BM25 + LanceDB Vector + RRF），理论上应该能通过语义相似度弥补纯 BM25 搜索的不足。然而，我们缺乏定量的评估数据来验证这一假设。

**约束条件**：
- 测试需要在独立的 CI/CD 环境中运行，不能依赖外部服务
- 测试数据应尽量模拟真实使用场景
- 评估指标应符合信息检索领域的标准实践
- 报告生成应自动化，易于在 PR Review 中查看

**利益相关者**：
- 开发团队：需要评估数据来优化搜索算法
- QA 团队：需要明确的接受标准来验证搜索质量
- 最终用户：受益于更准确的搜索结果

---

## Goals / Non-Goals

### Goals

1. **定量评估混合检索效果**：通过对比 BM25 vs Hybrid 的准确率，验证混合检索的改进幅度
2. **验证语义搜索能力**：确保系统能理解同义词、动作变体、中英文混合等语义查询
3. **建立质量基线**：为未来的搜索算法优化提供可对比的基线数据
4. **自动化报告生成**：每次代码变更后自动生成评估报告，便于回归测试

### Non-Goals

1. **不修改生产代码**：本次变更仅添加测试代码，不修改现有的搜索引擎实现
2. **不优化搜索算法**：如果准确率不达标，不在本次变更中调整 RRF 参数或 embedding 模型
3. **不替代人工评测**：自动化评估无法完全覆盖真实使用场景，仍需人工抽查验证
4. **不支持多语言**：本次仅测试英文和少量中文查询，不涉及其他语言

---

## Decisions

### Decision 1: 测试数据来源

**选择**：使用混合方案（Mock 文档 + 真实文档）

**理由**：
- **Mock 文档**：对于核心测试用例（如 "heal player" → "applyDamage"），使用构造的文档可以确保测试可控性和可重复性
- **真实文档**：对于扩展测试用例，从 `docs/minecraft-bedrock/` 加载真实 API 文档可以增加测试覆盖度和真实感

**实施细节**：
```rust
// Mock 文档示例
let mock_docs = vec![
    Document {
        id: "doc-001".to_string(),
        title: "Entity.applyDamage() Method".to_string(),
        summary: "Applies damage to an entity".to_string(),
        content: "The applyDamage() method applies damage to an entity. \
                  This is commonly used to hurt entities like players or mobs.".to_string(),
    },
    // ... 更多 mock 文档
];
```

**Alternatives Considered**：
- **纯 Mock 文档**：简单可控，但可能无法反映真实场景
- **纯真实文档**：真实但需要大量人工标注相关性，工作量较大

---

### Decision 2: 评估指标选择

**选择**：使用 Accuracy@K, NDCG@K, Hit Rate 三个指标

**理由**：
1. **Accuracy@K**：简单直观，易于理解，符合业务需求（用户主要关注 Top-3 结果）
2. **NDCG@K**：考虑排名质量，能反映搜索结果的整体排序质量
   - **关键价值**：使用多级相关性评分（0-3分）能更好地区分不同级别的相关度
   - **为什么重要**：混合检索（BM25 + 向量）最常见的情况是 BM25 能召回大量 1 分文档（字面匹配），但只有向量检索能把 3 分文档（语义匹配）顶到第一位
3. **Hit Rate**：与 Accuracy 类似，但不限制排名，用于评估召回能力

**相关性评分标准**（多级评分制）：
```rust
/// 推荐的相关性定义
/// - 3 = 完美匹配（精确API，如 "heal player" → "applyDamage"）
/// - 2 = 高度相关（同类方法，如 "heal player" → "hurtEntity"）
/// - 1 = 部分相关（概念周边，如 "heal player" → "Entity"）
/// - 0 = 不相关
#[derive(Debug, Clone)]
pub struct ExpectedDoc {
    pub doc_id: String,
    pub relevance_score: u8, // 0, 1, 2, 或 3
}
```

**计算公式**（零拷贝实现，无外部依赖）：
```rust
/// 计算单个位置的 DCG (Discounted Cumulative Gain)
/// 公式: DCG_k = sum_{i=1}^k (rel_i / log2(i + 1))
fn calculate_dcg(relevances: &[f64], k: usize) -> f64 {
    relevances
        .iter()
        .take(k)
        .enumerate()
        .fold(0.0, |acc, (i, &rel)| {
            // i 是 0-indexed，所以公式里的 (i + 1 + 1) 即为 (i + 2)
            acc + (rel / ((i + 2) as f64).log2())
        })
}

/// 计算 NDCG@K (Normalized Discounted Cumulative Gain)
///
/// `actual_ranking_ids`: 搜索引擎实际返回的文档 ID 列表 (按顺序)
/// `expected_scores`: 期望文档及其对应相关性得分的 HashMap
pub fn calculate_ndcg_at_k(
    actual_ranking_ids: &[String],
    expected_scores: &std::collections::HashMap<String, f64>,
    k: usize,
) -> f64 {
    // 1. 计算实际排名的相关性得分列表
    let actual_relevances: Vec<f64> = actual_ranking_ids
        .iter()
        .take(k)
        .map(|doc_id| *expected_scores.get(doc_id).unwrap_or(&0.0))
        .collect();

    // 2. 算出当前的实际 DCG
    let dcg = calculate_dcg(&actual_relevances, k);

    // 3. 计算理想情况下的 DCG (IDCG)
    // 做法：取出所有已知的相关性得分，降序排列，取前 K 个
    let mut ideal_relevances: Vec<f64> = expected_scores.values().copied().collect();
    ideal_relevances.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let idcg = calculate_dcg(&ideal_relevances, k);

    // 4. 防御性除零保护：如果期望文档全为空，或者 IDCG 为 0，说明这个测试用例无效
    if idcg <= 0.0 {
        return 0.0;
    }

    // 5. 归一化，得到 0.0 ~ 1.0 的最终得分
    dcg / idcg
}

// Accuracy@K: Top-K 结果中至少有一个相关文档的查询占比
fn accuracy_at_k(results: &[EvalResult], k: usize) -> f64 {
    let hit_count = results.iter()
        .filter(|r| r.hybrid_ranking.iter().take(k)
            .any(|id| {
                r.query.expected_docs.iter()
                    .any(|exp| exp.doc_id == *id && exp.relevance_score > 0)
            }))
        .count();
    hit_count as f64 / results.len() as f64
}
```

**Alternatives Considered**：
- **MAP (Mean Average Precision)**：更复杂，对于多级相关性收益有限
- **MRR (Mean Reciprocal Rank)**：仅关注第一个相关文档的排名，无法利用多级相关性
- **Precision/Recall**：需要定义明确的"检索到的文档集合"，不适合 Top-K 搜索场景
- **二元相关性（相关/不相关）**：无法区分不同级别的相关度，无法充分展示混合检索的优势

---

### Decision 3: 测试查询设计

**选择**：设计 10-15 个覆盖不同语义场景的查询

**查询类别**：

| 类别 | 示例查询 | 期望匹配 | 测试目标 |
|------|---------|---------|---------|
| 同义词 | heal player | applyDamage, hurtEntity | 语义理解能力 |
| 动作变体 | create block | Block.create(), BlockCustomComponent | 动作与对象的语义关联 |
| 中英文混合 | 方块 | Block | 多语言支持 |
| 缩写扩展 | entity component | EntityComponent, MinecraftBlockComponent | 缩写识别 |
| 功能描述 | spawn entity | Entity.create(), EntityType.spawn() | 功能描述与 API 的映射 |

**实施细节**：
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

let test_queries = vec![
    TestQuery {
        text: "heal player".to_string(),
        expected_docs: vec![
            ExpectedDoc {
                doc_id: "doc-001".to_string(),
                relevance_score: 3, // 完美匹配：Entity.applyDamage() - 精确API
            },
            ExpectedDoc {
                doc_id: "doc-002".to_string(),
                relevance_score: 2, // 高度相关：hurtEntity() - 同类方法
            },
            ExpectedDoc {
                doc_id: "doc-003".to_string(),
                relevance_score: 1, // 部分相关：Entity - 概念周边
            },
        ],
    },
    TestQuery {
        text: "create block".to_string(),
        expected_docs: vec![
            ExpectedDoc {
                doc_id: "doc-005".to_string(),
                relevance_score: 3, // 完美匹配：Block.create()
            },
            ExpectedDoc {
                doc_id: "doc-006".to_string(),
                relevance_score: 2, // 高度相关：BlockCustomComponent
            },
        ],
    },
    // ... 更多查询
];
```

**Alternatives Considered**：
- **随机采样查询**：简单但无法覆盖特定的语义场景
- **用户日志分析**：真实但需要大量用户数据，且用户查询可能存在偏差

---

### Decision 4: 报告生成格式

**选择**：生成 Markdown 格式的报告，参考 `docs/BM25_EVALUATION_REPORT.md`

**理由**：
- Markdown 格式易于在 GitHub/GitLab 中查看和 diff
- 可以直接在 PR Review 中评论和讨论
- 易于版本控制和历史追溯

**报告结构**：
```markdown
# 语义搜索评估报告

**生成时间**: 2026-05-02 12:00:00

## 📊 摘要

### BM25 vs Hybrid 整体对比

| 指标 | BM25 搜索 | Hybrid 搜索 | 改进 |
|------|-----------|-------------|------|
| Accuracy@1 | XX.X% | XX.X% | +XX.XX% |
| Accuracy@3 | XX.X% | XX.X% | +XX.XX% |
| NDCG@3 | X.XXX | X.XXX | +XX.XX% |

## 📈 详细对比

### 每个查询的 Top-3 结果对比

#### Q1 - heal player

**标准答案**: doc-001, doc-002

| 排名 | BM25 结果 | Hybrid 结果 | 状态 |
|------|-----------|-------------|------|
| 1 | doc-010 | doc-001 | ✅ |
| 2 | doc-005 | doc-002 | ✅ |
| 3 | doc-001 | doc-010 | ✅ |

... 更多查询

## ✅ 质量门禁

- ✅ 通过：Hybrid Top-3 准确率 (XX.X%) ≥ 80%

**结论**: 语义搜索验证通过 / 未通过
```

**Alternatives Considered**：
- **JSON 格式**：机器可读但不便于人类查看
- **HTML 格式**：美观但不易于版本控制

---

### Decision 5: 测试代码组织

**选择**：在 `packages/core/tests/` 下创建独立的评估测试文件

**理由**：
- 集成测试应放在 `tests/` 目录，而非 `src/` 目录
- 独立的测试文件便于维护和扩展
- 可以使用 `cargo test --test semantic_evaluation` 单独运行

**文件结构**：
```
packages/core/
├── tests/
│   └── semantic_evaluation_test.rs  # 主测试文件
└── src/
    └── evaluation/  # 可选：评估模块（如果需要复用）
        ├── mod.rs
        ├── metrics.rs  # 指标计算
        └── report.rs   # 报告生成
```

**Alternatives Considered**：
- **在 `src/` 中实现评估模块**：可能增加生产代码的复杂度
- **作为单独的 crate**：过度设计，增加维护成本

---

## Risks / Trade-offs

### Risk 1: 混合检索准确率不达标

**概率**：中等

**影响**：高

**缓解措施**：
1. **调整 RRF 参数**：如果准确率 < 80%，可以调整 `k` 值（当前 60）或权重
2. **优化 BM25 keywords**：为测试文档添加更准确的关键字
3. **更换 embedding 模型**：当前使用 BGE-small-en，可以尝试更大的模型
4. **扩充测试集**：如果部分查询设计不合理，可以调整或删除

**降级方案**：
- 如果无法达到 80%，可以降低接受标准到 70%，但需要在 Issue 中说明原因

---

### Risk 2: 测试数据构造偏差

**概率**：中等

**影响**：中

**缓解措施**：
1. **多样化查询设计**：覆盖同义词、动作变体、中英文混合等多种场景
2. **人工验证相关性**：每个查询的相关性标注需经至少 2 人审核
3. **引入真实文档**：部分测试使用 `docs/minecraft-bedrock/` 的真实文档
4. **定期更新测试集**：根据用户反馈和使用数据调整测试查询

---

### Risk 3: NDCG 计算复杂度高

**概率**：低

**影响**：低

**缓解措施**：
1. **参考现有实现**：参考 BM25 评估报告的 NDCG 计算逻辑
2. **单元测试验证**：为 NDCG 计算编写充分的单元测试
3. **简化相关性评分**：使用二值相关性（相关=1，不相关=0），而非多级评分

---

## Migration Plan

### 实施步骤

1. **阶段 1：准备测试数据**（任务 1-2）
   - 设计测试查询
   - 构造测试文档集
   - 手动验证相关性

2. **阶段 2：实现评估框架**（任务 3-5）
   - 实现评估测试框架
   - 实现指标计算
   - 实现报告生成

3. **阶段 3：测试和验证**（任务 6-8）
   - 编写集成测试
   - 运行完整测试套件
   - 生成评估报告

4. **阶段 4：Review 和迭代**（可选）
   - 如果准确率不达标，根据结果调整算法
   - 更新提案和文档

### Rollback Plan

- 如果评估框架无法正常运行，可以回滚到之前的纯 BM25 搜索
- 如果测试数据构造有问题，可以调整为更简单的 Mock 数据
- 如果报告生成失败，可以暂时使用 console output 而非 Markdown 文件

---

## Open Questions

1. **Q: 是否需要支持多语言查询？**
   - A: 本次仅支持英文和少量中文，多语言支持作为未来工作

2. **Q: 如何处理相关性标注的主观性？**
   - A: 每个查询的相关性标注需经至少 2 人审核，争议时采用多数投票

3. **Q: 评估报告是否需要包含图表？**
   - A: 本次仅生成 Markdown 文本，图表可以后续通过外部工具生成

4. **Q: 测试数据是否需要版本化？**
   - A: 测试查询和文档集应纳入版本控制，确保结果可重复

---

## References

- [Reciprocal Rank Fusion](https://plg.uwaterloo.ca/~gvcormac/cormacksigir09-rrf.pdf) - RRF 论文
- [NDCG Wikipedia](https://en.wikipedia.org/wiki/Discounted_cumulative_gain) - NDCG 定义
- `docs/BM25_EVALUATION_REPORT.md` - BM25 评估报告参考
- `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - 混合检索架构设计
