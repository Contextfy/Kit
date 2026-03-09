## ADDED Requirements

### Requirement: 文本向量化

The core engine SHALL provide a text embedding module that converts text into 384-dimensional float vectors using the BGE-small-en-v1.5 model via FastEmbed. 核心引擎 SHALL 提供文本向量化模块，使用 FastEmbed 的 BGE-small-en-v1.5 模型将文本转换为 384 维浮点向量。

#### Scenario: 初始化嵌入模型

- **当**系统调用 `EmbeddingModel::new()` 时
- **则**系统加载 BGE-small-en-v1.5 模型
- **并且**返回 `Result<EmbeddingModel>` 表示初始化成功或失败
- **如果**模型加载失败，返回描述性错误信息

#### Scenario: 将文本转换为向量

- **当**系统调用 `embed_text(text)` 方法时
- **则**系统将输入文本传递给 FastEmbed 模型
- **并且**返回 `Result<Vec<f32>>` 包含 384 维向量
- **并且**向量长度严格等于 384
- **如果**嵌入生成失败，返回 `Err(anyhow::Error)`

#### Scenario: 线程安全的模型共享

- **当** `EmbeddingModel` 被 `Arc` 包裹并在多线程环境中使用时
- **则**系统允许安全的并发调用 `embed_text` 方法
- **并且**模型实现 `Send + Sync` trait

#### Scenario: 向量化性能要求

- **当**系统对单条文本（少于 500 字符）执行向量化时
- **则**生成向量的时间应 < 100ms
- **并且**性能测试包含模型加载后的首次调用（冷启动）和后续调用（热启动）

#### Scenario: 相同文本产生相同向量

- **当**系统对相同文本内容多次调用 `embed_text` 时
- **则**系统返回相同的向量（浮点数误差除外）
- **并且**向量维度保持一致

### Requirement: FastEmbed 依赖集成

The core engine SHALL include the fastembed crate as a dependency in packages/core/Cargo.toml. 核心引擎 SHALL 在 packages/core/Cargo.toml 中包含 fastembed 依赖。

#### Scenario: 依赖版本管理

- **当**开发者在 `packages/core/Cargo.toml` 中添加 fastembed 依赖时
- **则**使用最新稳定版本
- **并且**依赖版本在工作空间中保持一致

#### Scenario: 编译时验证依赖

- **当**系统执行 `cargo build -p contextfy-core` 时
- **则**fastembed 依赖被成功解析和下载
- **并且**编译成功无依赖冲突错误

### Requirement: 嵌入模块导出

The core engine SHALL export the embeddings module through lib.rs for external use. 核心引擎 SHALL 通过 lib.rs 导出 embeddings 模块供外部使用。

#### Scenario: 模块公开访问

- **当**外部代码使用 `contextfy_core::embeddings` 时
- **则**模块必须公开可访问
- **并且** `EmbeddingModel` 结构体可被实例化

#### Scenario: 模块在 lib.rs 中注册

- **当** `contextfy-core` crate 被编译时
- **则**`embeddings` 模块包含在 `lib.rs` 的 `pub mod` 声明中
- **并且**模块的公共 API 可被依赖该 crate 的代码使用
