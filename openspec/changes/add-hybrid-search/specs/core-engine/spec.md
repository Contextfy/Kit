# core-engine Specification Deltas

## ADDED Requirements

### Requirement: 混合检索算法

The core engine SHALL provide hybrid search that combines BM25 keyword matching and vector semantic similarity with score normalization and weighted fusion. 核心引擎 SHALL 提供混合检索，结合 BM25 关键词匹配和向量语义相似度，支持分数归一化和加权融合。

#### Scenario: 混合检索返回融合排序结果

- **当**用户调用 `hybrid_search(query, top_k)` 时
- **则**系统分别调用 BM25 `search` 和 `vector_search` 获取 Top-N 结果（N = top_k * 2）
- **并且**系统对 BM25 分数执行 Min-Max 归一化到 [0, 1] 范围
- **并且**系统使用 `HashMap` 按文档 ID 合并两路结果，缺失分数设为 0.0
- **并且**系统按 `final_score = 0.7 * normalized_bm25 + 0.3 * vector_score` 计算最终得分
- **并且**系统按 `final_score` 降序排序，分数相同时使用文档 ID 作为 tie-breaker
- **并且**系统返回前 `top_k` 个 `BriefWithScore` 结果，包含原始分数和最终分数

#### Scenario: BM25 分数归一化处理

- **当**系统执行 BM25 分数 Min-Max 归一化时
- **则**系统找出所有**有效** BM25 分数（大于 0.0 且为有限值）的 `min` 和 `max` 值
- **并且**系统过滤掉 0.0 分数（来自仅向量召回的结果）避免污染归一化区间
- **并且**系统使用公式 `normalized = (score - min) / (max - min)` 计算归一化分数
- **如果** `max` 和 `min` 极度接近（`abs(max - min) < f32::EPSILON`），系统检查原始 BM25 分数：
  - 当原始分数 `> 0` 时，归一化为 `1.0`
  - 当原始分数为 `0.0` 时，归一化为 `0.0`
- **并且**系统使用 `f32::EPSILON` 检查防止除零错误

#### Scenario: 结果交并集合并

- **当**系统合并 BM25 和向量搜索结果时
- **则**系统使用 `HashMap<String, BriefWithScore>` 以文档 ID 为 Key 存储结果
- **如果**文档仅在 BM25 结果中，系统的 `vector_score` 设为 0.0
- **如果**文档仅在向量结果中，系统的 `bm25_score` 和归一化 BM25 分数均设为 0.0
- **如果**文档在两个结果中都存在，系统保留两个分数用于加权计算

#### Scenario: 优雅降级为纯 BM25 搜索

- **当**向量搜索不可用时（无嵌入模型或搜索失败）
- **则**系统记录警告并继续执行 BM25 搜索
- **并且**系统将所有 `vector_score` 设为 0.0
- **并且**系统按加权融合公式计算最终分数（`final_score = 0.7 * normalized_bm25 + 0.3 * 0.0`）
- **并且**系统不崩溃或返回错误

#### Scenario: 并发检索优化

- **当**借用关系允许时
- **则**系统使用 `tokio::join!` 并发执行 BM25 和向量搜索
- **并且**系统减少总检索延迟（接近较慢那路的延迟）
- **如果**存在借用冲突，系统顺序执行两路搜索
- **并且**系统不影响错误处理和优雅降级逻辑

#### Scenario: 加权权重配置

- **当**系统计算最终融合分数时
- **则**系统使用权重配置：BM25 70%，向量 30%
- **并且**系统确保权重和为 1.0（`0.7 + 0.3 = 1.0`）
- **并且**系统在文档注释中说明权重选择的依据（关键词匹配的精确性 vs 语义泛化能力）

#### Scenario: 排序稳定性和 NaN 处理

- **当**系统对结果排序时
- **则**系统的比较器对浮点数实现了全序排序（Total Ordering），将 NaN 分数归一化为最小值使其排在最后
- **并且**系统在分数相同时确定性地使用 record.id 打破平局（Tie-breaker）
- **并且**系统确保相同输入产生相同排序结果（可重现性）

#### Scenario: 边界条件处理

- **当** `top_k` 为 0 时
- **则**系统立即返回空结果
- **当** `query` 为空或仅包含空白字符时
- **则**系统立即返回空结果
- **当**两路搜索都返回空结果时
- **则**系统返回空结果列表

#### Scenario: 数据结构定义

- **当**系统定义 `BriefWithScore` 结构体时
- **则**系统包含以下字段：
  - `record: KnowledgeRecord` - 完整的知识记录
  - `bm25_score: f32` - 原始 BM25 分数
  - `vector_score: f32` - 余弦相似度（范围 [0, 1]）
  - `final_score: f32` - 加权融合后的最终分数
- **并且**系统派生 `Debug` 和 `Clone` trait
- **并且**系统在公共 API 中导出该结构体

#### Scenario: 混合检索与分数融合

- **当**系统计算最终融合分数时
- **则**系统使用 Min-Max 归一化公式：`normalized_bm25 = (bm25_score - min_bm25) / (max_bm25 - min_bm25)`，其中 `min_bm25` 和 `max_bm25` 仅包含大于 0.0 的有效 BM25 分数
- **并且**系统使用加权融合公式：`final_score = 0.7 * normalized_bm25 + 0.3 * vector_score`
- **并且**系统确保当文档同时具有 BM25 分数和向量分数时（归一化后均为 1.0），最终分数为 1.0
- **并且**系统确保当文档仅有 BM25 分数（归一化后为 1.0）时，最终分数为 0.7
- **并且**系统确保当文档仅有向量分数（为 1.0）时，最终分数为 0.3
- **并且**系统在除零情况下使用 `f32::EPSILON` 检查，当 `max_bm25 - min_bm25` 的绝对值小于 `EPSILON` 时，将归一化值设为 1.0（如果 BM25 分数大于 0）或 0.0

## MODIFIED Requirements

None

## REMOVED Requirements

None
