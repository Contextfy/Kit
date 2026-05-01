# 实施任务清单

## 1. 实现

- [x] 1.1 修改 `packages/core/src/retriever/mod.rs` 中的 `Brief` 结构体，添加 `score: f32` 字段
- [x] 1.2 修改 `packages/core/src/storage/mod.rs` 中的 `KnowledgeStore::search()` 返回分数信息
- [x] 1.3 更新 `packages/core/src/retriever/mod.rs` 中的 `Retriever::scout()` 将分数传递给 Brief
- [x] 1.4 更新 CLI `scout.rs` 使用格式 `"Score: {:.2} | [title] content"` 显示分数
- [x] 1.5 运行 `cargo fmt`、`cargo clippy`、`cargo test` 确保代码质量

## 2. 额外优化 (Nitpick Improvements)

- [x] 2.1 修复 Bridge 层的 Mock Drift：为 NAPI `Brief` 结构体添加 `parent_doc_title` 和 `score` 字段（使用 `f64` 类型以兼容 NAPI）
- [x] 2.2 CLI 终端颜色高亮：添加 `colored` 依赖，根据分数高低使用不同颜色（绿色/黄色/暗淡）
- [x] 2.3 修复 server 层 clippy 警告：移除不必要的显式解引用操作符
