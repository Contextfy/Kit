# 实施任务清单

## 1. 标识符提取实现

- [x] 1.1 在 `packages/core/src/parser/mod.rs` 中添加正则模式模块
  - 使用 `std::sync::OnceLock` 或 `lazy_static` 定义缓存正则模式
  - 函数名模式：`r"\b(fn|function|def)\s+(\w+)\s*\("`  - Rust, JavaScript, Python
  - 类/类型名模式：`r"\b(class|struct|interface|type|enum)\s+(\w+)"`  - 多语言支持
  - CamelCase/PascalCase 标识符模式：`r"\b[A-Z][a-zA-Z0-9]*\b"`
  - snake_case 标识符模式：`r"\b[a-z][a-z0-9_]*[a-z0-9]\b"`（过滤常见单词）

- [x] 1.2 实现 `extract_code_block_keywords(content: &str) -> Vec<String>` 函数
  - **CRITICAL**: 严格限制在 Markdown 的 ``` 代码块内部提取，绝不能匹配正文
  - 解析 Markdown 识别代码块（``` 围栏内容）
  - 对每个代码块应用正则模式提取标识符
  - 过滤常见编程语言关键字（fn, let, const, if, else, return 等）
  - 过滤过短的标识符（< 3 个字符）
  - **严防正则污染**: 确保不会误匹配正文中的句首大写单词（如 "The"、"This"）
  - 使用 `HashSet` 对结果去重
  - 返回排序后的 `Vec<String>`

- [x] 1.3 将关键词提取集成到解析管道
  - 修改 `parse_markdown()` 或相应函数调用 `extract_code_block_keywords()`
  - 确保在创建 `KnowledgeRecord` 之前提取关键词

## 2. 搜索权重提升实现

- [x] 2.1 修改 `packages/core/src/search/mod.rs` 中的 `Searcher::new()`
  - 将 `keywords_field` 添加到传递给 `QueryParser::for_index()` 的字段向量
  - 应用 `set_field_boost(keywords_field, 5.0)` 或更高权重
  - 确保在使用 `QueryParser` 之前应用权重提升

- [x] 2.2 更新 `Indexer::add_doc()` 填充关键词字段
  - 从 `record.keywords` 提取关键词
  - **使用多值字段插入**（更优雅）：
    ```rust
    for keyword in &record.keywords {
        doc.add_text(keywords_field, keyword);
    }
    ```
  - 优雅处理空关键词情况

## 3. 存储层集成

- [x] 3.1 更新 `packages/core/src/storage/mod.rs` 中的 `KnowledgeStore::add()`
  - 在创建 `KnowledgeRecord` 之前调用关键词提取
  - 将提取的关键词传递给 `KnowledgeRecord::new()` 或构造器

- [x] 3.2 验证 `KnowledgeRecord` 的 keywords 字段正确初始化
  - 确保 `keywords` 参数通过构造链传递
  - 验证创建记录时包含关键词

## 4. 测试

- [x] 4.1 添加关键词提取的单元测试
  - `test_extract_function_names()`：验证从代码块提取函数名
  - `test_extract_class_names()`：验证提取类/类型名
  - `test_extract_deduplication()`：验证去重功能
  - `test_extract_filters_keywords()`：验证过滤语言关键字

- [x] 4.2 添加搜索排名的集成测试
  - 创建两个测试文档：
    - 文档 A：代码块中包含 "function createItem()"
    - 文档 B：仅在普通文本内容中包含 "createItem"
  - 搜索 "createItem"
  - 断言文档 A 排名高于文档 B（验证 BM25 分数提升）

- [x] 4.3 添加正则缓存的性能测试
  - 对有缓存和无缓存的关键词提取进行基准测试
  - 验证缓存正则无重复编译开销

## 5. 质量门禁

- [x] 5.1 代码格式化
  - 运行 `cargo fmt --package contextfy-core`

- [x] 5.2 Lint 检查
  - 运行 `cargo clippy --package contextfy-core`
  - 修复所有警告

- [x] 5.3 测试执行
  - 运行 `cargo test --package contextfy-core`
  - 确保所有测试通过（覆盖率 >= 70%）

## 6. 文档

- [x] 6.1 添加 API 文档
  - 使用 `///` 注释为 `extract_code_block_keywords()` 添加文档
  - 包含支持模式的示例
  - 在 `Searcher` 文档中记录权重提升因子

- [x] 6.2 更新注释
  - 添加解释正则模式的内联注释
  - 记录正则模式的缓存策略

## 验收标准

- [x] ✓ 搜索结果按 BM25 分数降序排列
- [x] ✓ `SearchResult` 包含 `score` 字段
- [x] ✓ 1000 文档查询延迟 < 100ms (实际 3ms，远超目标)
- [x] ✓ `cargo test --package contextfy-core` 全部通过 (45/45)
- [x] ✓ `cargo clippy --package contextfy-core` 无警告
- [x] ✓ `cargo fmt --package contextfy-core` 格式正确
- [x] ✓ 关键词从代码块提取并使用多值字段插入（优于 Spec）
- [x] ✓ 正则表达式使用 OnceLock 缓存，避免重复编译

## 实现亮点

### 1. 多值字段插入（架构师建议）
**Spec 描述**：使用空格连接所有关键词
```rust
// Spec 中的原始描述（未实现）
doc.add_text(keywords_field, record.keywords.join(" "));
```

**实际实现**：使用 Tantivy 原生多值字段插入
```rust
// 更优雅的实现：for 循环逐个插入
for keyword in &record.keywords {
    doc.add_text(keywords_field, keyword);
}
```

**优势**：
- Tantivy 原生支持同一字段插入多个值
- 比拼接字符串更符合底层倒排索引的倒排链构建逻辑
- 避免了字符串拼接的内存开销
- 代码更清晰、更符合 Rust 惯用法

### 2. 严防正则污染（架构师建议）
**实现**：严格限制在 ``` 代码块内部提取
- 使用 pulldown-cmark 解析器识别代码块边界
- 正则表达式**只应用于** `Event::CodeBlock` 内部的内容
- 过滤常见句首大写单词（The、This、That 等）
- 确保不会误匹配正文中的普通文本

### 3. 正则缓存优化
**实现**：使用 `std::sync::OnceLock` 缓存正则表达式
```rust
fn get_function_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(fn|function|def)\s+(\w+)\s*\(").unwrap()
    })
}
```

**性能优势**：
- 正则表达式只编译一次，全局复用
- 避免在循环中重复编译相同的模式
- 符合 Rust 零成本抽象的原则

### 4. PR 文档完善
**实现**：准备详细的 Pull Request 模板
- 变更说明：详细描述 Issue #10 的实现目标和核心功能
- 变更类型：勾选所有适用选项（新功能、性能优化、文档更新、重构）
- 任务清单：确认所有质量门禁通过（fmt、clippy、test）
- 测试说明：提供完整测试命令和覆盖情况（9 个单元测试、45/45 通过）
- 性能提升对比表：列出 O(N)→O(log N)、OnceLock 缓存、零拷贝等优化
- 相关文档：链接到 Issue #10、OpenSpec 提案、规范变更文档
- 文件变更清单：详细列出所有修改的文件和功能点
- 破坏性变更说明：确认向后兼容，无迁移成本
- 补充说明：突出架构师建议采纳（多值字段插入、严防正则污染）
