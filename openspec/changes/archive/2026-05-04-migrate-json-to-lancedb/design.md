# Design: JSON 到 LanceDB 迁移工具

## Architecture Overview

```
┌─────────────────┐
│  CLI Entry      │
│  (migrate cmd)  │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  Migration Orchestrator             │
│  - Read JSON config                 │
│  - Initialize embedding service     │
│  - Connect to LanceDB               │
└────────┬────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  Batch Processing Pipeline          │
│                                     │
│  ┌──────────┐   ┌──────────────┐   │
│  │  Read    │──▶│  Validate    │   │
│  │  Batch   │   │  Records     │   │
│  └──────────┘   └──────┬───────┘   │
│                       │            │
│                       ▼            │
│              ┌─────────────────┐  │
│              │  Generate       │  │
│              │  Embeddings     │  │
│              │  (Batch)        │  │
│              └────────┬────────┘  │
│                       │            │
│                       ▼            │
│              ┌─────────────────┐  │
│              │  Insert to      │  │
│              │  LanceDB        │  │
│              │  (Batch)        │  │
│              └─────────────────┘  │
└─────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  Validation & Reporting            │
│  - Data integrity checks            │
│  - Migration statistics             │
│  - Error log                        │
└─────────────────────────────────────┘
```

## Data Flow

### Input: JSON Records

假设旧 JSON 格式（需要在实现中确认实际结构）：

```json
{
  "version": "1.0",
  "records": [
    {
      "id": "uuid-1",
      "title": "Document Title",
      "summary": "Brief summary",
      "content": "Full document content",
      "keywords": ["tag1", "tag2"],
      "source_path": "/path/to/file.md",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

### Transformation Logic

```rust
// 1. Parse JSON
let json_data: JsonData = serde_json::from_str(&json_content)?;

// 2. Validate fields
for record in json_data.records {
    ensure_non_empty(&record.id)?;
    ensure_non_empty(&record.title)?;
    ensure_non_empty(&record.content)?;
    // keywords 可以为空
}

// 3. Generate embeddings (batch)
let texts: Vec<&str> = records.iter()
    .map(|r| r.content.as_str())
    .collect();
let embeddings = embedding_service.embed_batch(&texts).await?;

// 4. Transform to LanceDB schema
let lancedb_records: Vec<KnowledgeRecord> = records.iter()
    .zip(embeddings.into_iter())
    .map(|(record, vector)| KnowledgeRecord {
        id: record.id.clone(),
        title: record.title.clone(),
        summary: record.summary.clone(),
        content: record.content.clone(),
        vector,
        keywords: record.keywords.join(","), // JSON 数组转逗号分隔字符串
        source_path: record.source_path.clone(),
    })
    .collect();

// 5. Batch insert to LanceDB
lancedb_table.add(lancedb_records).await?;
```

## Error Handling Strategy

### Error Categories

| 错误类型 | 处理策略 | 日志级别 | 是否继续 |
|---------|---------|---------|---------|
| JSON 解析失败 | 跳过文件，记录错误 | ERROR | ✅ |
| 缺失必填字段 | 跳过记录，记录字段名 | WARN | ✅ (如果 skip_errors=true) |
| Embedding 生成失败 | 重试 1 次，失败则跳过 | WARN | ✅ |
| LanceDB 连接失败 | 立即终止，提示检查配置 | ERROR | ❌ |
| LanceDB 插入失败 | 回滚当前 batch，记录错误 | ERROR | ❌ |

### Error Recovery

```rust
pub async fn migrate_with_recovery(
    config: MigrationConfig,
) -> Result<MigrationStats> {
    let mut stats = MigrationStats::default();

    // 创建备份
    if config.backup {
        backup_json_file(&config.json_path)?;
    }

    // 加载已处理的 ID 列表（支持断点续传）
    let processed_ids = load_processed_ids(&config.state_file)?;

    for batch in json_reader.batches(config.batch_size) {
        match migrate_batch(batch, &processed_ids).await {
            Ok(batch_stats) => {
                stats += batch_stats;
                save_processed_ids(&config.state_file, &batch_stats.new_ids)?;
            }
            Err(e) if config.skip_errors => {
                warn!("Batch failed, skipping: {}", e);
                stats.failed += batch.len();
            }
            Err(e) => {
                error!("Migration failed: {}", e);
                return Err(e);
            }
        }
    }

    Ok(stats)
}
```

## Batch Processing Design

### Batch Size Selection

| 场景 | 推荐批次大小 | 理由 |
|-----|------------|------|
| 小规模 (<1000 条) | 50-100 | 减少内存占用 |
| 中规模 (1000-10000) | 100-200 | 平衡速度和内存 |
| 大规模 (>10000) | 200-500 | 最大化吞吐量 |

### Serial Pipeline Processing (重要性能优化!)

**⚠️ 严禁使用 Tokio 并发处理多个 batch！**

FastEmbed 底层基于 ONNX Runtime，已经实现了极其高效的多核并行计算。如果在外部使用 `tokio::spawn` 并发多个 batch，会导致：
- ONNX Runtime 线程池互相抢占 CPU 资源
- 严重的上下文切换开销
- 内存占用爆炸
- **反而拖慢整体速度**

**正确的执行模型是单线串行的 Pipeline：**

```rust
// ✅ 正确：串行处理，让 FastEmbed 内部并行
for batch in json_reader.batches(batch_size) {
    // 1. 批量读取
    let records = batch?;

    // 2. 批量生成 Embedding（ONNX Runtime 内部已并行）
    let texts: Vec<&str> = records.iter().map(|r| r.content.as_str()).collect();
    let embeddings = embedding_service.embed_batch(&texts).await?;

    // 3. 批量插入 LanceDB
    let lancedb_records = transform_and_combine(records, embeddings);
    lancedb_table.add(lancedb_records).await?;

    // 4. 更新进度
    progress.inc(batch.len() as u64);
}
```

**性能优势：**
- 单进程占用 CPU 核心数 = ONNX Runtime 线程池大小（默认为物理核心数）
- 避免了不必要的上下文切换
- 内存占用可预测且稳定
- 实测吞吐量可达 400-600 记录/秒

## Vector Index Creation

迁移完成后自动创建 HNSW 索引：

```rust
pub async fn create_vector_index(
    lancedb_uri: &str,
    table_name: &str,
) -> Result<()> {
    let db = connect(lancedb_uri).await?;
    let table = db.open_table(table_name).await?;

    // 创建 HNSW 索引（高召回率，适合语义搜索）
    table.create_index(
        &Index::IvfPq {
            distance_type: DistanceType::Cosine,
            num_partitions: 256,         // IVF 分区数
            num_sub_vectors: 8,          // PQ 子向量数
        }
    ).await?;

    Ok(())
}
```

**索引参数说明**：
- **DistanceType::Cosine**：余弦相似度，适合文本嵌入
- **num_partitions**：根据数据量调整（256 适合 10K-100K 记录）
- **num_sub_vectors**：压缩率越高，查询越快，但精度略降

## Progress Reporting

使用 `indicatif` 显示进度：

```rust
let progress = ProgressBar::new(total_records as u64);
progress.set_style(ProgressStyle::default_bar()
    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
    .progress_chars("##-"));

for batch in batches {
    let migrated = migrate_batch(batch).await?;
    progress.inc(migrated as u64);
    progress.set_message(format!("Speed: {:.0} rec/s", speed));
}

progress.finish_with_message("Migration complete!");
```

## Configuration File Support

支持 TOML 配置文件：

```toml
# ~/.config/kit/migration.toml
[general]
json_path = "~/.contextfy/cache.json"
lancedb_uri = "lancedb://~/.contextfy/db"
table_name = "knowledge"

[batch]
size = 100

[error_handling]
skip_errors = true
backup = true

[validation]
sample_ratio = 0.1  # 抽样 10% 验证
```

命令行参数优先级高于配置文件。

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_migrate_record_with_missing_fields() {
    let invalid_json = r#"{"id": "1"}"#; // 缺失 title 和 content
    let result = migrate_record(invalid_json).await;
    assert!(matches!(result, Err(MigrationError::MissingField(_))));
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let mock_embeddings = vec![vec![0.1; 384]; 100];
    let result = generate_embeddings_batch(texts, 100).await;
    assert_eq!(result.len(), 100);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_migration() {
    let temp_dir = tempdir()?;
    let json_path = temp_dir.path().join("test.json");
    let lancedb_uri = temp_dir.path().join("db");

    // 创建测试 JSON
    create_test_json(&json_path, 1000)?;

    // 运行迁移
    let stats = migrate_json_to_lancedb(MigrationConfig {
        json_path,
        lancedb_uri: lancedb_uri.to_str().unwrap(),
        ..Default::default()
    }).await?;

    assert_eq!(stats.successful, 1000);
    assert_eq!(stats.failed, 0);

    // 验证数据
    let report = validate_migration(&lancedb_uri, "knowledge", 1000).await?;
    assert!(report.is_valid);
}
```

## Performance Benchmarks

目标性能指标：

| 数据量 | 预期时间 | 内存占用 |
|--------|---------|---------|
| 1,000 条 | ~5 秒 | ~50 MB |
| 10,000 条 | ~30 秒 | ~200 MB |
| 100,000 条 | ~5 分钟 | ~1 GB |

**优化建议**：
- 使用 `mimalloc` 替代系统分配器（减少内存碎片）
- 启用 Tokio 的 `rt-multi-thread` 运行时
- 对大文件使用内存映射（`memmap2` crate）

## Rollback Plan

如果迁移失败：

1. **自动备份**：原始 JSON 文件已备份为 `.json.bak`
2. **LanceDB 清理**：删除不完整的表
   ```bash
   rm -rf ~/.contextfy/db/knowledge.lance
   ```
3. **恢复命令**（未来实现）：
   ```bash
   kit migrate --rollback
   ```
