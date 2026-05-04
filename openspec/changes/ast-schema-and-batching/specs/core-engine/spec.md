# core-engine Specification Deltas

## ADDED Requirements

### Requirement: AST Chunk 数据模型

The core engine SHALL provide an `AstChunk` structure to represent code syntax tree nodes with file path, symbol name, node type, dependencies, and content fields. 核心引擎 SHALL 提供 `AstChunk` 结构体以表示代码语法树节点，包含文件路径、符号名、节点类型、依赖关系和内容字段。

#### Scenario: AstChunk 序列化包含所有字段

- **当**系统序列化 `AstChunk` 到 JSON 时
- **则** JSON 包含以下字段：
  - `id`: 字符串（哈希签名）
  - `file_path`: 字符串（如 `src/auth.rs`）
  - `symbol_name`: 字符串（如 `AuthManager`）
  - `node_type`: 字符串（如 `class`, `function`）
  - `content`: 字符串（完整代码块）
  - `dependencies`: 字符串数组
- **并且** `vector` 字段被跳过（使用 `#[serde(skip)]`）

#### Scenario: AstChunk 反序列化不包含 vector 字段

- **当**系统从 JSON 反序列化 `AstChunk` 时
- **则** `vector` 字段默认为 `None`
- **并且**不产生错误

### Requirement: LanceDB AST Chunk Schema

The core engine SHALL define an Arrow schema for LanceDB that maps AST chunk fields to Arrow types, with dependencies serialized as comma-separated strings. 核心引擎 SHALL 为 LanceDB 定义 Arrow Schema，将 AST chunk 字段映射到 Arrow 类型，dependencies 序列化为逗号分隔字符串。

#### Scenario: LanceDB Schema 字段映射正确

- **当**系统调用 `ast_chunk_schema()` 时
- **则**返回的 Schema 包含以下字段：
  - `id`: Utf8 (non-null)
  - `file_path`: Utf8 (non-null)
  - `symbol_name`: Utf8 (non-null)
  - `node_type`: Utf8 (non-null)
  - `content`: Utf8 (non-null)
  - `dependencies`: Utf8 (nullable)
  - `vector`: FixedSizeList(Float32, 384) (non-null)

#### Scenario: Dependencies 序列化为逗号分隔字符串

- **当**系统将 `AstChunk` 的 `dependencies` 字段写入 LanceDB 时
- **则** `Vec<String>` 被转换为逗号分隔字符串（如 `["tokio", "serde"]` → `"tokio,serde"`）
- **并且**空 `dependencies` 写入为 null 或空字符串

#### Scenario: LanceDB Schema 验证失败返回错误

- **当**现有 LanceDB 表的 Schema 与 `ast_chunk_schema()` 不匹配时
- **则** `validate_ast_chunk_schema()` 返回 `Err(String)`
- **并且**错误消息描述不匹配的字段和类型

### Requirement: Tantivy AST Chunk Schema

The core engine SHALL define a Tantivy schema for BM25 search with symbol_name field boosted to 5.0x weight for precise symbol retrieval. 核心引擎 SHALL 为 BM25 搜索定义 Tantivy Schema，symbol_name 字段提升到 5.0 倍权重以实现精确符号检索。

#### Scenario: Tantivy Schema 包含所有新字段

- **当**系统调用 `create_bm25_schema()` 时
- **则**返回的 Schema 包含以下字段：
  - `id`: STRING 类型（不进行分词）
  - `file_path`: TEXT 类型（jieba 分词）
  - `symbol_name`: TEXT 类型（jieba 分词，**5.0 倍权重**）
  - `node_type`: TEXT 类型（jieba 分词）
  - `content`: TEXT 类型（jieba 分词）
  - `dependencies`: TEXT 类型（jieba 分词，支持多值）

#### Scenario: symbol_name 字段获得最高权重

- **当**系统在 `QueryParser` 中配置字段权重时
- **则** `symbol_name` 权重为 5.0
- **并且** `dependencies` 权重为 2.0
- **并且** `content` 权重为 1.0（基准）

#### Scenario: Dependencies 多值字段存储

- **当**系统将包含多个依赖的 `AstChunk` 写入 Tantivy 时
- **则**每个依赖项单独调用 `doc.add_text(dependencies_field, dep)`
- **并且**搜索时匹配任一依赖项都能找到该文档

### Requirement: 批量向量生成

The core engine SHALL generate embeddings for multiple AST chunks in a single batch operation to avoid per-chunk model inference overhead. 核心引擎 SHALL 在单次批处理操作中为多个 AST chunks 生成向量，避免逐块模型推理开销。

#### Scenario: 批量生成向量使用 embed_batch

- **当**系统调用 `add_batch(chunks)` 时
- **则**一次性收集所有 chunks 的 `content` 字段到 `Vec<&str>`
- **并且**调用一次 `embed_batch(contents)` 生成所有向量
- **并且**绝不在循环中调用 `embed_text()`

#### Scenario: 批量向量生成性能优势

- **当**系统批量处理 N 个 chunks 时
- **则**总耗时显著低于 N 次单独调用 `embed_text()`
- **并且**至少快 50%（性能测试验证）

### Requirement: 批量数据库写入

The core engine SHALL write multiple AST chunks to LanceDB and Tantivy in batch operations to minimize transaction overhead. 核心引擎 SHALL 以批处理操作将多个 AST chunks 写入 LanceDB 和 Tantivy，以最小化事务开销。

#### Scenario: LanceDB 批量写入使用 RecordBatch

- **当**系统将 N 个 chunks 写入 LanceDB 时
- **则**构建单个 `RecordBatch` 包含所有 N 行
- **并且**调用一次 `table.add(reader)` 提交
- **并且**绝不在循环中调用 `table.add()`

#### Scenario: Tantivy 批量写入使用单次事务

- **当**系统将 N 个 chunks 写入 Tantivy 时
- **则**在一个 `writer` 锁中添加所有 N 个文档
- **并且**循环结束后调用一次 `writer.commit()`
- **并且**绝不在循环中调用 `writer.commit()`

#### Scenario: 批量写入性能目标

- **当**系统批量写入 1000 个 chunks 时
- **则**总耗时 < 10 秒（包括向量生成和数据库写入）
- **并且**相比逐条写入，性能提升 >= 50%

### Requirement: 向后兼容性

The core engine SHALL maintain backward compatibility by mapping legacy `add()` parameters to `AstChunk` structure internally. 核心引擎 SHALL 通过将旧版 `add()` 参数内部映射到 `AstChunk` 结构来保持向后兼容性。

#### Scenario: 旧版 add() 方法仍可工作

- **当**调用方使用 `add(id, title, summary, content, keywords)` 时
- **则**系统将参数映射到 `AstChunk`：
  - `id = id`
  - `file_path = "unknown"`
  - `symbol_name = title`
  - `node_type = "file"`
  - `content = content`
  - `dependencies = keywords.split_whitespace().collect()`
- **并且**调用 `add_batch(vec![chunk])`

#### Scenario: 现有调用方无需修改

- **当**现有代码（CLI、Server）调用 `SearchEngine::add()` 时
- **则**无需修改代码
- **并且**功能正常工作

## MODIFIED Requirements

### Requirement: Knowledge Storage

The core engine SHALL store parsed documents and their semantic chunks in file-system-based JSON and LanceDB/Tantivy backends with atomic write guarantees, AND **support AST chunk model with batch operations for high-performance code semantic indexing**. 核心引擎 SHALL 在基于文件系统的 JSON 和 LanceDB/Tantivy 后端中存储解析的文档和语义块，具有原子写入保证，**并支持 AST chunk 模型和批量操作以实现高性能代码语义索引**。

#### Scenario: 添加文档时使用 AST chunk 模型

- **当**用户将 `ParsedDoc` 添加到知识存储时
- **则**系统将每个语义块转换为 `AstChunk`
- **并且** `AstChunk` 包含：`id`, `file_path`, `symbol_name` (从 title 提取), `node_type`, `content`, `dependencies`
- **并且**调用 `add_batch(chunks)` 批量写入
- **并且**生成向量嵌入使用批量处理
- **并且**LanceDB 和 Tantivy 并发写入

## REMOVED Requirements

None
