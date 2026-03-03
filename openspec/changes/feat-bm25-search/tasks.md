# 实施任务清单

## 1. 核心结构体实现

- [x] 1.1 在 `packages/core/src/search/mod.rs` 中添加 `SearchResult` 结构体
  - 包含字段：`id: String`, `title: String`, `summary: String`, `score: f32`
  - 派生 `Debug`, `Clone` trait

- [x] 1.2 实现 `Indexer` 结构体
  - 添加字段：`index: Index`, `writer: IndexWriter`, `schema: Schema`
  - 实现 `Indexer::new(index: Index) -> Result<Self>`
  - 实现 `Indexer::add_doc(&mut self, record: &KnowledgeRecord) -> Result<()>`
  - 实现 `Indexer::commit(&mut self) -> Result<()>`
  - 实现 `Indexer::delete(&mut self, id: &str) -> Result<()>` (占位，未来实现)

- [x] 1.3 实现 `Searcher` 结构体
  - 添加字段：`index: Index`, `reader: IndexReader`, `query_parser: QueryParser`, `schema: Schema`
  - 实现 `Searcher::new(index: Index) -> Result<Self>`
  - 实现 `Searcher::search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>`
  - 确保返回结果按 BM25 分数降序排列

## 2. 存储层集成

- [x] 2.1 在 `KnowledgeStore` 结构体中添加 Tantivy 索引字段
  - 添加 `indexer: Option<Arc<Mutex<Indexer>>>` 字段（使用 Arc<Mutex<>> 支持并发和阻塞任务）
  - 添加 `searcher: Option<Arc<Searcher>>` 字段（使用 Arc 支持并发和阻塞任务）

- [x] 2.2 修改 `KnowledgeStore::new()` 方法
  - 在创建数据目录后初始化 Tantivy 索引
  - 索引目录：`{data_dir}/.tantivy`
  - 初始化 Indexer 和 Searcher 实例
  - 如果索引初始化失败，记录警告但继续运行（回退到原搜索）

- [x] 2.3 修改 `KnowledgeStore::add()` 方法
  - 在写入 JSON 文件后，同步添加文档到 Tantivy 索引
  - 使用 `tokio::task::spawn_blocking` 调用 `indexer.add_doc(record)` 和 `indexer.commit()`，避免阻塞 Tokio 线程池
  - 在所有文档添加后调用 `indexer.commit()` 提交索引
  - 如果索引失败，记录警告但不影响 JSON 存储

- [x] 2.4 替换 `KnowledgeStore::search()` 方法
  - 移除 O(n) 的 `find_by_title()` 遍历逻辑
  - 如果 Searcher 可用，使用 `tokio::task::spawn_blocking` 调用 `searcher.search(query, limit)` 执行 BM25 搜索，避免阻塞 Tokio 线程池
  - 通过 `result.id` 直接调用 `self.get_by_id_fast(&id)` 获取完整记录（O(1) 查询）
  - 增强容错性：使用 match 处理 JoinError 和读取失败，避免单个文件失败导致整批搜索中止
  - 如果 Searcher 不可用，回退到原搜索逻辑

## 3. 错误处理与质量门禁

- [x] 3.1 确保所有 Tantivy 错误正确转换为 `anyhow::Result`
  - 禁止使用 `unwrap()` 或 `expect()`
  - 使用 `.context()` 添加错误上下文信息
  - 所有 public 函数返回 `Result<T>`

- [x] 3.2 添加单元测试
  - `test_indexer_add_doc()`: 测试文档添加和 commit
  - `test_searcher_basic_query()`: 测试基本搜索查询
  - `test_searcher_bm25_scoring()`: 验证 BM25 分数计算
  - `test_search_results_ordering()`: 验证结果按分数降序排列
  - `test_searcher_empty_query()`: 测试空查询处理

- [x] 3.3 添加性能基准测试
  - `benchmark_search_latency()`: 插入 1000 个 Mock 文档，测量查询延迟
  - 验证查询延迟 < 100ms（实际结果：3ms）
  - 使用 `std::time::Instant`

- [x] 3.4 代码质量检查
  - 运行 `cargo fmt --package contextfy-core`
  - 运行 `cargo clippy --package contextfy-core` (修复所有警告)
  - 运行 `cargo test --package contextfy-core` (37/37 测试通过)

## 4. 文档与导出

- [x] 4.1 在 `packages/core/src/lib.rs` 中导出新增类型
  - `pub use search::{Indexer, Searcher, SearchResult}`

- [x] 4.2 添加公共 API 文档
  - 为 `Indexer` 及其方法添加 `///` 文档注释
  - 为 `Searcher` 及其方法添加 `///` 文档注释
  - 包含使用示例

## 5. 架构优化（PR Review 反馈）

- [x] 5.1 修复 O(n) 性能陷阱
  - 在 Schema 中添加 `id` 字段（STRING | STORED，精确匹配不分词）
  - 在 `Indexer::add_doc()` 中写入 `record.id`
  - 在 `Searcher::search()` 中提取并返回真实的 `record.id`
  - 删除 `find_by_title()` 方法，改为 `self.get_by_id_fast(&id)` O(1) 查询
  - 强制读写一致性：在搜索前调用 `reader.reload()` 确保读取最新 commit 数据

- [x] 5.2 清理代码坏味道
  - 删除多余的 `let _ = query_parser;` 语句
  - 运行 `cargo fmt` 统一代码风格

## 验收标准

- [x] ✓ 搜索结果按 BM25 分数降序排列
- [x] ✓ `SearchResult` 包含 `score` 字段
- [x] ✓ 1000 文档查询延迟 < 100ms (实际 3ms，远超目标)
- [x] ✓ `cargo test --package contextfy-core` 全部通过 (37/37)
- [x] ✓ `cargo clippy --package contextfy-core` 无警告
- [x] ✓ `cargo fmt --package contextfy-core` 格式正确
- [x] ✓ O(n) 遍历已消除，通过 record.id 直接查询
