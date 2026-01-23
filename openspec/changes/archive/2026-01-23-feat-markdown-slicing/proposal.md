# 变更：使用 H2 标题实现 Markdown 语义切片

## 为什么

长篇 markdown 文档（如 API 文档、技术规范）当前作为单个单元存储，导致检索精度较低。当用户查询特定主题时，他们收到的是完整文档而非相关章节，浪费了 token 并降低了相关性评分。使用 H2 标题进行语义切片将：

1. **提升检索精度** - 通过将文档分解为语义上有意义的块
2. **减少 token 浪费** - 在检索时仅返回相关章节
3. **支持细粒度上下文** - 使 AI 智能体能够处理特定主题
4. **对齐文档结构** - 因为大多数技术文档使用 H2 标题作为主要章节边界

## 变更内容

**Issue #1 边界**：本次变更仅实现解析器层面的切片功能，不涉及存储和检索。

- **新增 (ADDED)**：`Semantic Chunking` 需求 - 在 `packages/core/src/parser/` 模块中实现基于 H2 标题的文档切片
- **保持不变**：`parse_markdown()` 函数不作修改
- **不在本次范围**：存储 schema 更新（留待 Issue #3）、检索逻辑更新（留待后续）

### 实现细节

**目标模块**：`packages/core/src/parser/`

**1. 结构体定义** (`packages/core/src/parser/mod.rs`):
```rust
pub struct SlicedDoc {
    pub section_title: String,    // H2 标题文本
    pub content: String,          // 该 H2 下的完整内容
    pub parent_doc_title: String, // 父文档的 H1 标题
}
```

**2. 切片函数** (`packages/core/src/parser/mod.rs`):
```rust
pub fn slice_by_headers(content: &str, parent_title: &str) -> Vec<SlicedDoc>
```

**逻辑要求**：
- 使用 `pulldown-cmark` crate（已在 Cargo.toml 中）
- 遍历 markdown 事件，检测 `Event::Start(Tag::Heading(HeadingLevel::H2, ...))`
- 捕获 H2 标题之间的内容
- **边界处理**：
  - 第一个 H2 之前的内容：忽略
  - H3/H4 标题：作为当前 H2 切片的内容的一部分
  - 无 H2 标题：返回空向量

**3. 公开导出** (`packages/core/src/lib.rs`):
- 如有需要，重新导出 `SlicedDoc` 和 `slice_by_headers` 以供公共使用

### 验收标准

单元测试覆盖：
- ✅ 包含 3 个 H2 标题的文档 → 返回 3 个切片
- ✅ 没有标题的文档 → 返回空向量（0 个切片）
- ✅ 包含嵌套 H3 的文档 → H3 内容包含在父 H2 切片中

### 影响范围

- **受影响的规格**：`core-engine`（解析模块）
- **受影响的代码**：
  - `packages/core/src/parser/mod.rs`（定义 `SlicedDoc` 和 `slice_by_headers`）
  - `packages/core/src/lib.rs`（可能的公开导出）
- **不受影响**：
  - `parse_markdown()` 函数保持不变
  - `packages/core/src/storage.rs`（存储逻辑留待 Issue #3）
  - `packages/core/src/retrieval.rs`（检索逻辑留待后续）

### 后续工作

- **Issue #3**：将切片结果存储到 LanceDB
- **Issue #N**：更新检索逻辑以支持块级检索
