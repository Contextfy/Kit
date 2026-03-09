# Change: BM25 检索效果评估

## Why

BM25 全文搜索实现（Issue #5）已完成并集成到核心引擎中。但是，我们缺乏量化数据来证明 BM25 相比旧的朴素文本匹配实现（M1）确实提供了更好的搜索质量。本次评估将：

1. 提供 BM25 有效性的实证数据
2. 为未来搜索改进建立基线
3. 生成可复现的测试报告，供文档参考

## What Changes

- **添加评估脚手架模块**：创建 `packages/core/tests/evaluation_test.rs`，包含 A/B 测试逻辑
- **创建模拟数据集**：硬编码 22 篇 Minecraft 模组开发文档（混合代码块和散文）
- **定义查询集合**：实现 13 个代表性测试查询（精确 API 搜索、多词查询、TF词频场景、长度惩罚场景）
- **建立标准答案**：为每个查询手动指定预期的文档 ID
- **实现评估指标**：计算 Top-3 结果的 Accuracy、NDCG@3 和 Hit Rate
- **生成测试报告**：自动生成 `docs/BM25_EVALUATION_REPORT.md`，包含详细的 A/B 对比
- **质量门禁**：
  - 动态门禁：BM25 Accuracy 必须在 M1 基线的 5% 容差内
  - 静态门禁：BM25 Top-3 准确率必须达到 70%

**BREAKING**: None

## Impact

- **受影响的 spec**：`core-engine`（ADDED: BM25 Search Evaluation）
- **受影响的代码**：
  - `packages/core/tests/evaluation_test.rs`（新建文件）- A/B 测试脚手架和模拟数据
  - `packages/core/src/search/mod.rs` - 引用现有 BM25 实现
  - `packages/core/src/parser/mod.rs` - 恢复 `new`, `map`, `range` 到全局黑名单
  - `docs/BM25_EVALUATION_REPORT.md`（生成文件）- 测试报告

- **性能影响**：无（评估是独立的集成测试）
- **兼容性**：无破坏性变更；评估是可选项，通过 `cargo test --test evaluation_test` 运行

## 实际结果

✅ **所有任务已完成**

**测试结果**:
- BM25 Top-3 准确率: 100.0%（与 M1 持平 ✅）
- BM25 NDCG@3: 0.709（比 M1 提升 7.5% ✅）
- 测试覆盖: 13 个查询 × 22 篇模拟文档
- 代码行数: 约 1700 行（evaluation_test.rs）

**质量保证**:
- ✅ `cargo fmt` 通过
- ✅ `cargo clippy` 通过
- ✅ `cargo test --test evaluation_test` 通过
- ✅ 报告生成在 `docs/BM25_EVALUATION_REPORT.md`
