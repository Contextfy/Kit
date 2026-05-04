# Tasks: LanceDB 依赖集成

## 任务列表

### 1. 添加依赖到 Cargo.toml

- [x] 在 `packages/core/Cargo.toml` 中取消注释 `lancedb` 依赖（当前第 11 行）
- [x] 在 `packages/core/Cargo.toml` 中取消注释 `arrow` 依赖（当前第 12 行）
- [x] 运行 `cargo check --package contextfy-core` 验证依赖可编译
- [x] 确认与工作空间现有 `arrow = "=57"` 无版本冲突

**依赖**：无前置任务

---

### 2. 创建 LanceDB Schema 定义文件

- [x] 创建新文件 `packages/core/src/storage/lancedb_store.rs`
- [x] 创建模块，导入 LanceDB 和 Arrow Schema 类型
- [x] 实现 `knowledge_record_schema()` 函数，返回 Arrow Schema
- [x] Schema 必须包含 7 个字段：id, title, summary, content, vector, keywords, source_path
- [x] `vector` 字段类型为 `DataType::FixedSizeList(Float32, 384)`
- [x] 在 `packages/core/src/storage/mod.rs` 中添加 `pub mod lancedb_store;` 声明
- [x] 验证代码编译无类型错误
- [x] 验证 Schema 可实例化并打印用于调试

**依赖**：任务 1（必须先添加依赖）

---

### 3. 实现连通性函数

- [x] 实现 `pub async fn connect_lancedb(uri: &str) -> Result<LanceConnection>`
  - [x] 使用 `lancedb::connect(uri).await` 建立连接
  - [x] 使用 `anyhow::Context` 处理连接错误
- [x] 实现 `pub async fn create_table_if_not_exists(conn: &LanceConnection, table_name: &str) -> Result<()>`
  - [x] 使用 `conn.table_names()` 获取现有表列表
  - [x] 检查表是否存在，不存在则创建
  - [x] 使用 `conn.create_empty_table(table_name, schema)` 创建新表
  - [x] 如果已存在，验证 Schema 并返回成功
- [x] 实现公共辅助函数 `pub async fn initialize_lancedb_db(uri: &str, table_name: &str) -> Result<()>`
- [x] 验证函数编译无错误
- [x] 验证错误处理使用 `anyhow::Result` 并覆盖所有 LanceDB SDK result 类型

**依赖**：任务 2（必须先定义 Schema）

---

### 4. 编写连通性单元测试

- [x] 在 `lancedb_store.rs` 中添加 `#[cfg(test)]` 模块
- [x] 创建测试用例 `test_initialize_lancedb_db_creates_table`
  - [x] 使用 `tempfile::tempdir()` 创建临时目录
  - [x] 调用 `initialize_lancedb_db(db_uri, table_name)` 初始化
  - [x] 连接到数据库并打开表
  - [x] 验证 Schema 包含 7 个字段
  - [x] 验证 `id` 字段类型为 `DataType::Utf8`
  - [x] 验证 `vector` 字段为 384 维 `FixedSizeList`
- [x] 创建幂等性测试 `test_initialize_lancedb_db_idempotent`
- [x] 添加向量字段维度验证测试
- [x] 验证测试在 10 秒内完成
- [x] 验证临时目录被正确清理

**依赖**：任务 3（必须先实现连通性函数）

---

### 5. 运行完整测试套件和 Clippy

- [x] 运行 `cargo test --package contextfy-core` 确保无回归
- [x] 运行 `cargo clippy --all-targets --all-features -- -D warnings` 检查警告
- [x] 运行 `cargo fmt --all --check` 验证格式化
- [x] 如有警告或格式问题，修复后重新验证

**依赖**：任务 1-4 全部完成

---

### 6. 验证构建成功

- [x] 运行 `cargo build --package contextfy-core`
- [x] 检查二进制大小增加 < 10MB
- [x] 验证首次构建时间可接受（< 2 分钟为佳）

**依赖**：任务 5（所有测试和检查必须通过）

---

## 成功标准

1. ✅ LanceDB 0.26.2 依赖与 Arrow 57 可编译
2. ✅ Schema 定义与 `KnowledgeRecord` 结构匹配
3. ✅ 单元测试连接到临时 DB 并验证 Schema
4. ✅ `cargo test` 通过，无回归
5. ✅ `cargo clippy` 无警告
6. ✅ Release 构建成功

## 风险与缓解

**风险**：

- LanceDB 的 C++ 依赖可能导致编译时间增加
- Arrow 版本冲突

**缓解措施**：

- 使用稳定版 LanceDB 0.26.2
- 固定 Arrow 版本为工作空间版本（=57）
- 优先在 Linux（主要目标平台）上测试
