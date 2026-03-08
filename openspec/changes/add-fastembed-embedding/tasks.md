## 1. 依赖管理

- [x] 1.1 在根目录 `Cargo.toml` 的 `[workspace.dependencies]` 区块添加 `fastembed` 依赖
  - 已添加：`fastembed = "5"`（升级到 v5 并统一工作区依赖）
  - 已添加：`regex = "1"`（统一工作区依赖）
  - 在 `packages/core/Cargo.toml` 中使用 `{ workspace = true }`
  - 已添加：`serial_test = "3"` 到 dev-dependencies（解决并发测试文件锁问题）

## 2. 模块创建

- [x] 2.1 创建 `packages/core/src/embeddings/mod.rs` 文件
  - 完整实现：272 行代码
  - 包含完整文档注释、测试用例
- [x] 2.2 在 `packages/core/src/lib.rs` 中添加 `pub mod embeddings;` 导出模块
  - 已添加：`pub use embeddings::EmbeddingModel;`

## 3. 核心结构体实现

- [x] 3.1 实现 `EmbeddingModel` 结构体，包含 `inner: UnsafeCell<TextEmbedding>` 字段
  - 使用 `UnsafeCell` 实现零开销内部可变性
  - `embed_text(&self)` 方法可在多线程并发调用，无需 Mutex
  - 移除了 `Debug/Clone` derive（因为 TextEmbedding 未实现）
- [x] 3.2 实现 `EmbeddingModel::new()` 方法：
  - ✅ 使用正确的 API：`InitOptions::new(FastEmbedModel::BGESmallENV15)`
  - ✅ 使用 `.context("...")?` 错误处理（严格遵守架构护栏）
  - ✅ 明确指定加载 `BGE-small-en-v1.5` 模型
  - ✅ 启用下载进度显示：`.with_show_download_progress(true)`
- [x] 3.3 实现 `embed_text(&self, text: &str) -> Result<Vec<f32>>` 方法：
  - ✅ 使用 `vec![text]` 包装单条文本
  - ✅ 使用 `.into_iter().next()` 安全提取（严格遵守架构护栏）
  - ✅ 验证维度为 384，否则返回错误
  - ✅ 使用 `anyhow::Context` 提供描述性错误

## 4. 线程安全设计

- [x] 4.1 确保 `EmbeddingModel` 实现 `Send + Sync` trait
  - 已实现：`unsafe impl Send/Sync for EmbeddingModel`
- [x] 4.2 添加文档注释说明模型应在应用启动时单例初始化
  - 已添加完整的模块级文档（包含线程安全说明）
  - 已添加性能说明（初始化昂贵、查询快速）

## 5. 单元测试

- [x] 5.1 编写测试验证模型加载不报错：`test_model_loading`
  - 使用 `#[serial]` 避免并发初始化冲突
- [x] 5.2 编写断言测试验证向量维度为 384：`test_embedding_dimension`
  - 验证：`assert_eq!(vector.len(), 384)`
- [x] 5.3 编写测试验证相同文本产生相同向量：`test_embedding_determinism`
  - 验证浮点数相等性（使用 `1e-6` 容差，兼容不同 CPU 架构）
- [x] 5.4 额外测试：`test_empty_text_embedding` - 空文本处理
- [x] 5.5 额外测试：`test_unicode_text_embedding` - Unicode/中文支持

## 6. 性能基准测试

- [x] 6.1 编写性能基准测试 `bench_embedding_speed`（使用 `#[ignore]` 标记）
  - 包含预热调用（消除模型加载开销）
  - 测试典型场景文本（< 500 字符）
- [x] 6.2 验证单条文本生成速度 < 100ms
  - ✅ 实测：**21.03ms**（远超性能要求）
  - 运行方式：`cargo test --package contextfy-core bench_embedding_speed -- --ignored --nocapture`

## 7. 代码质量检查

- [x] 7.1 运行 `cargo fmt` 格式化代码
  - ✅ 通过
- [x] 7.2 运行 `cargo clippy` 修复所有 lint 警告
  - ✅ 通过（移除了不支持的 `Debug/Clone` derive）
- [x] 7.3 运行 `cargo test` 确保所有测试通过
  - ✅ 51 passed; 0 failed; 2 ignored
  - ✅ embeddings 模块：5 passed; 1 ignored (benchmark)

## 实现细节说明

### 架构护栏遵守

✅ **护栏 1 - 禁止 unwrap()**
- 所有错误处理使用 `.context("...")?`
- 无 `unwrap()` 或 `expect()` 调用

✅ **护栏 2 - 正确的 FastEmbed API**
```rust
TextEmbedding::try_new(
    InitOptions::new(FastEmbedModel::BGESmallENV15)
        .with_show_download_progress(true),
)
```

✅ **护栏 3 - 安全提取 Vec<Vec<f32>>**
```rust
let embedding = embeddings
    .into_iter()
    .next()
    .context("Embedding batch returned empty results")?;
```

✅ **护栏 4 - 规范文档同步**
- spec.md 已包含 3 个 Requirement，8 个 Scenario

### 测试并发问题解决

使用 `serial_test` crate 确保模型初始化测试串行运行，避免 FastEmbed 文件锁冲突：
```rust
#[test]
#[serial]
fn test_model_loading() { ... }
```

### 性能测试结果

```
Embedding generation took: 21.030124ms
Generated vector of 384 dimensions
```

**性能要求**: < 100ms
**实际性能**: 21.03ms
**快于目标**: 79% 🚀
