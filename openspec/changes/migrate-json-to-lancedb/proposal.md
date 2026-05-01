# Change: JSON 数据迁移到 LanceDB

## Why

当前 Contextfy/Kit 项目已经完成了 LanceDB 基础设施集成（`add-lancedb-dependency`），但现有用户数据仍以旧格式存储：

1. **历史包袱**：早期版本使用 JSON 文件缓存知识记录，存储在用户目录下的 `.contextfy/cache.json` 或类似路径
2. **数据割裂**：旧数据无法利用新的向量搜索能力，导致用户体验降级
3. **迁移需求**：需要一次性迁移工具，将 JSON 记录批量转换为 LanceDB 格式并生成向量索引

本次变更实现数据迁移管道，确保用户平滑过渡到新存储架构，零数据丢失。

## What Changes

### 1. 迁移模块架构

创建 `packages/core/src/migration/mod.rs`，包含以下核心组件：

```rust
/// 从 JSON 迁移到 LanceDB 的主入口函数
pub async fn migrate_json_to_lancedb(config: MigrationConfig) -> Result<MigrationStats>

/// 迁移配置
pub struct MigrationConfig {
    pub json_path: PathBuf,           // JSON 文件或目录路径
    pub lancedb_uri: String,           // LanceDB 连接 URI
    pub table_name: String,            // 目标表名
    pub batch_size: usize,             // 批处理大小（默认 100）
    pub skip_errors: bool,             // 遇到错误是否跳过（默认 false）
}

/// 迁移统计信息
pub struct MigrationStats {
    pub total_processed: usize,        // 处理的总记录数
    pub successful: usize,             // 成功迁移的记录数
    pub failed: usize,                 // 失败的记录数
    pub skipped: usize,                // 跳过的记录数
}
```

### 2. 批处理 Embedding 生成

实现高效的批量向量化流程：

```rust
/// 批量生成 Embedding（减少 FastEmbed 调用次数）
async fn generate_embeddings_batch(
    embedding_svc: &EmbeddingService,
    texts: Vec<String>,
    batch_size: usize
) -> Result<Vec<Vec<f32>>>
```

**关键优化**：
- 每 100 条文本聚合为一个 batch
- 单次 FastEmbed API 调用处理整个 batch
- 避免逐条生成导致的性能瓶颈

### 3. 容错与恢复机制

实现健壮的错误处理策略：

```rust
/// 处理单条 JSON 记录的迁移（带容错）
async fn migrate_record(
    record: JsonRecord,
    lancedb: &mut LanceDbTable,
    embeddings: &mut Vec<Vec<f32>>
) -> Result<MigrateResult>

enum MigrateResult {
    Success(String),      // 成功迁移，返回记录 ID
    Skipped(String),      // 跳过（重复/无效），返回原因
    Failed(String, Error) // 失败，返回错误信息
}
```

**容错策略**：
- **损坏的 JSON 文件**：记录警告日志，跳过该文件，继续处理
- **缺失必填字段**：记录错误，根据 `skip_errors` 配置决定是否继续
- **Embedding 生成失败**：重试 1 次，失败后跳过该记录
- **LanceDB 插入失败**：回滚当前 batch，记录错误

### 4. CLI 入口设计

**方案 A（推荐）**：隐藏的 CLI 子命令
```rust
// packages/cli/src/commands/migrate.rs
pub async fn run_migrate_command(args: MigrateArgs) -> Result<()> {
    // 1. 读取配置文件或命令行参数
    // 2. 调用 migration::migrate_json_to_lancedb()
    // 3. 打印迁移统计和进度条
}

// 使用方式：
// kit migrate --json ~/.contextfy/cache.json --lancedb ~/.contextfy/db
// kit migrate --batch-size 50 --skip-errors
```

**方案 B**：独立的迁移模块
- 将迁移逻辑放在 `packages/core/src/migration/`
- 供其他工具或测试直接调用

**最终建议**：同时实现两者（CLI 作为便利入口，模块化设计便于测试）

### 5. 数据完整性验证

迁移后自动验证：

```rust
/// 验证迁移后的数据完整性
pub async fn validate_migration(
    lancedb_uri: &str,
    table_name: &str,
    expected_count: usize
) -> Result<ValidationReport>
```

**验证项**：
- 记录总数匹配
- 所有记录包含非空向量（384 维）
- 抽样 10% 记录，检查字段完整性

## Impact

- **Affected specs**: core-engine (migration capability)
- **Affected code**:
  - `packages/core/src/migration/**` (新建模块)
  - `packages/core/src/embeddings/mod.rs` (添加 batch embedding 方法)
  - `packages/cli/src/commands/migrate.rs` (新增 CLI 命令)
  - `packages/core/Cargo.toml` (添加 `indicatif` 进度条依赖)
- **BREAKING**: None（纯新增功能，不修改现有 API）

## Migration Strategy

**阶段 1：备份（可选）**
- 在迁移前自动创建 JSON 文件的备份（`.bak` 后缀）

**阶段 2：批量迁移**
- 按配置的 batch_size 读取 JSON 记录
- 批量生成 Embedding
- 批量插入 LanceDB

**阶段 3：验证与清理**
- 运行数据完整性检查
- 打印迁移报告（成功/失败/跳过数量）
- 用户确认后可删除旧 JSON 文件

**回滚策略**：
- 保留原始 JSON 文件直到用户手动删除
- 提供验证失败时的回滚提示

## Performance Considerations

**预期性能**：
- **Embedding 生成**：~100ms/batch（100 条记录）
- **LanceDB 插入**：~50ms/batch（100 条记录）
- **总吞吐量**：约 400-600 记录/秒（取决于硬件）

**优化点**：
- 使用 `tokio::spawn` 并行处理多个 batch（限制并发数为 4）
- 流式读取 JSON，避免一次性加载大文件到内存
- 使用 `indicatif` 显示进度条和 ETA

## Testing Strategy

**单元测试**：
- Mock EmbeddingService 和 LanceDB 连接
- 测试容错逻辑（损坏的 JSON、缺失字段）
- 测试批处理边界（batch_size = 1, 100, 1000）

**集成测试**：
- 使用临时目录创建测试 JSON 文件
- 运行完整迁移流程
- 验证 LanceDB 中的数据正确性

**端到端测试**：
- 使用真实 JSON 文件（~1000 条记录）
- 测量迁移时间和内存占用
- 验证向量搜索结果质量
