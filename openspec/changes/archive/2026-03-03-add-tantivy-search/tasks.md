# 实施任务清单

## 1. 依赖设置
- [x] 1.1 在 `packages/core/Cargo.toml` 中添加 `tantivy` 依赖（检查 workspace Cargo.toml 的版本约束）
- [x] 1.2 在 workspace 根目录运行 `cargo check` 验证依赖正确解析

## 2. 模块结构
- [x] 2.1 创建 `packages/core/src/search/` 目录
- [x] 2.2 创建 `packages/core/src/search/mod.rs` 文件，包含模块导出
- [x] 2.3 在 `packages/core/src/lib.rs` 中添加 `pub mod search;`
- [x] 2.4 运行 `cargo check` 验证模块编译通过

## 3. Schema 定义

- [x] 3.1 实现 `create_schema()` 函数，返回 Tantivy Schema
- [x] 3.2 在 Schema 中定义四个 TEXT 字段：`title`、`summary`、`content`、`keywords`
- [x] 3.3 配置字段为 TEXT 记录类型（分词 + 存储）
- [x] 3.4 添加单元测试 `test_schema_fields()` 验证所有四个字段存在且配置正确
- [x] 3.5 在 `packages/core/Cargo.toml` 中添加 `tantivy-jieba` 依赖
- [x] 3.6 初始化 `JiebaTokenizer` 并配置到所有 TEXT 字段
- [x] 3.7 在 `create_index()` 中注册 jieba 分词器

## 4. 索引初始化

- [x] 4.1 实现 `create_index()` 函数，支持可选的目录参数
- [x] 4.2 支持内存索引创建（不传目录参数）
- [x] 4.3 支持文件系统索引创建（传入目录路径参数）
- [x] 4.4 返回 Result<Index, Error> 以进行正确的错误处理
- [x] 4.5 添加单元测试 `test_create_in_memory_index()` 验证内存索引创建
- [x] 4.6 添加单元测试 `test_create_filesystem_index()` 使用 tempfile 验证持久化索引创建
- [x] 4.7 实现打开现有索引的功能（open_or_create 语义）
- [x] 4.8 使用 `anyhow::Context` 添加路径信息的错误提示

## 5. 验证与测试

- [x] 5.1 运行 `cargo build --package contextfy-core` 确保编译成功
- [x] 5.2 运行 `cargo test --package contextfy-core` 验证所有单元测试通过
- [x] 5.3 运行 `cargo clippy --package contextfy-core` 检查代码规范问题
- [x] 5.4 运行 `cargo fmt --package contextfy-core` 确保代码格式符合规范
- [x] 5.5 添加 `test_jieba_tokenizer_registered()` 测试验证分词器正确注册

## 6. 文档编写
- [x] 6.1 为 `create_schema()` 函数添加公共 API 文档（`///` 注释）
- [x] 6.2 为 `create_index()` 函数添加带示例的公共 API 文档
- [x] 6.3 添加模块级文档，说明 search 模块的目的和用法

## 验收标准确认
- [x] ✓ `cargo build` 成功，无编译错误
- [x] ✓ 单元测试成功在内存或临时目录中创建 Tantivy Index
- [x] ✓ 单元测试验证 Schema 包含所有 4 个必需字段（title、summary、content、keywords）
- [x] ✓ 所有测试通过：`cargo test --package contextfy-core`
- [x] ✓ 代码通过 clippy 检查：`cargo clippy --package contextfy-core`
- [x] ✓ 未修改 `storage/mod.rs`（架构红线已验证）
