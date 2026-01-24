# 实现任务清单

## 任务概览

根据 Issue #3 的要求，本次实现分为三个主要任务：

- **Task-01**: 数据结构更新（`ParsedDoc` 添加 `sections`，`KnowledgeRecord` 添加 `source_path`）
- **Task-02**: 存储逻辑实现（重写 `add()` 以扁平化切片为记录）
- **Task-03**: 验证测试（添加单元测试确保功能正确）

---

## Task-01: 结构体更新

**上下文**:
- `packages/core/src/lib.rs` - 导出和结构体定义
- `packages/core/src/storage/mod.rs` - 存储相关结构体
- `packages/core/src/parser/mod.rs` - 解析逻辑
- `packages/cli/src/main.rs` - CLI 模块（可能需要修复编译错误）

**子任务**:
- [ ] 1.1 在 `packages/core/src/lib.rs` 中导入 `SlicedDoc`
  - 添加 `pub use parser::{slice_by_headers, SlicedDoc};`
- [ ] 1.2 在 `ParsedDoc` 结构体中添加 `pub sections: Vec<SlicedDoc>` 字段
  - 位置：`packages/core/src/parser/mod.rs` 或 `lib.rs`（根据当前定义位置）
- [ ] 1.3 处理生命周期问题
  - `SlicedDoc<'a>` 有生命周期参数，需要决定 `ParsedDoc` 的生命周期策略
  - **方案 A**：让 `ParsedDoc` 也携带生命周期 `ParsedDoc<'a>`
  - **方案 B**：将 `SlicedDoc` 改为拥有所有权（移除生命周期，复制数据）
  - **建议**：先尝试方案 A，如果 CLI 或存储层使用过于复杂，再考虑方案 B
- [ ] 1.4 在 `KnowledgeRecord` 结构体中添加 `pub source_path: String` 字段
  - 位置：`packages/core/src/storage/mod.rs`
- [ ] 1.5 修改所有 `KnowledgeRecord` 初始化代码
  - 检查 `storage/mod.rs` 中的 `add()` 方法
  - 确保每次创建 `KnowledgeRecord` 时都传入 `source_path`
- [ ] 1.6 修改 `parse_markdown()` 函数以填充 `sections` 字段
  - 在解析完成后调用 `slice_by_headers(&content, &title)`
  - 将结果赋值给 `doc.sections`
- [ ] 1.7 检查并修复 CLI 模块中的编译错误
  - 运行 `cargo build --bin contextfy-cli`
  - 修复任何因结构体字段变更导致的错误
- [ ] 1.8 运行 `cargo test -p contextfy-core` 确保没有破坏现有测试

**预期产出**:
- `ParsedDoc` 包含 `sections: Vec<SlicedDoc>` 字段
- `KnowledgeRecord` 包含 `source_path: String` 字段
- 所有模块编译通过，无警告

---

## Task-02: 存储逻辑实现

**上下文**:
- `packages/core/src/storage/mod.rs` - `KnowledgeStore::add()` 方法

**子任务**:
- [ ] 2.1 分析当前 `add()` 方法的实现逻辑
  - 当前：创建 1 个 `KnowledgeRecord`，序列化为 JSON，写入文件
- [ ] 2.2 重写 `add()` 方法以支持切片存储
  ```rust
  pub async fn add(&self, doc: &ParsedDoc) -> Result<Vec<String>> {
      let mut ids = Vec::new();

      if doc.sections.is_empty() {
          // 回退逻辑：存储整个文档为 1 条记录
          let id = Uuid::new_v4().to_string();
          let record = KnowledgeRecord {
              id: id.clone(),
              title: doc.title.clone(),
              summary: doc.summary.clone(),
              content: doc.content.clone(),
              source_path: doc.path.clone(),  // 新增字段
          };
          // 序列化并写入文件...
          ids.push(id);
      } else {
          // 新逻辑：为每个切片创建独立记录
          for slice in &doc.sections {
              let id = Uuid::new_v4().to_string();
              let record = KnowledgeRecord {
                  id: id.clone(),
                  title: slice.section_title.clone(),
                  summary: slice.content.chars().take(200).collect::<String>(),
                  content: slice.content.to_string(),  // 注意：可能需要复制数据
                  source_path: doc.path.clone(),
              };
              // 序列化并写入文件...
              ids.push(id);
          }
      }

      Ok(ids)  // 返回所有切片的 ID
  }
  ```
- [ ] 2.3 处理生命周期和数据所有权问题
  - `SlicedDoc.content` 是 `&str`，存储时需要转换为 `String`
  - 使用 `.to_string()` 复制数据（损失零拷贝优势，但 JSON 序列化不可避免）
- [ ] 2.4 更新方法签名返回类型
  - 从 `Result<String>` 改为 `Result<Vec<String>>`（返回所有切片的 ID）
- [ ] 2.5 修复调用点
  - CLI 模块中的 `store.add(&doc).await?` 需要适配新的返回类型
- [ ] 2.6 添加错误处理
  - 处理空切片、空内容等边界情况
- [ ] 2.7 编写临时调试日志
  - 打印存储的切片数量和 ID 列表

**预期产出**:
- `add()` 方法能将 `ParsedDoc.sections` 扁平化为多条记录
- 每条记录包含 `source_path` 字段
- 返回所有切片的 UUID 列表

---

## Task-03: 验证测试

**上下文**:
- `packages/core/src/storage/mod.rs` - 单元测试

**子任务**:
- [ ] 3.1 编写单元测试 `test_add_sliced_doc`
  ```rust
  #[tokio::test]
  async fn test_add_sliced_doc() {
      // 创建临时测试目录
      let temp_dir = tempfile::tempdir().unwrap();
      let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap()).unwrap();

      // 手动构造包含 2 个切片的 ParsedDoc
      let doc = ParsedDoc {
          path: "/fake/path.md".to_string(),
          title: "Test Doc".to_string(),
          summary: "Test summary".to_string(),
          content: "Full content".to_string(),
          sections: vec![
              SlicedDoc {
                  section_title: "Section 1".to_string(),
                  content: "Content 1",
                  parent_doc_title: "Test Doc",
              },
              SlicedDoc {
                  section_title: "Section 2".to_string(),
                  content: "Content 2",
                  parent_doc_title: "Test Doc",
              },
          ],
      };

      // 调用 add()
      let ids = store.add(&doc).await.unwrap();

      // 断言：返回 2 个 ID
      assert_eq!(ids.len(), 2);

      // 断言：存储目录中有 2 个 JSON 文件
      let json_files: Vec<_> = fs::read_dir(temp_dir.path())
          .unwrap()
          .filter_map(|e| e.ok())
          .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
          .collect();
      assert_eq!(json_files.len(), 2);

      // 断言：每个记录都有正确的 source_path
      for json_file in json_files {
          let content = fs::read_to_string(json_file.path()).unwrap();
          let record: KnowledgeRecord = serde_json::from_str(&content).unwrap();
          assert_eq!(record.source_path, "/fake/path.md");
      }
  }
  ```
- [ ] 3.2 编写边界情况测试 `test_add_empty_sections`
  - 测试 `sections` 为空时的回退逻辑
  - 确保整个文档作为 1 条记录存储
- [ ] 3.3 编写鲁棒性测试 `test_storage_robustness` (极端情况)
  ```rust
  #[tokio::test]
  async fn test_storage_robustness() {
      let temp_dir = tempfile::tempdir().unwrap();
      let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap()).unwrap();

      // 构造极端数据
      let mut sections = vec![
          // Case A: 标题为空，内容包含 Emoji 和特殊符号
          SlicedDoc {
              section_title: "".to_string(),
              content: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
              parent_doc_title: "Edge Case Doc",
          },
          // Case B: 只有标题，内容为空
          SlicedDoc {
              section_title: "Empty Content".to_string(),
              content: "".to_string(),
              parent_doc_title: "Edge Case Doc",
          },
      ];

      // Case C: 大量切片 (模拟长文) - 循环生成 50 个切片
      for i in 0..50 {
          sections.push(SlicedDoc {
              section_title: format!("Section {}", i),
              content: format!("Content for section {}", i),
              parent_doc_title: "Edge Case Doc",
          });
      }

      let doc = ParsedDoc {
          path: "C:\\Windows\\System32\\weird_path.md".to_string(), // Windows 路径反斜杠测试
          title: "Edge Case Doc".to_string(),
          summary: "".to_string(),
          content: "".to_string(),
          sections,
      };

      // 验证是否能成功写入，不 Panic
      let ids = store.add(&doc).await.unwrap();

      // 验证 Case C: 确保生成的 ID 数量正确 (2个手动 + 50个循环 = 52)
      assert_eq!(ids.len(), 52);

      // 验证 JSON 读取回来的数据完整性 (确保特殊字符没有乱码)
      // 读取第一个文件，反序列化，断言 content == "🚀 Emoji & \"Quotes\" & \nNewlines"
      let first_record = store.get(&ids[0]).await.unwrap().unwrap();
      assert_eq!(first_record.content, "🚀 Emoji & \"Quotes\" & \nNewlines");
      assert_eq!(first_record.source_path, "C:\\Windows\\System32\\weird_path.md");
  }
  ```
- [ ] 3.4 编写端到端集成测试
  - 使用真实的 markdown 文件
  - 调用 `parse_markdown()` → `store.add()` → 验证存储结果
- [ ] 3.5 运行所有测试并确保通过
  ```bash
  cargo test -p contextfy-core
  ```
- [ ] 3.6 运行代码格式化和静态检查
  ```bash
  cargo fmt
  cargo clippy -p contextfy-core
  ```
- [ ] 3.7 手动测试 CLI 流程
  ```bash
  cd /home/haotang/my-project/contextfy/Kit
  cargo build --bin contextfy-cli
  # 创建测试文档并运行 contextfy build
  # 检查 .contextfy/data/ 目录中的 JSON 文件数量
  ```

**预期产出**:
- 单元测试覆盖主要路径和边界情况
- 所有测试通过
- 代码通过 fmt 和 clippy 检查

---

## 实现亮点记录

完成所有任务后，在此处记录实现亮点和技术决策：

_（留待实现完成后填写）_

### 设计决策
- _（记录关键技术选择，如生命周期处理、错误处理策略等）_

### 已知问题
- _（记录任何遗留问题或限制）_

### 后续优化方向
- _（记录未来可以改进的地方）_
