use crate::parser::ParsedDoc;
use anyhow::Result;
// use lancedb::connect;
// use arrow::array::{StringArray, StringBuilder};
// use arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
// use arrow::record_batch::RecordBatch;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// 知识库中的一条记录
///
/// # 字段
///
/// * `id` - 记录的唯一标识符（UUID）
/// * `title` - 记录标题（对于切片文档，这是 H2 标题）
/// * `summary` - 内容摘要（前 200 个字符）
/// * `content` - 完整内容
/// * `source_path` - 原始文件路径，用于追溯源文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub source_path: String, // 新增字段：记录原始文件路径
}

pub struct KnowledgeStore {
    data_dir: String,
}

impl KnowledgeStore {
    pub fn new(data_dir: &str) -> Result<Self> {
        fs::create_dir_all(data_dir)?;
        Ok(KnowledgeStore {
            data_dir: data_dir.to_string(),
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<KnowledgeRecord>> {
        let mut records = Vec::new();

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    if record.title.to_lowercase().contains(&query.to_lowercase())
                        || record
                            .summary
                            .to_lowercase()
                            .contains(&query.to_lowercase())
                    {
                        records.push(record);
                    }
                }
            }
        }

        Ok(records)
    }

    pub async fn get(&self, id: &str) -> Result<Option<KnowledgeRecord>> {
        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    if record.id == id {
                        return Ok(Some(record));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn add(&self, doc: &ParsedDoc) -> Result<Vec<String>> {
        let mut ids = Vec::new();

        if doc.sections.is_empty() {
            // 回退逻辑：如果文档没有切片，将整个文档作为单条记录存储
            // 这种情况可能出现在：
            // 1. 文档没有 H2 标题
            // 2. 旧版本解析的文档（向后兼容）
            let id = Uuid::new_v4().to_string();
            let record = KnowledgeRecord {
                id: id.clone(),
                title: doc.title.clone(),
                summary: doc.summary.clone(),
                content: doc.content.clone(),
                source_path: doc.path.clone(),
            };

            let json = serde_json::to_string_pretty(&record)?;
            fs::write(Path::new(&self.data_dir).join(format!("{}.json", id)), json)?;
            ids.push(id);
        } else {
            // 新逻辑：为每个切片创建独立的记录
            // 这样可以实现细粒度的检索，提升搜索精度
            for slice in &doc.sections {
                let id = Uuid::new_v4().to_string();

                // 性能考虑：SlicedSection 已经拥有所有权，这里直接使用即可
                //
                // 为什么 ParsedDoc 使用拥有所有权的 SlicedSection？
                // - 简化生命周期管理：ParsedDoc 无需生命周期参数
                // - 避免"返回局部变量借用"的问题
                // - 在存储层（JSON 序列化）零拷贝优势无法体现
                //
                // TODO(优化): 当前为每个切片分配新的 String 对象
                // 如果性能分析显示批量索引时这里是瓶颈，可以考虑：
                // 1. 使用 Cow<'a, str> 在 KnowledgeRecord 中实现零拷贝
                // 2. 延迟序列化，先在内存中累积记录
                // 3. 使用流式 JSON 序列化器避免中间缓冲区
                //
                // 权衡：内存分配开销 vs 代码复杂度
                // 当前选择：优先代码简洁性，牺牲一定的性能

                let record = KnowledgeRecord {
                    id: id.clone(),
                    title: slice.section_title.clone(),
                    summary: slice.content.chars().take(200).collect::<String>(),
                    content: slice.content.clone(), // SlicedSection 拥有所有权，直接克隆
                    source_path: doc.path.clone(),
                };

                let json = serde_json::to_string_pretty(&record)?;
                fs::write(Path::new(&self.data_dir).join(format!("{}.json", id)), json)?;
                ids.push(id);
            }
        }

        Ok(ids) // 返回所有切片的 ID（如果有切片）或单个文档 ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SlicedSection;
    use std::fs;

    /// 基本切片存储测试
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
                SlicedSection {
                    section_title: "Section 1".to_string(),
                    content: "Content 1".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                },
                SlicedSection {
                    section_title: "Section 2".to_string(),
                    content: "Content 2".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
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

    /// 空切片回退测试
    #[tokio::test]
    async fn test_add_empty_sections() {
        // 创建临时测试目录
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap()).unwrap();

        // 构造没有切片的 ParsedDoc（回退逻辑）
        let doc = ParsedDoc {
            path: "/legacy/doc.md".to_string(),
            title: "Legacy Doc".to_string(),
            summary: "Legacy summary".to_string(),
            content: "Full legacy content".to_string(),
            sections: vec![], // 空切片，触发回退逻辑
        };

        // 调用 add()
        let ids = store.add(&doc).await.unwrap();

        // 断言：返回 1 个 ID（整篇文档作为单条记录）
        assert_eq!(ids.len(), 1);

        // 断言：存储目录中有 1 个 JSON 文件
        let json_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(json_files.len(), 1);

        // 断言：记录的 title 是文档标题（而非切片标题）
        let content = fs::read_to_string(json_files[0].path()).unwrap();
        let record: KnowledgeRecord = serde_json::from_str(&content).unwrap();
        assert_eq!(record.title, "Legacy Doc");
        assert_eq!(record.source_path, "/legacy/doc.md");
    }

    /// 鲁棒性测试（极端情况）
    #[tokio::test]
    async fn test_storage_robustness() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap()).unwrap();

        // 构造极端数据
        let mut sections = vec![
            // Case A: 标题为空，内容包含 Emoji 和特殊符号
            SlicedSection {
                section_title: "".to_string(),
                content: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
            },
            // Case B: 只有标题，内容为空
            SlicedSection {
                section_title: "Empty Content".to_string(),
                content: "".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
            },
        ];

        // Case C: 大量切片 (模拟长文) - 循环生成 50 个切片
        for i in 0..50 {
            sections.push(SlicedSection {
                section_title: format!("Section {}", i),
                content: format!("Content for section {}", i),
                parent_doc_title: "Edge Case Doc".to_string(),
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
        assert_eq!(
            first_record.source_path,
            "C:\\Windows\\System32\\weird_path.md"
        );

        // 验证 Case B: 空内容切片也能正确存储
        let second_record = store.get(&ids[1]).await.unwrap().unwrap();
        assert_eq!(second_record.title, "Empty Content");
        assert_eq!(second_record.content, "");

        // 验证 Case C: 所有 ID 都是唯一的（通过集合去重后数量不变）
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 52);
    }
}
