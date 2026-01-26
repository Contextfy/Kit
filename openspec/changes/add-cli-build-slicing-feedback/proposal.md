# 变更：添加 CLI 构建 切片统计和进度反馈

## 为什么

CLI `build` 命令当前支持文档切片处理，但缺乏清晰的用户反馈。用户无法查看正在处理的文档和切片总数，且缺少单个切片的进度跟踪。这使得理解构建范围和跟踪进度变得困难，特别是对于大型文档集。

## 变更内容

- **增强的构建命令**：更新 `contextfy build` 以在完成时显示摘要统计
- **进度反馈**：为每个正在处理的切片添加进度指示器
- **统计输出**：在构建结束时显示 "找到 X 个文档，Y 个切片"
- **计数器**：在整个构建过程中跟踪文档总数和切片总数

## 影响范围

- 受影响的规格：`cli`（MODIFIED - 扩展现有功能）
- 受影响的代码：`packages/cli/src/commands/build.rs`（在 refactor-cli-commands-structure 完成后）
- 无破坏性变更 - 纯粹的增强功能

## 依赖

此变更依赖 `refactor-cli-commands-structure` 变更完成。重构完成后，build 命令将位于 `commands/build.rs` 中。
