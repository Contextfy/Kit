## ADDED Requirements

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
