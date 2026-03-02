# 实现任务

## 1. 核心引擎 - 搜索优化

- [x] 1.1 在 `KnowledgeStore::search()` 中实现基于分词的匹配
  - 将查询按空格分割为 tokens
  - 根据 token 命中数计算匹配分数（title 权重 2，summary 权重 1）
  - 按相关性降序排序结果（添加完全匹配和部分匹配奖励）

## 2. 验证脚本

- [x] 2.1 创建 `scripts/run_mvp_scout_tests.sh`
  - 硬编码 5 个测试查询（必须一字不差）：
    1. "create custom block"
    2. "player health"
    3. "spawn entity"
    4. "dimension API"
    5. "item registration"
  - 对每个查询调用 `cargo run -p contextfy-cli -- scout "<query>"`
  - 捕获并解析输出，提取 Top-3 结果
  - 计算 Top-1、Top-3 准确率
  - 生成 markdown 报告到 `docs/MVP_VALIDATION_REPORT.md`

## 3. 文档

- [x] 3.1 生成验证报告（由脚本自动生成）
  - 包含：测试查询、真实 scout 输出、Top-3 结果
  - 准确率指标：Top-1 (4/5 达标)、Top-3 (4/5)
  - 结论部分：
    - 对齐验收标准（明确写明 4/5 是否达标）
    - 分析当前算法的局限性（分词匹配的脆弱性）
    - 明确建议引入 Tantivy 全文检索 + BM25 算法

## 4. 测试与验证

- [x] 4.1 运行脚本 `bash scripts/run_mvp_scout_tests.sh`
- [x] 4.2 验证报告生成在 `docs/MVP_VALIDATION_REPORT.md`
- [x] 4.3 确认 Top-1 准确率 ≥ 60%（实际 80%，4/5 正确）
- [x] 4.4 运行 `cargo fmt` 和 `cargo clippy`
- [x] 4.5 手动检查报告内容真实性（基于实际 scout 输出）
