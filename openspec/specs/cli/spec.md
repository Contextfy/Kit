# cli Specification

## Purpose
TBD - created by archiving change refactor-cli-commands-structure. Update Purpose after archive.
## Requirements
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

### Requirement: 可配置文档路径
CLI SHALL 支持通过 `contextfy.json` 配置文件指定文档源目录路径，使 build 命令能够处理不同项目结构的文档目录。

#### Scenario: 读取配置的文档路径
- **当**用户执行 `contextfy build` 命令且 `contextfy.json` 包含 `docs_path` 配置时
- **则**系统从配置文件读取 `docs_path` 字段作为文档源目录
- **并且**使用该目录扫描 Markdown 文件进行解析和存储

#### Scenario: 回退到默认路径
- **当**用户执行 `contextfy build` 命令且 `contextfy.json` 不存在或不包含 `docs_path` 字段时
- **则**系统回退到默认路径 `docs/examples` 作为文档源目录
- **并且**在控制台输出提示信息指明使用的路径

#### Scenario: contextfy.json 配置结构
- **当**系统读取 `contextfy.json` 配置文件时
- **则**配置文件应包含以下结构：
  ```json
  {
    "name": "项目名称",
    "version": "版本号",
    "description": "项目描述",
    "docs_path": "文档目录相对路径"
  }
  ```
- **并且**`docs_path` 字段为可选字段

#### Scenario: 配置路径不存在时错误提示
- **当**`contextfy.json` 中配置的 `docs_path` 指向不存在的目录时
- **则**系统返回描述性错误信息
- **并且**错误信息包含配置的路径和提示用户检查配置或运行 `contextfy init`

### Requirement: CLI scout 命令显示分数和颜色高亮
CLI SHALL display search results with BM25 relevance scores and terminal color highlighting. CLI SHALL 显示带有 BM25 相关性分数和终端颜色高亮的搜索结果。

#### Scenario: 显示带分数的搜索结果
- **当**用户执行 CLI scout 命令时
- **则**系统显示搜索结果，格式为：
  - `Score: {score:.2} | [parent_doc] section_title` - 对于切片文档
  - `Score: {score:.2} | document_title` - 对于非切片文档
  - `ID: {id}`
  - `Summary: {summary}`
- **并且**分数显示保留 2 位小数

