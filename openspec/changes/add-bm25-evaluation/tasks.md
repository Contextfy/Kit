# 实现任务清单

## 1. 准备工作

- [x] 1.1 在 `packages/core/tests/evaluation_test.rs` 创建评估测试文件结构
- [x] 1.2 复习 `packages/core/src/search/mod.rs` 中的现有 BM25 实现
- [x] 1.3 复习 `packages/core/src/storage/mod.rs:search()` 中的朴素匹配回退逻辑

## 2. 模拟数据集构建

- [x] 2.1 定义 18 个模拟 `KnowledgeRecord` 文档，模拟 Minecraft 模组开发内容
  - 包含带有 `createItem()`、`BlockCustomComponent` 等 API 代码块的文档
  - 混合技术文档和说明性散文
  - 确保覆盖：精确 API 名称、模糊概念、中文内容、边界情况
- [x] 2.2 为每个模拟文档分配唯一 ID，用于标准答案引用

## 3. 查询集合和标准答案定义

- [x] 3.1 定义 10 个代表性测试查询：
  - Q1: 精确 API 名称搜索（如 "createItem"）
  - Q2: 另一个精确 API 名称（如 "BlockCustomComponent"）
  - Q3: 模糊意图搜索（如 "how to register block"）
  - Q4: 概念搜索（如 "event handling"）
  - Q5: 中文分词测试（如 "方块"）
  - Q6: 多关键词搜索（如 "block register component"）
  - Q7: 代码特定搜索（如 "MinecraftBlockComponent"）
  - Q8: 动作导向搜索（如 "define custom item"）
  - Q9: 部分 API 名称（如 "create"）
  - Q10: 中文概念（如 "物品"）
- [x] 3.2 为每个查询手动定义预期的 Top-3 相关文档 ID（标准答案）

## 4. 基线 M1 实现（朴素匹配）

- [x] 4.1 实现 `naive_match_search(query: &str, documents: &[KnowledgeRecord]) -> Vec<(String, f32)>`
  - 使用基于空格的分词（按空白字符分割）
  - 检查 token 在 title（权重 2.0）和 summary（权重 1.0）中的存在性
  - 对 title 中完全匹配 tokens 的情况应用奖励分
  - 返回排序后的 (document_id, score) 元组列表
- [x] 4.2 确保评分归一化到 BM25 范围（乘以 10.0，如 storage 回退逻辑）

## 5. BM25 搜索集成

- [x] 5.1 为评估设置内存中的 Tantivy 索引
- [x] 5.2 使用现有 `Indexer` 索引所有模拟文档
- [x] 5.3 创建 `Searcher` 并实现 `bm25_search(query: &str, limit: usize) -> Vec<(String, f32)>`
  - 使用搜索模块中的现有 BM25 实现
  - 返回 (document_id, score) 元组以便比较

## 6. 评估指标实现

- [x] 6.1 实现指标计算函数：
  - `accuracy_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32`
    - 衡量：至少有一个标准答案文档出现在 Top-K 中的查询百分比
  - `ndcg_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32`
    - 衡量：归一化折损累积增益，考虑位置因素
    - **关键修复**: 使用 `(position + 1.0).log2()` 避免 `log2(1)=0` 除零错误
  - `hit_rate_at_k(results: &[String], ground_truth: &[&str], k: usize) -> f32`
    - 衡量：是否有任何标准答案文档出现在 Top-K 中
- [x] 6.2 实现评估主循环
  - 通过两个引擎运行所有 10 个查询
  - 收集每个查询的 Top-3 结果
  - 计算每个查询的指标和聚合平均值

## 7. 测试报告生成

- [x] 7.1 定义 `EvaluationReport` 结构体，包含：
  - 每个查询的结果：查询字符串、M1 Top-3、BM25 Top-3、标准答案、指标
  - 聚合指标：平均 accuracy@3、NDCG@3、hit rate@3
  - 改进百分比：((BM25 - M1) / M1) * 100
- [x] 7.2 实现 `generate_report(report: &EvaluationReport) -> String`
  - 格式化为 markdown，包含表格和章节
  - 包含详细的每个查询 A/B 对比
  - 突出显示 BM25 的改进和任何回归
- [x] 7.3 在测试中，将报告写入 `docs/BM25_EVALUATION_REPORT.md`

## 8. 集成测试实现

- [x] 8.1 创建主测试函数 `#[test] fn test_bm25_evaluation()`
- [x] 8.2 设置模拟文档和 Tantivy 索引
- [x] 8.3 运行两个搜索引擎（M1 朴素匹配和 BM25）
- [x] 8.4 生成并保存报告到磁盘（docs/ 目录）
- [x] 8.5 断言 BM25 Top-3 准确率 > 70%（质量门禁）
- [x] 8.6 打印摘要到控制台以便立即反馈

## 9. 质量保证

- [x] 9.1 运行 `cargo fmt` 并验证格式化
- [x] 9.2 运行 `cargo clippy` 并修复所有警告
- [x] 9.3 运行 `cargo test --test evaluation_test` 并确保通过
- [x] 9.4 验证 `docs/BM25_EVALUATION_REPORT.md` 生成且内容正确
- [x] 9.5 人工审查报告，确保 BM25 相比 M1 显示有意义的改进

## 实际结果

✅ **所有任务已完成**

**测试结果**:
- BM25 Top-3 准确率: 90.0%（质量门禁 70% ✅）
- BM25 NDCG@3: 0.693（比 M1 提升 6.3% ✅）
- 测试覆盖: 10 个查询 × 18 篇模拟文档
- 代码行数: 1278 行（evaluation_test.rs）

**质量保证**:
- ✅ `cargo fmt` 通过
- ✅ `cargo clippy` 通过（修复了单字符字符串字面量警告）
- ✅ `cargo test --test evaluation_test` 通过
- ✅ 报告生成在 `docs/BM25_EVALUATION_REPORT.md`
