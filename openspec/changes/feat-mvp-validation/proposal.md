# Change: MVP Baseline Validation

## Why

MVP 需要基准准确率测量来验证当前检索系统能否满足最低可用性标准（测试查询的 Top-1 准确率 ≥60%）。没有自动化验证，我们无法客观衡量检索质量或证明引入 BM25/全文检索改进的迫切需求。

## What Changes

- **Core**: 极小幅度的检索算法优化（基于分词的匹配替代简单的 `.contains()`），提高多词查询的命中率
- **Scripts**: 创建外部验证脚本 `scripts/run_mvp_scout_tests.sh`，通过调用现有的 `contextfy scout` 命令执行测试
- **Documentation**: 生成 `docs/MVP_VALIDATION_REPORT.md`，包含详细的准确率指标和发现

**BREAKING**: None - incremental feature

## Impact

- **Affected specs**: `core-engine` (搜索改进)
- **Affected code**:
  - `packages/core/src/storage/mod.rs` (搜索函数优化)
  - `scripts/run_mvp_scout_tests.sh` (新文件 - 外部脚本)
- **Dependencies**: 无（使用现有存储层和 CLI scout 命令）
