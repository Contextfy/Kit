## TDD 任务列表

### Task-01: 设置与测试框架
- [x] 1.1 在 `packages/core/src/parser/mod.rs` 中定义 `SlicedDoc` 结构体
  - 字段：`pub section_title: String`, `pub content: String`, `pub parent_doc_title: String`
  - 确保所有字段为公开，以便其他模块使用
- [x] 1.2 创建单元测试文件 `packages/core/src/parser/slicing_tests.rs`（或在 mod.rs 中添加测试模块）
- [x] 1.3 编写测试用例（覆盖边界情况）：
  - **测试 1 - 标准**：3 个 H2 标题 → 3 个切片
  - **测试 2 - 无标题**：无 H2 → 空向量
  - **测试 3 - 嵌套**：H2 包含 H3 → H3 归属于父 H2
  - **测试 4 - 代码块陷阱**：代码块内的 `##` 不应触发切片（验证 AST 解析能力）
  - **测试 5 - 空内容**：连续的 `##` 标题 → 前一个切片内容为空
  - **测试 6 - Unicode**：包含中文和 Emoji 的标题与内容 → 字节切片不 Panic

### Task-02: 实现切片逻辑
- [x] 2.1 在 `packages/core/src/parser/mod.rs` 中实现 `pub fn slice_by_headers(content: &str, parent_title: &str) -> Vec<SlicedDoc>`
- [x] 2.2 使用 `pulldown-cmark` crate 遍历 markdown 事件
- [x] 2.3 检测 `Event::Start(Tag::Heading(HeadingLevel::H2, ...))` 事件
- [x] 2.4 捕获 H2 标题之间的内容
- [x] 2.5 处理边界情况：
  - 忽略第一个 H2 之前的内容
  - 将 H3/H4 标题作为当前 H2 切片内容的一部分
  - 无 H2 标题时返回空向量
- [x] 2.6 运行测试，确保所有测试通过（TDD 绿灯阶段）

### Task-03: 公开导出
- [x] 3.1 检查 `packages/core/src/lib.rs`，确定是否需要重新导出 `SlicedDoc` 和 `slice_by_headers`
- [x] 3.2 如需导出，在 `lib.rs` 中添加 `pub use parser::{SlicedDoc, slice_by_headers};`
- [x] 3.3 运行 `cargo test` 确保公共 API 可访问
- [x] 3.4 运行 `cargo clippy` 检查代码质量
- [x] 3.5 运行 `cargo fmt` 格式化代码

### 验收检查清单
- [x] ✅ 包含 3 个 H2 标题的文档 → 返回 3 个切片
- [x] ✅ 没有标题的文档 → 返回空向量（0 个切片）
- [x] ✅ 包含嵌套 H3 的文档 → H3 内容包含在父 H2 切片中
- [x] ✅ `parse_markdown()` 函数未被修改
- [x] ✅ 所有单元测试通过（6/6 切片测试）
- [x] ✅ 代码通过 clippy 检查
- [x] ✅ 代码格式化完成

### 实现亮点
- ✅ 使用 `into_offset_iter()` 实现零拷贝切片
- ✅ 完整的 AST 解析，正确处理代码块中的 `##`
- ✅ Unicode 和 Emoji 支持，无 Panic
- ✅ TDD 驱动开发，红-绿-重构循环
