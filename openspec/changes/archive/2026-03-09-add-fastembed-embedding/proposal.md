# Change: 集成 FastEmbed 向量化模块

## Why

当前核心引擎缺乏文本向量化能力，无法实现语义搜索（Semantic Search）。为了支持混合检索（Hybrid Search），需要引入一个本地化、高性能的向量化模型。FastEmbed 是一个基于 ONNX Runtime 的轻量级嵌入库，无需外部 API 调用，适合离线场景。

## What Changes

- 在 `packages/core/Cargo.toml` 中添加 `fastembed` 依赖（最新稳定版）
- 创建 `packages/core/src/embeddings/mod.rs` 模块，并在 `lib.rs` 中导出
- 实现 `EmbeddingModel` 结构体，封装 FastEmbed 的 `TextEmbedding`
- 提供 `EmbeddingModel::new()` 方法，加载 BGE-small-en-v1.5 模型
- 实现 `embed_text(&self, text: &str) -> Result<Vec<f32>>` 方法
- 编写单元测试验证向量维度为 384，性能基准测试 < 100ms

**BREAKING**: None

## Impact

- Affected specs: core-engine
- Affected code: packages/core/src/
