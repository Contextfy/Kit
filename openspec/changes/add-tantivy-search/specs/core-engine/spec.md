## ADDED Requirements

### Requirement: Tantivy Index Schema Definition
The core engine SHALL define a Tantivy Schema for full-text search indexing with TEXT field configuration supporting tokenization for Chinese and English content. 核心引擎 SHALL 为全文搜索索引定义 Tantivy Schema，使用 TEXT 字段配置支持中英文内容的分词。

#### Scenario: 定义包含四个 TEXT 字段的 Schema
- **当**系统初始化 Tantivy Schema 时
- **则**Schema 必须包含以下字段，均配置为 TEXT 类型（支持分词）：
  - `title` - 用于索引文档标题
  - `summary` - 用于索引文档摘要
  - `content` - 用于索引文档完整内容
  - `keywords` - 用于索引文档关键词

#### Scenario: 验证 Schema 字段配置
- **当**系统查询 Schema 定义时
- **则**所有字段必须配置为 TEXT 记录类型（TEXT）
- **并且**字段必须启用分词支持（TOKENIZED）
- **并且**字段必须支持存储（STORED）以用于结果展示

### Requirement: Tantivy Index Initialization
The core engine SHALL provide a function to create and initialize a Tantivy Index with support for both in-memory and filesystem-backed storage modes. 核心引擎 SHALL 提供函数来创建和初始化 Tantivy 索引，支持内存和文件系统两种存储模式。

#### Scenario: 在内存中创建临时索引
- **当**系统调用索引创建函数且未指定目录路径时
- **则**系统在内存中创建临时 Tantivy Index
- **并且**Index 使用预定义的 Schema（包含 title, summary, content, keywords 字段）
- **并且**Index 可用于文档索引和搜索操作
- **并且**索引数据在程序退出后自动释放

#### Scenario: 在指定目录创建持久化索引
- **当**系统调用索引创建函数并指定有效目录路径时
- **则**系统在指定目录创建持久化 Tantivy Index
- **并且**Index 使用预定义的 Schema
- **并且**索引元数据和数据写入到指定目录
- **并且**目录已存在时可重新打开现有索引

#### Scenario: 处理无效目录路径
- **当**系统尝试在不存在的父目录下创建索引且父目录无法创建时
- **则**系统返回描述性错误
- **并且**错误信息包含具体的路径和失败原因

### Requirement: Tantivy Module Structure
The core engine SHALL expose a public `search` module containing the Tantivy Index initialization and Schema definition functions. 核心引擎 SHALL 公开 `search` 模块，包含 Tantivy 索引初始化和 Schema 定义函数。

#### Scenario: 导出 search 模块
- **当**外部代码使用 `contextfy_core::search` 时
- **则**模块必须公开可访问
- **并且**模块提供创建 Index 的公共函数
- **并且**模块提供获取或定义 Schema 的方法

#### Scenario: 集成到 core crate
- **当**用户编译 `contextfy-core` crate 时
- **则**`search` 模块包含在 `lib.rs` 的导出列表中
- **并且**外部依赖可通过 `use contextfy_core::search` 引入模块
