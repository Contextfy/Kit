# 变更：重构 CLI 命令结构

## 为什么

当前 CLI 将所有命令逻辑（init、build、scout、serve）都放在 `main.rs` 一个文件中，随着后续需要实现更多命令（如 inspect、export、import 等），这种结构会导致：
- `main.rs` 过于庞大，难以维护
- 新增命令时需要修改核心分发逻辑，违反单一职责原则
- 测试各个命令时需要加载整个 main.rs，不利于单元测试
- 命令间职责不清晰，代码复用困难

## 变更内容

- **创建 commands 目录**：在 `packages/cli/src/` 下创建 `commands/` 目录
- **命令模块化**：将每个命令拆分到独立文件：
  - `commands/mod.rs` - 命令模块入口和公共类型
  - `commands/init.rs` - init 命令实现
  - `commands/build.rs` - build 命令实现
  - `commands/scout.rs` - scout 命令实现
  - `commands/serve.rs` - serve 命令实现
- **简化 main.rs**：`main.rs` 仅保留 CLI 解析和命令分发逻辑
- **命令特征**：定义统一的命令执行接口，方便未来扩展

## 影响范围

- 受影响的规格：`cli`（ADDED - 新增命令模块化结构）
- 受影响的代码：`packages/cli/src/main.rs`，新增 `packages/cli/src/commands/`
- 破坏性变更：无 - 内部重构，用户接口不变
