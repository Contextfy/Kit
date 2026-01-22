---
name: 📋 Development Task
about: 开发者接手的开发任务（由项目维护者创建）
title: '[TASK] 模块名称 - 任务描述'
labels: 'area:core,type:enhancement,priority:medium'
assignees: ''
---

## 🎯 任务目标

一句话描述这个任务的目标。

## 📦 涉及模块

- [ ] `core-engine` - 核心 Rust 引擎
- [ ] `cli` - 命令行工具
- [ ] `server` - Web 服务器
- [ ] `bridge` - FFI 桥接层
- [ ] `web-dashboard` - Web 界面
- [ ] `docs` - 文档

## 📝 需求详情

详细描述需要实现的功能或修复的问题。

### 接受标准

- [ ] 功能实现完成
- [ ] 添加了必要的单元测试
- [ ] 代码通过 `cargo test`
- [ ] 代码通过 `cargo fmt` 和 `cargo clippy`
- [ ] 更新了相关文档

### 技术要求

- Rust 版本要求: `>= 1.75.0`
- 遵循现有的代码风格和架构模式
- 使用 `anyhow::Result` 作为错误类型
- 添加适当的注释

## 📚 相关资源

- 相关 OpenSpec 变更: (如果有)
- 相关 PR: #
- 参考实现: (链接到文档或类似代码)

## 🚫 不需要做的事

明确说明这个任务**不需要**做的事情，避免范围蔓延。

---

## ✅ 完成清单（接手者填写）

### 开发完成

- [ ] 代码实现
- [ ] 单元测试
- [ ] 文档更新
- [ ] 本地测试通过

### PR 提交前

- [ ] 代码格式化: `cargo fmt`
- [ ] 代码检查: `cargo clippy`
- [ ] 所有测试通过: `cargo test`
- [ ] PR 描述清晰，包含相关 Issue 链接

## 💬 讨论

遇到问题在这里讨论，维护者和其他开发者会提供帮助。
