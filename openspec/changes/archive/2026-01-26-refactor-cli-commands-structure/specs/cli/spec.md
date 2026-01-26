# CLI 规格

## 目的
CLI 规格定义了 Contextfy Kit 的命令行界面需求，包括 build、scout 和其他面向用户的命令。

## ADDED Requirements

### Requirement: 命令模块化结构
CLI SHALL 使用模块化目录结构组织命令代码，每个命令作为独立模块。

#### Scenario: 命令目录结构
- **当**开发者查看 `packages/cli/src/` 目录时
- **则**系统应包含：
  - `main.rs` - CLI 入口和命令分发
  - `commands/` - 命令实现目录
    - `mod.rs` - 命令模块入口
    - `init.rs` - init 命令实现
    - `build.rs` - build 命令实现
    - `scout.rs` - scout 命令实现
    - `serve.rs` - serve 命令实现

#### Scenario: 命令模块职责分离
- **当**命令被分发执行时
- **则**每个命令模块 SHALL：
  - 包含该命令的所有实现逻辑
  - 不包含其他命令的代码
  - 定义命令执行函数或结构体
  - 可导出公共类型供其他模块使用

#### Scenario: main.rs 简化
- **当**查看 `main.rs` 文件时
- **则**文件 SHALL：
  - 定义 `Cli` 和 `Commands` 结构体（clap 配置）
  - 实现命令分发逻辑（match 语句）
  - 导入并调用相应的命令模块
  - 不包含具体的命令实现逻辑

#### Scenario: 新增命令扩展性
- **当**开发者需要添加新命令时
- **则**系统 SHALL：
  - 在 `commands/` 目录创建新文件（如 `export.rs`）
  - 在 `commands/mod.rs` 中导出新模块
  - 在 `Commands` 枚举中添加新变体
  - 在 `main.rs` 的 match 语句中添加分发逻辑
  - 不需要修改其他命令的代码
