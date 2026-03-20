# Change: LanceDB 依赖集成

## Why

当前 Contextfy/Kit 实现依赖内存 HashMap 缓存和 JSON 文件存储，这带来了三个根本性限制：

1. **扩展性瓶颈**：启动时必须将所有记录加载到内存中，在大型知识库（100MB+ 文档）场景下导致 OOM 风险
2. **零拷贝原则违背**：当前从 JSON 反序列化到 Rust 结构体会产生不必要的内存拷贝，违背了项目"Zero-Copy Read"架构原则
3. **缺乏向量原生存储**：虽然 `KnowledgeRecord` 中存在 `embedding` 字段，但它们作为普通 JSON 数组存储，无法进行高效向量相似度搜索

LanceDB 提供：
- **Arrow 原生存储**：使用 Apache Arrow 格式实现零拷贝读取
- **内置向量搜索**：原生 HNSW 索引，支持 <20ms 语义检索
- **磁盘操作**：无需加载全部数据集即可高效访问
- **Rust 优先**：官方 Rust SDK，支持 async/await

本次变更为基础架构设施，为未来向量搜索优化（Issue #21）铺路，不修改现有业务逻辑。

## What Changes

### 1. 依赖集成

添加到 `packages/core/Cargo.toml`：
```toml
lancedb = "0.26.2"
arrow = { workspace = true }  # 已在工作空间中，使用版本 57
```

### 2. LanceDB Schema 定义

创建 `packages/core/src/storage/lancedb_store.rs`，使用 Arrow Schema 定义：
- `id`：String（主键）
- `title`：String
- `summary`：String（用于 Scout 检索）
- `content`：String（用于 Inspect 检索）
- `vector`：FixedSizeList<Float32>（384 维，BGE-small-en 模型输出）
- `keywords`：String（JSON 序列化数组，或 List<String> 如果 LanceDB 支持）
- `source_path`：String（用于源文件追溯）

### 3. 连通性脚手架

实现基础初始化函数：
```rust
pub async fn connect_lancedb(uri: &str) -> Result<Connection>
pub async fn create_table_if_not_exists(conn: &Connection, table_name: &str) -> Result<()>
```

### 4. 单元测试验证

在 `packages/core/src/storage/lancedb_store.rs` 中创建 `#[tokio::test]`：
- 连接到临时本地目录（使用 `tempfile::tempdir()`）
- 使用定义的 Schema 创建表
- 验证 Schema 正确加载
- 清理临时目录

**范围限制**：
- 不修改现有 `KnowledgeStore` 业务逻辑
- 不替换现有 JSON/Tantivy 存储层
- 不集成现有搜索流程
- 纯基础设施脚手架

**BREAKING**: None（纯基础设施添加，不修改现有代码）

## Impact

- Affected specs: core-engine
- Affected code:
  - packages/core/Cargo.toml（添加依赖）
  - packages/core/src/storage/mod.rs（添加模块声明）
  - packages/core/src/storage/lancedb_store.rs（新建文件）
