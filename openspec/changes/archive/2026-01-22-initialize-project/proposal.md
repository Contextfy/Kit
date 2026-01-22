# 变更：使用最小可用组件初始化项目

## 原因
Contextfy/Kit 项目目前拥有完整的文档（PRD、架构、MVP），但没有实际实现。此变更创建基础项目结构，包含所有核心组件的最小"helloworld"实现，这些组件可以端到端地连接在一起，在深入复杂功能之前验证架构决策。

## 变更内容
- 创建包含三个 crate 的 Rust 工作空间：`core`、`bridge` 和 `server`
- 实现最小化的 CLI，包含 `init`、`build`、`scaffold` 命令
- 创建一个简单的 markdown 解析器，用于读取和索引 .md 文件
- 实现具有简单 schema 的基本 LanceDB 存储
- 使用 Next.js 添加基础 web 仪表盘
- 为 Node.js 设置 FFI 绑定（存根实现）
- 确保所有组件能够执行简单的端到端流程：解析 markdown 文件 → 存储到 LanceDB → 通过 CLI 检索 → 在 web UI 中显示

## 影响
- **影响的需求**：新能力（无现有需求）
- **影响的代码**：`packages/` 目录中的所有新代码
- **破坏性变更**：无（新项目）
- **依赖项**：添加 Rust、LanceDB、Next.js、napi-rs 作为项目依赖
