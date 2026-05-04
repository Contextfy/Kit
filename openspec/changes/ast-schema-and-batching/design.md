# Design: AST Schema 重塑与批量操作优化

## Context

当前 Contextfy 核心引擎使用**文档级**存储模型（`title`, `summary`, `content`, `keywords`），这是为 Markdown 文档检索设计的。然而，Issue #22 要求支持**代码语义检索**，需要存储代码语法树（AST）节点信息，如类名、函数名、文件路径、依赖关系等。

同时，现有实现使用逐条插入，导致严重的性能瓶颈：
- 向量生成：每次调用 `embed_text()` 触发一次模型推理
- 数据库写入：每次 `add()` 触发一次数据库事务
- **实测性能**：1000 文档构建 > 20s（远超 10s 目标）

## Constraints

1. **向后兼容性**：现有调用方（CLI、Server）不能因 Schema 变更而崩溃
2. **数据迁移**：现有 LanceDB 和 Tantivy 索引需要迁移到新 Schema
3. **时间紧迫**：Issue #22 要求 1 小时完成，但实际是架构级重构
4. **技术栈限制**：
   - LanceDB 使用 Arrow 格式，`ListArray<Utf8>` 构建复杂
   - Tantivy 需要手动配置字段权重（通过 `QueryParser::set_field_boost`）
   - FastEmbed 支持批量，但必须在调用方正确使用

## Goals / Non-Goals

**Goals**:
- ✅ 引入 `AstChunk` 结构体，支持代码 AST 节点表示
- ✅ 实现批量操作 `add_batch()`，性能提升 >= 50%
- ✅ BM25 检索时 `symbol_name` 字段获得最高权重（5.0x）
- ✅ 向后兼容：旧 API 仍可工作

**Non-Goals**:
- ❌ 不修改现有 `VectorStoreTrait::add()` 和 `Bm25StoreTrait::add()` 签名（保持兼容）
- ❌ 不实现自动数据迁移工具（手动迁移或重建索引）
- ❌ 不引入新的依赖库（仅使用现有 FastEmbed、LanceDB、Tantivy）

## Decisions

### Decision 1: AstChunk 字段设计

**What**: 定义 7 个字段的 `AstChunk` 结构体：
- `id`: String（哈希签名）
- `file_path`: String（如 `src/auth.rs`）
- `symbol_name`: String（如 `AuthManager`，**BM25 最高权重**）
- `node_type`: String（Enum: `file`, `class`, `function`, `method`, `variable`）
- `content`: String（完整代码块，用于向量嵌入）
- `dependencies`: Vec<String>（依赖列表）
- `vector`: Option<Vec<f32>>（向量，入库时生成）

**Why**:
- `file_path` + `symbol_name` 唯一标识代码实体（比单纯的 `title` 更精确）
- `node_type` 支持按类型过滤（如只搜索函数）
- `dependencies` 支持依赖图分析（未来可扩展）
- `vector` 设为 Option，调用方无需关心生成逻辑

**Alternatives considered**:
1. **扩展现有 `KnowledgeRecord`**： rejected，因为字段语义完全不同（`title` vs `symbol_name`）
2. **使用泛型 `Document<T>`**： rejected，增加复杂度，Rust 序列化困难
3. **分离 CodeChunk 和 TextChunk**： rejected，会导致 trait 爆炸（`VectorStoreTrait<Code>`, `VectorStoreTrait<Text>`）

### Decision 2: LanceDB dependencies 存储策略

**What**: 将 `dependencies: Vec<String>` 序列化为逗号分隔字符串存储（如 `"tokio,serde,anyhow"`）

**Why**:
- **Arrow ListArray 构建复杂**：需要管理偏移量数组（`offsets: [0, 2, 5, ...]`），容易出错
- **时间紧迫**：Issue #22 预计 1 小时，但实际是架构重构，需权衡
- **查询性能**：字符串查询 `"tokio"` 使用 `LIKE "%tokio%"` 即可

**Alternatives considered**:
1. **使用 `DataType::List(Box::new(DataType::Utf8))`**： rejected，Arrow ListArray 构建复杂，调试成本高
2. **使用 JSON 序列化**： rejected，LanceDB 无法高效查询 JSON 内部字段
3. **拆分为多行**（一个依赖一行）： rejected，破坏 `id` 唯一性约束

**Trade-offs**:
- ❌ 无法使用 Arrow 的原生列表查询
- ✅ 实现简单，不易出错
- ✅ 存储紧凑（逗号分隔比 JSON 更短）

### Decision 3: Tantivy 字段权重配置

**What**: 在 `QueryParser` 中配置字段权重：
- `symbol_name^5.0`（最高权重）
- `dependencies^2.0`（次要权重）
- `content^1.0`（基准权重）

**Why**:
- **精确符号检索**：用户搜索 "AuthManager" 时，符号名应优先匹配
- **避免噪音**：`content` 字段包含大量代码，权重过高会返回不相关结果
- **依赖关系增强**：搜索 "tokio" 时，优先返回依赖 tokio 的模块

**Alternatives considered**:
1. **使用 Tantivy 的 `BoostQuery`**： rejected，需要在每个查询中手动构造
2. **分离多个索引**（symbol_name 索引 + content 索引）： rejected，维护成本高
3. **后处理排序**（先检索再排序）： rejected，无法利用 Tantivy 的倒排索引优化

### Decision 4: 批量操作性能防线

**What**: 在 `add_batch()` 实现中强制执行三条防线：
1. **LanceDB**: `embed_batch()` 绝不在循环中调用
2. **LanceDB**: 构建单个 `RecordBatch`，一次性写入
3. **Tantivy**: 单次事务 `commit()`，绝不在循环中提交

**Why**:
- **向量生成瓶颈**：FastEmbed 的批量 API 可减少 GPU/CPU 上下文切换
- **数据库事务开销**：每次 `commit()` 触发磁盘刷盘，批量提交减少 I/O
- **实测数据**：批量处理可提升 3-5x 性量（参考 Issue #22 描述）

**Alternatives considered**:
1. **使用异步批处理**（累积一定数量后批量写入）： rejected，增加复杂度，延迟不可控
2. **并发写入**（多线程同时写入）： rejected，LanceDB 和 Tantivy 不是线程安全的
3. **缓存后批量刷盘**： rejected，系统崩溃可能丢失数据

### Decision 5: 向后兼容策略

**What**: 在 Facade 层保留 `add(id, title, summary, content, keywords)` 方法，内部映射到 `AstChunk`

**Why**:
- **零修改迁移**：现有 CLI 和 Server 无需修改代码
- **渐进式迁移**：新代码使用 `add_batch()`，旧代码逐步迁移

**映射逻辑**:
```rust
AstChunk {
    id: id.to_string(),
    file_path: "unknown".to_string(),  // 旧 API 无此信息
    symbol_name: title.to_string(),     // 使用 title 作为 symbol_name
    node_type: "file".to_string(),      // 默认为文件类型
    content: content.to_string(),
    dependencies: keywords
        .map(|k| k.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default(),
    vector: None,
}
```

**Alternatives considered**:
1. **废弃旧 API**（标记为 deprecated）： rejected，会导致现有代码编译失败
2. **提供迁移工具**： rejected，时间成本高，用户需要手动运行
3. **Feature flag 控制新旧 API**： rejected，增加维护成本

## Risks / Trade-offs

### Risk 1: Arrow ListArray 复杂性

**Risk**: 如果未来需要复杂查询（如"依赖 tokio 但不依赖 serde"），字符串序列化方案不够用

**Mitigation**:
- 短期：使用字符串查询 `LIKE "%tokio%" AND NOT LIKE "%serde%"`
- 长期：如果需求复杂化，再迁移到 Arrow ListArray（已有 Schema 版本控制）

### Risk 2: Tantivy 字段权重不生效

**Risk**: `QueryParser::set_field_boost()` 配置可能被覆盖（如用户自定义查询语法）

**Mitigation**:
- 在文档中明确说明权重配置
- 提供查询语法验证（检测用户是否手动覆盖权重）
- 添加单元测试验证权重生效

### Risk 3: 批量操作内存占用

**Risk**: 1000+ chunks 的 `add_batch()` 可能占用大量内存（向量数据 + Arrow 数组）

**Mitigation**:
- 文档中建议分批处理（如每次 100-500 chunks）
- 实现流式处理（未来优化方向）
- 添加内存监控（日志中记录内存使用）

### Risk 4: 数据迁移失败

**Risk**: 现有索引迁移到新 Schema 时可能失败（如字段不兼容）

**Mitigation**:
- 提供迁移脚本（`scripts/migrate_to_ast_schema.sh`）
- 文档中说明迁移步骤
- 保留旧 Schema 代码作为回退方案

## Migration Plan

### 阶段 1: 准备工作
1. 备份现有 LanceDB 和 Tantivy 索引
2. 创建新分支 `feat/ast-schema-migration`
3. 通知所有开发者暂停索引写入

### 阶段 2: Schema 部署
1. 部署新代码（包含 `AstChunk` 和新 Schema）
2. 运行 Schema 验证（`openspec validate`）
3. 确认新 Schema 正确

### 阶段 3: 数据迁移
1. **选项 A**：重建索引（推荐，简单）
   ```bash
   rm -rf data/lancedb data/tantivy
   cargo run --bin contextfy-cli -- build
   ```

2. **选项 B**：迁移工具（复杂，保留数据）
   ```bash
   cargo run --bin migration-tool -- migrate-to-ast
   ```

### 阶段 4: 验证
1. 运行测试套件（`cargo test`）
2. 性能测试（1000 文档构建 < 10s）
3. 搜索功能验证（BM25 权重生效）

### 回滚计划
如果迁移失败：
1. 恢复备份的索引
2. 回滚到旧代码（`git checkout <pre-migration-commit>`）
3. 重新部署

## Open Questions

1. **Cocoindex 输出格式**：Cocoindex 是否完全匹配 `AstChunk` 结构？是否需要适配层？
   - **建议**：在实现时验证 Cocoindex 输出，必要时添加转换函数

2. **向量生成时机**：`AstChunk.vector` 是在调用方生成还是 `add_batch()` 内部生成？
   - **建议**：在 `add_batch()` 内部生成（调用方无需关心 FastEmbed 细节）

3. **错误处理策略**：`add_batch()` 中某一条 chunk 失败，是否回滚整个 batch？
   - **建议**：整批失败（简化语义），调用方负责重试

4. **性能测试基准**：当前逐条插入的基准性能是多少？（需要实测）
   - **建议**：在实现前先测量现有性能，作为对比基准

## Implementation Notes

### LanceDB Arrow Schema 构建

关键代码模式：
```rust
// Dependencies 序列化
let dependencies_array = StringArray::from(
    chunks.iter()
        .map(|c| {
            if c.dependencies.is_empty() {
                None
            } else {
                Some(c.dependencies.join(","))
            }
        })
        .collect::<Vec<Option<String>>>()
);

// Vector: FixedSizeListArray
let all_vector_values = Float32Array::from(
    embeddings.into_iter()
        .flat_map(|vec| vec.into_iter())
        .collect::<Vec<f32>>()
);
let vector_array = FixedSizeListArray::new(
    Arc::new(Field::new("item", DataType::Float32, false)),
    VECTOR_DIM,
    Arc::new(all_vector_values),
    None,
);
```

### Tantivy QueryParser 权重配置

关键代码模式：
```rust
let mut query_parser = QueryParser::for_index(
    &index,
    vec![symbol_name_field, content_field, dependencies_field, ...],
);

// 设置权重
query_parser.set_field_boost(symbol_name_field, 5.0);
query_parser.set_field_boost(dependencies_field, 2.0);
query_parser.set_field_boost(content_field, 1.0);
```

### Tantivy Dependencies 多值字段

关键代码模式：
```rust
// 写入时
for dep in &chunk.dependencies {
    doc.add_text(dependencies_field, dep);
}

// 搜索时（自动匹配任一依赖）
// 无需特殊处理，Tantivy 会自动匹配多值字段
```

## References

- Issue #22: [TASK] Core - 批量操作优化
- LanceDB Arrow Schema 文档: https://lancedb.github.io/lancedb/arrow/
- Tantivy QueryParser 文档: https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html
- FastEmbed 批处理 API: https://docs.rs/fastembed/latest/fastembed/
- 已归档提案: `openspec/changes/archive/2026-03-09-add-vector-storage/`
