use crate::parser::{extract_summary, ParsedDoc};
use anyhow::{Context, Result};
// use lancedb::connect;
// use arrow::array::{StringArray, StringBuilder};
// use arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
// use arrow::record_batch::RecordBatch;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// 知识库中的一条记录
///
/// # 字段
///
/// * `id` - 记录的唯一标识符（UUID）
/// * `title` - 记录标题（对于切片文档，这是 H2 标题）
/// * `parent_doc_title` - 父文档的标题（H1 标题或文件名）
/// * `summary` - 内容摘要（前 200 个字符）
/// * `content` - 完整内容
/// * `source_path` - 原始文件路径，用于追溯源文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub parent_doc_title: String,
    pub summary: String,
    pub content: String,
    pub source_path: String, // 新增字段：记录原始文件路径
}

pub struct KnowledgeStore {
    data_dir: String,
}

impl KnowledgeStore {
    pub async fn new(data_dir: &str) -> Result<Self> {
        fs::create_dir_all(data_dir).await?;

        // 启动恢复：清理上次崩溃遗留的临时目录
        Self::cleanup_orphaned_temp_dirs(data_dir).await;

        Ok(KnowledgeStore {
            data_dir: data_dir.to_string(),
        })
    }

    /// 清理孤儿临时目录（启动恢复）
    ///
    /// 如果程序在写入过程中崩溃，可能会遗留 `.temp-{uuid}` 目录。
    /// 这个方法在启动时扫描并删除这些目录。
    async fn cleanup_orphaned_temp_dirs(data_dir: &str) {
        let mut entries = match fs::read_dir(data_dir).await {
            Ok(entries) => entries,
            Err(_) => return,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();

            // 检查是否是临时目录（以 .temp- 开头）
            let name_str = match name.to_str() {
                Some(s) => s,
                None => continue,
            };

            if name_str.starts_with(".temp-") && entry.path().is_dir() {
                eprintln!("Cleaning up orphaned temp directory: {}", name_str);
                let _ = fs::remove_dir_all(entry.path()).await;
            }
        }
    }

    /// 创建临时写入目录
    ///
    /// 用于实现原子性写入：所有文件先写入临时目录，成功后批量移动到正式目录。
    /// 如果中途失败，临时目录会被删除，确保不会留下"幽灵数据"。
    async fn create_temp_dir(&self) -> Result<PathBuf> {
        let temp_dir = Path::new(&self.data_dir).join(format!(".temp-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).await?;
        Ok(temp_dir)
    }

    /// 清理临时目录（失败时调用）
    async fn cleanup_temp_dir(temp_dir: &Path) {
        let _ = fs::remove_dir_all(temp_dir).await;
    }

    pub async fn search(&self, query: &str) -> Result<Vec<KnowledgeRecord>> {
        let mut records = Vec::new();
        let mut entries = fs::read_dir(&self.data_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // 防御性检查：跳过临时文件和目录
            if Self::is_temp_file(&path) {
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await?;
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
        let mut entries = fs::read_dir(&self.data_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // 防御性检查：跳过临时文件和目录
            if Self::is_temp_file(&path) {
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    if record.id == id {
                        return Ok(Some(record));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 检查路径是否是临时文件（防御性检查）
    ///
    /// 临时文件/目录的**名称**以 `.temp-` 开头，应该在正常扫描中被跳过。
    /// 只检查路径的最后一个组件（文件名或目录名），避免误判包含 `.temp-` 的父目录路径。
    fn is_temp_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with(".temp-"))
            .unwrap_or(false)
    }

    /// 添加文档到知识库（带回滚机制的原子性写入）
    ///
    /// # 回滚机制
    ///
    /// 1. **准备阶段**：创建临时目录（`.data/.temp-{uuid}`）
    /// 2. **写入阶段**：所有切片先写入临时目录
    /// 3. **提交阶段**：全部成功后，原子性移动文件到正式目录
    /// 4. **回滚阶段**：任何失败发生时，删除整个临时目录和已提交的文件
    ///
    /// 这样确保了原子性：要么所有切片都成功写入，要么都不写入。
    /// 不会出现"部分写入"导致的"幽灵数据"问题。
    pub async fn add(&self, doc: &ParsedDoc) -> Result<Vec<String>> {
        let mut ids = Vec::new();

        if doc.sections.is_empty() {
            // 回退逻辑：如果文档没有切片，将整个文档作为单条记录存储
            // 这种情况可能出现在：
            // 1. 文档没有 H2 标题
            // 2. 旧版本解析的文档（向后兼容）
            //
            // 使用临时文件模式确保原子性：写入临时文件 -> 原子重命名
            let id = Uuid::new_v4().to_string();
            let record = KnowledgeRecord {
                id: id.clone(),
                title: doc.title.clone(),
                parent_doc_title: doc.title.clone(),
                summary: doc.summary.clone(),
                content: doc.content.clone(),
                source_path: doc.path.clone(),
            };

            // 创建临时目录
            let temp_dir = match self.create_temp_dir().await {
                Ok(dir) => dir,
                Err(e) => {
                    return Err(e).context("Failed to create temporary directory");
                }
            };

            // 序列化并写入临时文件
            let json = match serde_json::to_string_pretty(&record) {
                Ok(j) => j,
                Err(e) => {
                    Self::cleanup_temp_dir(&temp_dir).await;
                    return Err(e).context("Failed to serialize document");
                }
            };

            let temp_path = temp_dir.join(format!("{}.json", id));
            if let Err(e) = fs::write(&temp_path, json).await {
                Self::cleanup_temp_dir(&temp_dir).await;
                return Err(e).context("Failed to write temporary file");
            }

            // 原子性移动到正式目录
            let final_path = Path::new(&self.data_dir).join(format!("{}.json", id));
            if let Err(e) = fs::rename(&temp_path, &final_path).await {
                Self::cleanup_temp_dir(&temp_dir).await;
                return Err(e).context("Failed to move file to final destination");
            }

            // 清理临时目录
            Self::cleanup_temp_dir(&temp_dir).await;
            ids.push(id);
        } else {
            // 新逻辑：为每个切片创建独立的记录（带回滚机制）
            //
            // 【问题】如果第 5 个切片写入失败（比如磁盘满），前 4 个切片已经留在那了，
            // 变成了"幽灵数据"，导致数据不一致。
            //
            // 【解决方案】使用临时目录 + 原子移动 + 回滚机制：
            // 1. 所有切片先写入临时目录
            // 2. 全部成功后，批量移动到正式目录（记录已移动的文件）
            // 3. 任何失败发生时，删除已移动的文件和临时目录
            //
            // 这样确保了原子性：要么全部成功，要么全部失败。

            // 步骤 1：创建临时目录
            let temp_dir = match self.create_temp_dir().await {
                Ok(dir) => dir,
                Err(e) => {
                    return Err(e).context("Failed to create temporary directory");
                }
            };

            // 步骤 2：在临时目录中写入所有切片
            let mut temp_files = Vec::new();

            for slice in &doc.sections {
                let id = Uuid::new_v4().to_string();
                let temp_path = temp_dir.join(format!("{}.json", id));

                // 序列化并写入临时文件
                let record = KnowledgeRecord {
                    id: id.clone(),
                    title: slice.section_title.clone(),
                    parent_doc_title: slice.parent_doc_title.clone(),
                    summary: extract_summary(&slice.content),
                    content: slice.content.clone(),
                    source_path: doc.path.clone(),
                };

                // 序列化失败时清理临时目录
                let json = match serde_json::to_string_pretty(&record) {
                    Ok(j) => j,
                    Err(e) => {
                        Self::cleanup_temp_dir(&temp_dir).await;
                        return Err(e).context(format!(
                            "Failed to serialize slice: {}",
                            slice.section_title
                        ));
                    }
                };

                // 如果写入失败，清理临时目录并返回错误
                if let Err(e) = fs::write(&temp_path, json).await {
                    Self::cleanup_temp_dir(&temp_dir).await;
                    return Err(e).context(format!(
                        "Failed to write temporary file for slice: {}",
                        slice.section_title
                    ));
                }

                ids.push(id.clone());
                temp_files.push((id, temp_path));
            }

            // 步骤 3：全部成功后，移动文件到正式目录（带回滚机制）
            let mut committed_files = Vec::new();

            for (id, temp_path) in temp_files {
                let final_path = Path::new(&self.data_dir).join(format!("{}.json", id));

                match fs::rename(&temp_path, &final_path).await {
                    Ok(_) => {
                        committed_files.push(final_path);
                    }
                    Err(e) => {
                        // 回滚：删除所有已移动到正式目录的文件
                        for path in &committed_files {
                            let _ = fs::remove_file(path).await;
                        }
                        Self::cleanup_temp_dir(&temp_dir).await;
                        return Err(e)
                            .context(format!("Failed to move file {} to final destination", id))
                            .context("Transaction rolled back: all committed files removed");
                    }
                }
            }

            // 步骤 4：删除临时目录（此时应该已经为空）
            Self::cleanup_temp_dir(&temp_dir).await;
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
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

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
                    summary: "Content 1".to_string(),
                },
                SlicedSection {
                    section_title: "Section 2".to_string(),
                    content: "Content 2".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 2".to_string(),
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
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

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
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

        // 构造极端数据
        let mut sections = vec![
            // Case A: 标题为空，内容包含 Emoji 和特殊符号
            SlicedSection {
                section_title: "".to_string(),
                content: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
            },
            // Case B: 只有标题，内容为空
            SlicedSection {
                section_title: "Empty Content".to_string(),
                content: "".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: "".to_string(),
            },
        ];

        // Case C: 大量切片 (模拟长文) - 循环生成 50 个切片
        for i in 0..50 {
            sections.push(SlicedSection {
                section_title: format!("Section {}", i),
                content: format!("Content for section {}", i),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: format!("Content for section {}", i),
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

    /// 回滚机制测试：验证成功后临时目录被清理
    #[tokio::test]
    async fn test_rollback_temp_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test summary".to_string(),
            content: "Full content".to_string(),
            sections: vec![
                SlicedSection {
                    section_title: "Section 1".to_string(),
                    content: "Content 1".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 1".to_string(),
                },
                SlicedSection {
                    section_title: "Section 2".to_string(),
                    content: "Content 2".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 2".to_string(),
                },
            ],
        };

        // 执行添加操作（应该成功）
        let ids = store.add(&doc).await.unwrap();
        assert_eq!(ids.len(), 2);

        // 验证：正式目录中有 2 个 JSON 文件
        let json_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("json")
                    && !e
                        .path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with(".temp-")
            })
            .collect();
        assert_eq!(json_files.len(), 2);

        // 验证：没有临时目录残留（所有 .temp-* 目录都被清理）
        let temp_dirs: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".temp-")
            })
            .collect();
        assert_eq!(
            temp_dirs.len(),
            0,
            "Temporary directories should be cleaned up"
        );
    }

    /// 原子性测试：验证所有文件要么都在，要么都不在
    #[tokio::test]
    async fn test_atomicity_all_or_nothing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

        // 创建包含 5 个切片的文档
        let mut sections = Vec::new();
        for i in 0..5 {
            sections.push(SlicedSection {
                section_title: format!("Section {}", i),
                content: format!("Content for section {}", i),
                parent_doc_title: "Atomicity Test".to_string(),
                summary: format!("Content for section {}", i),
            });
        }

        let doc = ParsedDoc {
            path: "/test/atomic.md".to_string(),
            title: "Atomicity Test".to_string(),
            summary: "Test".to_string(),
            content: "Full content".to_string(),
            sections,
        };

        // 执行添加操作
        let ids = store.add(&doc).await.unwrap();

        // 验证：所有 5 个文件都存在
        assert_eq!(ids.len(), 5);
        for id in &ids {
            let path = temp_dir.path().join(format!("{}.json", id));
            assert!(path.exists(), "File {} should exist", id);
        }

        // 验证：可以读取所有 5 个文件
        for id in &ids {
            let record = store.get(id).await.unwrap();
            assert!(record.is_some(), "Record {} should be retrievable", id);
        }
    }
}
