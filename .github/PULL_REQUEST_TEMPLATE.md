## 变更说明
简要描述这个 PR 做了什么。

## 变更类型

- [ ] Bug 修复 (non-breaking change which fixes an issue)
- [ ] 新功能 (non-breaking change which adds functionality)
- [ ] 破坏性变更 (fix or feature that would cause existing functionality to not work as expected)
- [ ] 重构 (code change that neither fixes a bug nor adds a feature)
- [ ] 文档更新
- [ ] 性能优化
- [ ] 其他 (please describe)

## 📋 任务清单

- [ ] 我的代码遵循项目代码规范
- [ ] 我已执行 `cargo fmt` 格式化代码
- [ ] 我已执行 `cargo clippy` 检查代码
- [ ] 我已运行所有测试 (`cargo test`)
- [ ] 我已更新相关文档
- [ ] 我已为新增功能编写测试
- [ ] 我的变更不会引入新的警告

## 🧪 测试

描述如何测试这个变更：

```bash
# 测试步骤
cargo run --bin contextfy init
cargo run --bin contextfy build
cargo run --bin contextfy scout "test"
```

- [ ] 手动测试通过
- [ ] 单元测试通过
- [ ] 集成测试通过

## 📸 截图

如果适用，添加截图展示变更效果（UI 变更、性能提升等）。

## 📚 相关文档

- 相关 Issue: (关联的 Issue 编号)
- 相关 PR: (相关联的其他 PR)

## 📝 变更说明

详细列出主要的文件变更：

- `packages/core/src/parser/mod.rs`: 添加了 BM25 排序算法
- `packages/cli/src/main.rs`: 更新了 help 文本

## ⚠️ 破坏性变更

如果是破坏性变更，说明迁移指南：

```rust
// 旧 API
store.search(query).await?;

// 新 API
store.search(query, &SearchOptions::default()).await?;
```

## 💬 补充说明

任何需要 reviewer 特别注意的地方，或者待讨论的问题。
