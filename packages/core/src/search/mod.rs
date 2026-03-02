//! Tantivy 全文搜索模块
//!
//! 此模块提供基于 Tantivy 的全文搜索索引功能，支持 BM25 关键词搜索和中文分词。

use anyhow::Result;
use std::path::Path;
use tantivy::{schema::*, Index};
use tantivy_jieba::JiebaTokenizer;

/// 文档字段名称常量
pub const FIELD_TITLE: &str = "title";
pub const FIELD_SUMMARY: &str = "summary";
pub const FIELD_CONTENT: &str = "content";
pub const FIELD_KEYWORDS: &str = "keywords";

/// 创建 Tantivy Schema
///
/// 定义包含四个 TEXT 字段的 Schema：title、summary、content、keywords。
/// 所有字段均配置为 TEXT 类型，支持分词（TOKENIZED）和存储（STORED），
/// 并使用 Jieba 分词器进行中文分词。
///
/// # 示例
///
/// ```rust
/// use contextfy_core::search::create_schema;
///
/// let schema = create_schema();
/// assert!(schema.get_field("title").is_ok());
/// ```
pub fn create_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // 创建 Jieba 分词器的文本索引配置
    let text_indexing = TextFieldIndexing::default().set_tokenizer("jieba");

    // 创建文本字段选项：分词 + 存储 + 自定义分词器
    let text_options = TextOptions::default()
        .set_indexing_options(text_indexing)
        .set_stored();

    // 添加四个 TEXT 字段，支持分词和存储，使用 Jieba 分词器
    schema_builder.add_text_field(FIELD_TITLE, text_options.clone());
    schema_builder.add_text_field(FIELD_SUMMARY, text_options.clone());
    schema_builder.add_text_field(FIELD_CONTENT, text_options.clone());
    schema_builder.add_text_field(FIELD_KEYWORDS, text_options);

    schema_builder.build()
}

/// 创建 Tantivy 索引
///
/// 支持两种模式：
/// - 内存索引：不传目录参数，索引仅存在于内存中
/// - 文件系统索引：传入目录路径，索引持久化到磁盘
///
/// 对于文件系统索引，如果目录中已存在索引，将打开现有索引；否则创建新索引。
///
/// # 参数
///
/// * `directory` - 可选的目录路径。如果为 None，则创建内存索引。
///
/// # 返回
///
/// 返回 `Result<Index>`，成功时返回创建的索引，失败时返回错误。
///
/// # 示例
///
/// ```rust,no_run
/// use contextfy_core::search::create_index;
/// use std::path::Path;
///
/// // 创建内存索引
/// let index = create_index(None).unwrap();
///
/// // 创建文件系统索引
/// let index = create_index(Some(Path::new("/tmp/index"))).unwrap();
/// ```
pub fn create_index(directory: Option<&Path>) -> Result<Index> {
    let schema = create_schema();

    let index = match directory {
        Some(path) => {
            // 创建或打开文件系统索引
            // 先尝试打开现有索引，如果不存在则创建
            Index::open_in_dir(path).or_else(|open_err| {
                Index::create_in_dir(path, schema.clone()).map_err(|create_err| {
                    anyhow::anyhow!(
                        "Failed to open or create index in directory {}: open error: {}, create error: {}",
                        path.display(),
                        open_err,
                        create_err
                    )
                })
            })?
        }
        None => Index::create_in_ram(schema),
    };

    // 注册 Jieba 分词器（统一处理，避免代码重复）
    index.tokenizers().register("jieba", JiebaTokenizer {});

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_schema_fields() {
        let schema = create_schema();

        // 验证所有四个字段都存在
        let title_field = schema.get_field(FIELD_TITLE);
        assert!(title_field.is_ok(), "title field should exist");

        let summary_field = schema.get_field(FIELD_SUMMARY);
        assert!(summary_field.is_ok(), "summary field should exist");

        let content_field = schema.get_field(FIELD_CONTENT);
        assert!(content_field.is_ok(), "content field should exist");

        let keywords_field = schema.get_field(FIELD_KEYWORDS);
        assert!(keywords_field.is_ok(), "keywords field should exist");

        // 验证字段配置
        if let Ok(field) = title_field {
            let entry = schema.get_field_entry(field);
            assert!(
                matches!(entry.field_type(), FieldType::Str(_)),
                "title field should be Str type"
            );
        }
    }

    #[test]
    fn test_create_in_memory_index() {
        let index = create_index(None);
        assert!(index.is_ok(), "Should create in-memory index successfully");

        let index = index.unwrap();
        // 验证索引可以正常使用
        let schema = index.schema();
        assert!(schema.get_field(FIELD_TITLE).is_ok());
    }

    #[test]
    fn test_create_filesystem_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        let index = create_index(Some(index_path));
        assert!(index.is_ok(), "Should create filesystem index successfully");

        let index = index.unwrap();
        // 验证索引可以正常使用
        let schema = index.schema();
        assert!(schema.get_field(FIELD_TITLE).is_ok());

        // 验证索引文件已创建
        assert!(index_path.exists(), "Index directory should exist");
    }

    #[test]
    fn test_reopen_existing_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        // 创建索引
        let index1 = create_index(Some(index_path));
        assert!(index1.is_ok());

        // 重新打开已存在的索引
        let _schema = create_schema();
        let index2 = Index::open_in_dir(index_path);
        assert!(index2.is_ok(), "Should be able to reopen existing index");

        let index2 = index2.unwrap();
        // 验证 Schema 一致
        assert!(index2.schema().get_field(FIELD_TITLE).is_ok());
    }

    #[test]
    fn test_jieba_tokenizer_registered() {
        // 测试内存索引的 Jieba 分词器注册
        let index = create_index(None).expect("Failed to create in-memory index");
        let tokenizer = index.tokenizers().get("jieba");
        assert!(tokenizer.is_some(), "Jieba tokenizer should be registered");

        // 测试文件系统索引的 Jieba 分词器注册
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let index_path = temp_dir.path();

        let index = create_index(Some(index_path)).expect("Failed to create filesystem index");
        let tokenizer = index.tokenizers().get("jieba");
        assert!(
            tokenizer.is_some(),
            "Jieba tokenizer should be registered in filesystem index"
        );
    }
}
