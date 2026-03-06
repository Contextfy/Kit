## 1. 实现

- [ ] 1.1 修改 `packages/core/src/retriever/mod.rs` 中的 `Brief` 结构体，添加 `score: f32` 字段
- [ ] 1.2 修改 `packages/core/src/storage/mod.rs` 中的 `KnowledgeStore::search()` 返回分数信息
- [ ] 1.3 更新 `packages/core/src/retriever/mod.rs` 中的 `Retriever::scout()` 将分数传递给 Brief
- [ ] 1.4 更新 CLI `scout.rs` 使用格式 `"Score: {:.2} | [title] content"` 显示分数
- [ ] 1.5 运行 `cargo fmt`、`cargo clippy`、`cargo test` 确保代码质量
