# Change: Store Document Slices as Independent Records

## Why

In Issue #1 (feat-markdown-slicing), we implemented H2 header-based semantic slicing functionality with `slice_by_headers()` returning `Vec<SlicedDoc>`. However, the current storage system has the following issues:

1. **ParsedDoc does not carry slice information**: The parsed document structure does not include slice results, preventing the slicing functionality from integrating with the storage layer
2. **Storage granularity too coarse**: Entire documents are stored as single JSON files, unable to achieve fine-grained block-level retrieval
3. **Missing source path tracking**: Stored records lack the `source_path` field, making it impossible to trace back to the original file path
4. **Does not match hierarchical retrieval model**: The parent-child relationship hierarchical retrieval defined in the specification cannot be implemented with the current flat storage model

This change aims to connect the parser slicing functionality with the storage layer, enabling true fine-grained storage and retrieval.

## What Changes

**Issue #3 边界**：本次变更专注于数据结构更新和存储逻辑修改，严格限制在 Storage 层面

### 核心变更点

#### 1. 数据结构更新
- **`ParsedDoc`** 新增字段 `pub sections: Vec<SlicedDoc>`
  - 在 `packages/core/src/lib.rs` 中修改（需导入 `SlicedDoc`）
  - 由 `parse_markdown()` 函数在解析时填充切片结果
- **`KnowledgeRecord`** 新增字段 `pub source_path: String`
  - 在 `packages/core/src/storage/mod.rs` 中修改
  - 存储原始文件路径，支持反向追溯

#### 2. 解析逻辑增强
- 修改 `parse_markdown()` 函数：
  - 调用 `slice_by_headers()` 生成切片
  - 将切片结果赋值给 `ParsedDoc.sections`
  - 处理生命周期参数（`SlicedDoc` 使用零拷贝借用）

#### 3. 存储逻辑重构
- 修改 `KnowledgeStore::add()` 方法：
  - **当前逻辑**：存储 1 条记录 per `ParsedDoc`
  - **新逻辑**：
    - 遍历 `doc.sections`
    - 为每个切片创建独立的 `KnowledgeRecord`
    - 使用切片的 `section_title` 作为 `title`
    - 使用切片的 `content` 作为 `content`
    - 在每个记录中存储 `source_path`（原始文件路径）
    - 为每个切片生成唯一的 UUID
  - **回退逻辑**：如果 `sections` 为空（旧文档），存储整个文档为 1 条记录

### 不在本次范围
- **向量嵌入和语义搜索**（留待后续 Issue）
- **LanceDB 集成**（当前继续使用文件系统 JSON 存储）
- **增量构建的哈希跟踪**（留待后续 Issue）
- **`parent_id`、`is_parent`、`position` 字段**（不在 Issue #3 要求中，避免过度设计）
- **检索逻辑修改**（`search()` 和 `get()` 方法暂不修改）

## 影响

### 受影响的规格
- `specs/core-engine/spec.md` - Knowledge Storage 需求（部分场景）

### 受影响的代码
- `packages/core/src/lib.rs`
  - 导入 `SlicedDoc`
  - `ParsedDoc` 结构体定义
- `packages/core/src/parser/mod.rs`
  - `parse_markdown()` 函数实现
- `packages/core/src/storage/mod.rs`
  - `KnowledgeRecord` 结构体定义
  - `KnowledgeStore::add()` 方法实现
- `packages/cli/src/main.rs`
  - 可能需要修复结构体初始化相关的编译错误

### 破坏性变更
- ✅ **向后兼容**：新增字段不影响现有 JSON 存储文件（但需要重新生成以利用切片功能）
- ⚠️ **API 变更**：`KnowledgeRecord` 新增必填字段 `source_path`，需修改所有初始化代码
- ⚠️ **存储格式变更**：单个文档现在产生多条 JSON 记录，旧数据需要迁移

## 实现策略

1. **TDD 驱动开发**：先编写单元测试 `test_add_sliced_doc`，定义期望行为
2. **渐进式修改**：
   - 先更新数据结构（Task 01）
   - 再修改存储逻辑（Task 02）
   - 最后验证端到端功能（Task 03）
3. **确保 CLI 编译通过**：在修改 `ParsedDoc` 后，检查并修复 CLI 模块中的相关代码
4. **向后兼容处理**：`sections` 为空时的回退逻辑确保旧文档仍可存储
