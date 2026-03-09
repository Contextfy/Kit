## ADDED Requirements

### Requirement: BM25 搜索效果评估

The core engine SHALL provide an automated evaluation harness to quantify BM25 search effectiveness compared to naive text matching (M1), with reproducible test reports and quality gates. 核心引擎 SHALL 提供自动化评估脚手架以量化 BM25 搜索相比朴素文本匹配（M1）的效果，具有可复现的测试报告和质量门禁。

#### Scenario: 运行 A/B 搜索效果评估测试

- **当**开发者运行集成测试 `cargo test --test evaluation_test` 时
- **则**系统加载 18 篇硬编码的模拟 Minecraft 模组开发文档
- **并且**系统对预定义的 10 个查询同时执行 M1 朴素匹配和 BM25 搜索
- **并且**系统对比两者的 Top-3 结果与人工标注的 Ground Truth
- **并且**系统计算 Accuracy@3、NDCG@3、Hit Rate@3 三项指标
- **并且**系统在 `docs/` 目录生成 `BM25_EVALUATION_REPORT.md` 报告文件
- **并且**系统断言 BM25 的 Top-3 准确率必须 > 70%（质量门禁）

#### Scenario: M1 朴素匹配搜索实现（基线对比）

- **当**评估脚手架调用 `naive_match_search(query, documents)` 时
- **则**系统使用空格分词将查询拆分为多个 tokens
- **并且**系统检查每个 token 是否出现在文档的 `title` 字段（权重 2.0）或 `summary` 字段（权重 1.0）
- **并且**系统如果 title 包含所有查询 tokens，给予额外奖励分（+3.0）
- **并且**系统如果 title 包含至少一半查询 tokens，给予部分奖励分（+1.0）
- **并且**系统将分数归一化到 BM25 量级（乘以 10.0 系数）
- **并且**系统返回按分数降序排列的 `(document_id, score)` 元组列表

#### Scenario: BM25 搜索集成（测试环境）

- **当**评估脚手架初始化测试索引时
- **则**系统创建内存中的 Tantivy 索引（避免磁盘 I/O）
- **并且**系统使用现有 `Indexer` 将所有模拟文档添加到索引
- **并且**系统调用 `Indexer::commit()` 确保文档可搜索
- **当**评估脚手架调用 `bm25_search(query, limit)` 时
- **则**系统使用现有 `Searcher::search()` 执行 BM25 查询
- **并且**系统返回按 BM25 分数降序排列的 `(document_id, score)` 元组列表

#### Scenario: 评估指标计算

- **当**评估脚手架计算单个查询的 `accuracy_at_k(results, ground_truth, k)` 时
- **则**系统检查 Top-K 结果中是否有任何 Ground Truth 文档
- **并且**返回布尔值：有命中则为 1.0，否则为 0.0
- **当**评估脚手架计算 `ndcg_at_k(results, ground_truth, k)` 时
- **则**系统使用标准 NDCG 公式：`DCG / IDCG`
- **其中** `DCG = sum(relevance_i / log2(position + 1))`（relevance_i 为 1 if 在 ground truth 中 else 0，position 从 1 开始）
- **并且**`IDCG` 假设理想排序下所有相关文档排在最前面
- **并且**返回归一化的 NDCG 分数（0.0 到 1.0）
- **当**评估脚手架计算 `hit_rate_at_k(results, ground_truth, k)` 时
- **则**系统返回 Top-K 中是否有任何 Ground Truth 文档的布尔值（1.0 或 0.0）

#### Scenario: 测试报告生成格式

- **当**评估脚手架生成 `docs/BM25_EVALUATION_REPORT.md` 时
- **则**报告必须包含以下章节：
  - **摘要部分**：BM25 vs M1 的整体对比（Accuracy@3、NDCG@3、Hit Rate@3）
  - **详细对比表**：每个查询的 Top-3 结果对比（M1 结果、BM25 结果、Ground Truth）
  - **指标分析**：BM25 相比 M1 的改进百分比（如 "BM25 Accuracy 比 M1 提升 45%"）
  - **失败案例分析**：列出 BM25 表现不如 M1 的查询（如有）
- **并且**报告使用 Markdown 表格格式，方便在 GitHub 上渲染
- **并且**报告包含测试运行时间戳和文档数量信息

#### Scenario: 质量门禁断言

- **当**评估测试完成所有查询和指标计算后
- **则**系统断言 `bm25_accuracy >= m1_accuracy - 0.05`（BM25 Top-3 准确率应在 M1 基线的 5% 容差内）
- **并且**系统断言 `bm25_accuracy >= 0.70`（BM25 Top-3 准确率必须达到 70%）
- **如果**任一断言失败，测试 panic 并显示详细指标对比
- **并且**系统无论如何都会生成报告文件供人工审查
