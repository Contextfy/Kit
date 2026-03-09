# core-engine Specification Deltas

## ADDED Requirements

### Requirement: 向量持久化存储

The core engine SHALL persist document embeddings in the JSON-based storage to enable semantic similarity computation without re-embedding. 核心引擎 SHALL 在基于 JSON 的存储中持久化文档向量，以实现语义相似度计算而无需重新嵌入。

#### Scenario: 存储带向量的知识记录

- **当**系统将 `KnowledgeRecord` 写入 JSON 文件时
- **则**记录包含 `embedding` 字段，类型为 `Option<Vec<f32>>`
- **并且**如果向量存在，JSON 中包含完整的浮点数数组
- **并且**如果向量为 None，字段在 JSON 中可以不存在或为 null

#### Scenario: 反序列化旧版 JSON 兼容性

- **当**系统读取不包含 `embedding` 字段的旧版 JSON 文件时
- **则**反序列化成功，`embedding` 字段默认为 `None`
- **并且**不产生错误或警告
- **并且**记录可正常使用

### Requirement: 余弦相似度计算

The core engine SHALL provide a cosine similarity function for comparing two embedding vectors with proper normalization and divide-by-zero protection. 核心引擎 SHALL 提供余弦相似度函数用于比较两个嵌入向量，具有正确的归一化和除零保护。

#### Scenario: 计算相同向量的相似度

- **当**系统对两个相同的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 1.0（完全相似）
- **并且**不产生除零错误

#### Scenario: 计算正交向量的相似度

- **当**系统对两个正交（点积为 0）的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 0.5（归一化后的中间值）

#### Scenario: 计算相反向量的相似度

- **当**系统对两个相反（`b = -a`）的向量调用 `cosine_similarity(a, b)` 时
- **则**返回 0.0（完全不相似）

#### Scenario: 处理零向量

- **当**系统遇到至少一个输入为零向量（所有元素为 0）时
- **则**返回 0.0（除零保护）
- **并且**不产生 panic 或数值错误

#### Scenario: 归一化到 0-1 范围

- **当**系统计算余弦相似度时
- **则**结果被归一化到 [0.0, 1.0] 范围
- **并且**使用公式：`(raw_cosine + 1.0) / 2.0`
- **其中** `raw_cosine = (a · b) / (||a|| * ||b||)`

### Requirement: 批量向量生成

The core engine SHALL provide batch embedding generation to efficiently process multiple texts in a single model inference call. 核心引擎 SHALL 提供批量向量生成以高效地在单次模型推理中处理多个文本。

#### Scenario: 批量生成向量

- **当**系统调用 `embed_batch(texts)` 时
- **则**系统一次性处理所有文本
- **并且**返回 `Vec<Vec<f32>>`，长度与输入一致
- **并且**每个向量维度均为 384
- **并且**利用 FastEmbed 的原生批处理能力

#### Scenario: 处理空输入

- **当**系统调用 `embed_batch(&[])` 时
- **则**返回空 `Vec::new()`
- **并且**不产生错误

#### Scenario: 批量处理性能优势

- **当**系统批量处理 N 条文本时
- **则**总耗时显著低于 N 次单独调用 `embed_text`
- **并且**减少模型推理开销

## MODIFIED Requirements

### Requirement: Knowledge Storage

The core engine SHALL store parsed documents and their semantic chunks in a file-system-based JSON storage with atomic write guarantees, support for slice-level storage, AND integrate Tantivy full-text search index for BM25 scoring, AND extract keywords from code blocks for enhanced API search relevance, **AND generate and persist embeddings for slices with content or fallback records without slices using batch processing with graceful degradation**. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证、支持切片级别存储，并集成 Tantivy 全文搜索索引以进行 BM25 评分，并从代码块提取关键词以增强 API 搜索相关性，**并使用批处理为有内容的切片或无切片的 Fallback 记录生成和持久化向量，并支持优雅降级**。

#### Scenario: 添加文档时提取关键词并同步更新索引

- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统解析文档内容并从代码块中提取关键词
- **并且**将提取的关键词存储到 `KnowledgeRecord.keywords` 字段
- **并且在写入 JSON 文件后，同步添加文档到 Tantivy 索引**
- **并且**为每个切片调用 `Indexer::add_doc(record)` 时包含 keywords 字段
- **并且**在所有切片添加后调用 `Indexer::commit()` 提交索引
- **并且**收集所有切片的 `title` 和 `summary` 拼接文本**
- **并且**调用 `embed_batch()` 一次性生成所有切片的向量**
- **并且**将向量赋值给对应 `KnowledgeRecord` 的 `embedding` 字段**
- **如果**索引操作失败，记录警告但不影响 JSON 存储
- **如果**向量模型未注入，所有切片的 `embedding` 字段为 `None`（优雅降级，文档仍可通过 BM25 检索）**
- **如果**向量生成失败，记录警告并将 `embedding` 设为 `None`（优雅降级，文档仍可通过 BM25 检索）**
- **如果**向量数量与切片数量不匹配，打印警告并将所有切片的 `embedding` 设为 `None`（优雅降级，防止越界 panic）**
- **并且**返回所有切片的 UUID 列表

#### Scenario: 初始化知识存储时创建索引

- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统在 `{data_dir}/.tantivy` 目录创建或打开 Tantivy 索引
- **并且**初始化 `Indexer` 和 `Searcher` 实例
- **并且**初始化或接收共享的 `EmbeddingModel` 实例**
- **如果**索引初始化失败（如权限问题），记录警告并继续运行
- **如果**向量模型初始化失败，记录警告并继续运行（向量字段将为 `None`）**
- **并且**系统自动清理上次崩溃遗留的 `.temp-*` 临时目录

#### Scenario: 搜索存储的切片（BM25 搜索替换朴素匹配）

- **当**用户使用查询字符串搜索文档时
- **则**系统优先使用 Tantivy `Searcher` 执行 BM25 全文搜索
- **并且**返回结果包含 BM25 相关性分数
- **并且**结果按 BM25 分数降序排列（替代原有的分词匹配分数）
- **如果** Tantivy 索引不可用，回退到原有的基于分词的匹配逻辑
- **并且**返回匹配的切片记录列表（按相关性排序）

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

#### Scenario: 批量将文本转换为向量

- **当**系统调用 `embed_batch(texts)` 方法时
- **则**系统将输入文本列表传递给 FastEmbed 模型的批处理接口
- **并且**返回 `Result<Vec<Vec<f32>>>` 包含所有 384 维向量
- **并且**返回向量数量严格等于输入文本数量
- **并且**每个向量长度严格等于 384
- **如果**批量嵌入生成失败，返回 `Err(anyhow::Error)`

#### Scenario: 线程安全的模型共享

- **当** `EmbeddingModel` 被 `Arc` 包裹并在多线程环境中使用时
- **则**系统允许安全的并发调用 `embed_text` 和 `embed_batch` 方法
- **并且**模型实现 `Send + Sync` trait

#### Scenario: 向量化性能要求

- **当**系统对单条文本（少于 500 字符）执行向量化时
- **则**生成向量的时间应 < 100ms
- **并且**性能测试包含模型加载后的首次调用（冷启动）和后续调用（热启动）

#### Scenario: 批量向量化性能要求

- **当**系统对 N 条文本执行批量向量化时
- **则**总耗时应显著低于 N 次单独调用的总和
- **并且**利用 FastEmbed 的批处理优化

#### Scenario: 相同文本产生相同向量

- **当**系统对相同文本内容多次调用 `embed_text` 时
- **则**系统返回相同的向量（浮点数误差除外）
- **并且**向量维度保持一致

## REMOVED Requirements

None
