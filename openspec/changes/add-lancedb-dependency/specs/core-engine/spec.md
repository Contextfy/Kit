# core-engine Specification Deltas

## ADDED Requirements

### Requirement: LanceDB 依赖集成

The core engine SHALL include LanceDB and Arrow dependencies to enable vector database connectivity and zero-copy read operations. 核心引擎 SHALL 包含 LanceDB 和 Arrow 依赖以启用向量数据库连接和零拷贝读取操作。

#### Scenario: 添加 LanceDB 依赖到 Cargo.toml

- **当**开发者在 `packages/core/Cargo.toml` 中添加 LanceDB 相关依赖时
- **则**系统包含 `lancedb = "0.26.2"` 或更高稳定版本
- **并且**系统包含 `arrow` 依赖（使用工作空间版本 `=57`）
- **并且**系统可能包含 `arrow-schema` 和 `arrow-array`（如果不在主 arrow crate 中）
- **并且**所有依赖版本与工作空间定义兼容
- **并且**`cargo build` 成功编译无版本冲突

#### Scenario: 验证依赖编译成功

- **当**开发者执行 `cargo build --package contextfy-core` 时
- **则**LanceDB 和 Arrow 依赖被成功解析和下载
- **并且**C++ 标准库依赖正确链接
- **并且**编译成功无依赖冲突错误
- **并且**二进制大小增加不超过 10MB

### Requirement: LanceDB Schema 定义

The core engine SHALL define an Apache Arrow schema for knowledge records compatible with LanceDB storage format. 核心引擎 SHALL 定义与 LanceDB 存储格式兼容的 Apache Arrow 知识记录 Schema。

#### Scenario: 定义 Arrow Schema 字段

- **当**系统初始化 LanceDB Schema 时
- **则**Schema 必须包含以下字段：
  - `id`: DataType::Utf8 (non-nullable) - 记录的唯一标识符
  - `title`: DataType::Utf8 (non-nullable) - 记录标题
  - `summary`: DataType::Utf8 (non-nullable) - 内容摘要（用于 Scout 检索）
  - `content`: DataType::Utf8 (non-nullable) - 完整内容（用于 Inspect 检索）
  - `vector`: DataType::FixedSizeList(Float32, 384) (non-nullable) - 向量嵌入
  - `keywords`: DataType::Utf8 (nullable) - JSON 序列化的关键词数组
  - `source_path`: DataType::Utf8 (non-nullable) - 原始文件路径
- **并且**Schema 可被实例化和打印用于调试
- **并且**Schema 定义与 `KnowledgeRecord` 结构字段对应

#### Scenario: 验证向量字段维度

- **当**系统查询 Schema 中的 `vector` 字段时
- **则**字段类型为 `DataType::FixedSizeList`
- **并且**列表维度严格等于 384（BGE-small-en 模型输出维度）
- **并且**元素类型为 `DataType::Float32`

### Requirement: LanceDB 连通性脚手架

The core engine SHALL provide basic LanceDB connectivity functions for database initialization and table creation. 核心引擎 SHALL 提供基础 LanceDB 连接函数用于数据库初始化和表创建。

#### Scenario: 连接到 LanceDB 数据库

- **当**系统调用 `connect_lancedb(uri)` 时
- **则**系统使用 `lancedb::connect(uri).await` 建立连接
- **并且**系统返回 `Result<Connection>` 表示连接成功或失败
- **如果**连接失败，返回描述性错误信息（包含 URI 和失败原因）
- **并且**系统使用 `anyhow::Context` 添加错误上下文

#### Scenario: 创建表（如果不存在）

- **当**系统调用 `create_table_if_not_exists(conn, table_name)` 时
- **则**系统尝试打开现有表：`conn.open_table(table_name)`
- **如果**表已存在，返回成功不创建新表
- **如果**表不存在，使用预定义 Schema 创建新表：`conn.create_table(table_name, schema)`
- **并且**系统返回 `Result<()>` 表示操作成功或失败

#### Scenario: 初始化 LanceDB 数据库（辅助函数）

- **当**系统调用 `initialize_lancedb_db(uri, table_name)` 时
- **则**系统连接到指定 URI 的数据库
- **并且**系统创建表（如果不存在）
- **并且**系统返回 `Result<()>` 表示整体初始化成功或失败
- **并且**该函数为公共 API 供外部使用

### Requirement: LanceDB 连通性单元测试

The core engine SHALL include unit tests to verify LanceDB connectivity and schema correctness using temporary directories. 核心引擎 SHALL 包含单元测试以验证 LanceDB 连通性和 Schema 正确性，使用临时目录。

#### Scenario: 测试数据库连接和表创建

- **当**开发者运行 `cargo test lancedb` 时
- **则**系统创建临时目录（使用 `tempfile::tempdir()`）
- **并且**系统连接到临时目录的 LanceDB 实例
- **并且**系统创建测试表并验证 Schema
- **并且**系统断言 Schema 包含 7 个字段
- **并且**系统断言 `id` 字段类型为 `DataType::Utf8`
- **并且**系统断言 `vector` 字段为 384 维 `FixedSizeList`
- **并且**测试完成后临时目录被自动清理

#### Scenario: 验证测试在 10 秒内完成

- **当**系统执行 LanceDB 连通性测试时
- **则**测试应在 10 秒内完成
- **并且**包括连接、表创建、Schema 验证、清理的全过程

### Requirement: LanceDB 模块导出

The core engine SHALL export the LanceDB storage module through lib.rs for future integration work. 核心引擎 SHALL 通过 lib.rs 导出 LanceDB 存储模块供未来集成工作使用。

#### Scenario: 导出 storage 模块

- **当**外部代码使用 `contextfy_core::storage` 时
- **则**模块必须公开可访问
- **并且**`lancedb_store.rs` 中的公共函数可被调用
- **并且**Schema 定义函数可被访问

#### Scenario: 模块在 storage/mod.rs 中注册

- **当** `contextfy-core` crate 被编译时
- **则**`storage` 模块包含 `pub mod lancedb_store;` 声明
- **并且**模块的公共 API 可被依赖该 crate 的代码使用
