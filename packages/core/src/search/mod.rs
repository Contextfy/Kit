//! Tantivy 全文搜索模块
//!
//! 此模块提供基于 Tantivy 的全文搜索索引功能，支持 BM25 关键词搜索和中文分词。

use crate::storage::KnowledgeRecord;
use anyhow::{Context, Result};
use std::path::Path;
use tantivy::{
    collector::TopDocs, query::QueryParser, schema::*, Index, IndexReader, IndexWriter,
    ReloadPolicy,
};
use tantivy_jieba::JiebaTokenizer;

/// 文档字段名称常量
pub const FIELD_ID: &str = "id";
pub const FIELD_TITLE: &str = "title";
pub const FIELD_SUMMARY: &str = "summary";
pub const FIELD_CONTENT: &str = "content";
pub const FIELD_KEYWORDS: &str = "keywords";

/// 创建 Tantivy Schema
///
/// 定义包含五个字段的 Schema：id (STRING)、title、summary、content、keywords (TEXT)。
/// id 字段为 STRING 类型，精确匹配索引并存储（不分词），用于直接定位原文件。
/// 其他字段为 TEXT 类型，支持分词（TOKENIZED）和存储（STORED），
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

    // 添加 id 字段（STRING 类型，精确匹配，不分词）
    schema_builder.add_text_field(FIELD_ID, STRING | STORED);

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

/// 搜索结果结构体
///
/// 包含从 BM25 搜索返回的文档信息及相关性分数。
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 记录的唯一标识符
    pub id: String,
    /// 记录标题
    pub title: String,
    /// 记录摘要
    pub summary: String,
    /// BM25 相关性分数
    pub score: f32,
}

/// 索引写入器
///
/// 负责将文档添加到 Tantivy 索引并提交更改。
pub struct Indexer {
    #[allow(dead_code)]
    index: Index,
    writer: IndexWriter,
    schema: Schema,
}

impl Indexer {
    /// 创建新的索引写入器。
    ///
    /// # 参数
    ///
    /// * `index` - Tantivy 索引实例
    ///
    /// # 返回
    ///
    /// 返回 `Result<Indexer>`，成功时返回创建的写入器，失败时返回错误。
    pub fn new(index: Index) -> Result<Self> {
        let schema = index.schema();
        // 使用 50MB 缓冲区创建索引写入器
        let writer = index
            .writer(50_000_000)
            .context("Failed to create index writer")?;

        Ok(Self {
            index,
            writer,
            schema,
        })
    }

    /// 添加文档到索引。
    ///
    /// 将 `KnowledgeRecord` 转换为 Tantivy 文档并添加到索引缓冲区。
    /// 注意：必须调用 `commit()` 才能使文档可搜索。
    ///
    /// # 参数
    ///
    /// * `record` - 要添加的知识记录
    pub fn add_doc(&mut self, record: &KnowledgeRecord) -> Result<()> {
        let id_field = self
            .schema
            .get_field(FIELD_ID)
            .context("Missing id field")?;
        let title_field = self
            .schema
            .get_field(FIELD_TITLE)
            .context("Missing title field")?;
        let summary_field = self
            .schema
            .get_field(FIELD_SUMMARY)
            .context("Missing summary field")?;
        let content_field = self
            .schema
            .get_field(FIELD_CONTENT)
            .context("Missing content field")?;
        let keywords_field = self
            .schema
            .get_field(FIELD_KEYWORDS)
            .context("Missing keywords field")?;

        let mut doc = TantivyDocument::new();
        doc.add_text(id_field, &record.id);
        doc.add_text(title_field, &record.title);
        doc.add_text(summary_field, &record.summary);
        doc.add_text(content_field, &record.content);

        // 多值字段插入：每个关键词作为单独的值插入
        // Tantivy 原生支持同一字段插入多个值，比拼接字符串更高效
        for keyword in &record.keywords {
            doc.add_text(keywords_field, keyword);
        }

        self.writer
            .add_document(doc)
            .context("Failed to add document to index")?;

        Ok(())
    }

    /// 提交索引更改。
    ///
    /// 将所有缓冲的写入操作持久化到磁盘，使新添加的文档可搜索。
    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit().context("Failed to commit index")?;
        Ok(())
    }

    /// 删除指定 ID 的文档。
    ///
    /// # 参数
    ///
    /// * `id` - 要删除的文档 ID（当前占位实现）
    #[allow(dead_code)]
    pub fn delete(&mut self, id: &str) -> Result<()> {
        // TODO: 实现基于 ID 的删除 (delete-term)
        // ID 字段已添加到 Schema (STRING | STORED)，可以使用 `term` API 删除
        anyhow::bail!("Indexer::delete is not implemented yet for id={}", id);
    }
}

/// 搜索器
///
/// 负责执行 BM25 全文搜索查询。
pub struct Searcher {
    #[allow(dead_code)]
    index: Index,
    reader: IndexReader,
    query_parser: QueryParser,
    schema: Schema,
}

impl Searcher {
    /// 创建新的搜索器。
    ///
    /// # 参数
    ///
    /// * `index` - Tantivy 索引实例
    ///
    /// # 返回
    ///
    /// 返回 `Result<Searcher>`，成功时返回创建的搜索器，失败时返回错误。
    pub fn new(index: Index) -> Result<Self> {
        let schema = index.schema();
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        // 创建查询解析器：在 title、summary、content、keywords 四个字段中搜索
        let title_field = schema
            .get_field(FIELD_TITLE)
            .context("Missing title field")?;
        let summary_field = schema
            .get_field(FIELD_SUMMARY)
            .context("Missing summary field")?;
        let content_field = schema
            .get_field(FIELD_CONTENT)
            .context("Missing content field")?;
        let keywords_field = schema
            .get_field(FIELD_KEYWORDS)
            .context("Missing keywords field")?;

        let mut query_parser = QueryParser::for_index(
            &index,
            vec![title_field, summary_field, content_field, keywords_field],
        );

        // 为 keywords 字段设置高权重（5.0），确保精确的 API 名称匹配排在最前面
        // 这样当用户搜索 "createItem" 时，在代码块中包含该名称的文档会显著高于仅在正文中提及的文档
        query_parser.set_field_boost(keywords_field, 5.0);

        // CRITICAL: 使用索引中注册的 jieba 分词器
        // QueryParser 会自动使用 Index 中注册的 jieba 分词器进行中文分词

        Ok(Self {
            index,
            reader,
            query_parser,
            schema,
        })
    }

    /// 执行搜索查询。
    ///
    /// # 参数
    ///
    /// * `query` - 查询字符串
    /// * `limit` - 返回的最大结果数
    ///
    /// # 返回
    ///
    /// 返回 `Result<Vec<SearchResult>>`，按 BM25 分数降序排列的结果列表。
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query = query.trim();

        // 前置拦截：空查询直接返回空结果
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // 强制刷新索引 reader，确保读取最新的 commit 数据
        self.reader
            .reload()
            .context("Failed to reload index reader before search")?;

        // 每次搜索获取最新的 searcher 快照，避免陈旧读取
        let searcher = self.reader.searcher();

        // 使用 QueryParser 解析查询（自动应用 jieba 分词器）
        let parsed_query = self
            .query_parser
            .parse_query(query)
            .with_context(|| format!("Failed to parse query: {}", query))?;

        // 执行搜索，使用 TopDocs 收集器获取 BM25 分数
        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit))
            .context("Failed to execute search")?;

        let id_field = self
            .schema
            .get_field(FIELD_ID)
            .context("Missing id field")?;
        let title_field = self
            .schema
            .get_field(FIELD_TITLE)
            .context("Missing title field")?;
        let summary_field = self
            .schema
            .get_field(FIELD_SUMMARY)
            .context("Missing summary field")?;

        // 转换搜索结果
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher
                .doc(doc_address)
                .context("Failed to retrieve document")?;

            // 提取字段值
            let id = extracted_text_value(&retrieved_doc, id_field);
            let title = extracted_text_value(&retrieved_doc, title_field);
            let summary = extracted_text_value(&retrieved_doc, summary_field);

            results.push(SearchResult {
                id,
                title,
                summary,
                score,
            });
        }

        Ok(results)
    }
}

/// 从 Tantivy 文档中提取文本字段的值。
fn extracted_text_value(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|value| value.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
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

    #[test]
    fn test_indexer_add_doc() {
        let index = create_index(None).expect("Failed to create index");
        let mut indexer = Indexer::new(index).expect("Failed to create indexer");

        // 创建测试记录
        let record = KnowledgeRecord {
            id: "test-1".to_string(),
            title: "Test Document".to_string(),
            parent_doc_title: "Parent Doc".to_string(),
            summary: "This is a test summary".to_string(),
            content: "This is the full content of the test document".to_string(),
            source_path: "/test/path.md".to_string(),
            keywords: vec![],
        };

        // 添加文档
        assert!(
            indexer.add_doc(&record).is_ok(),
            "Should add document successfully"
        );

        // 提交索引
        assert!(indexer.commit().is_ok(), "Should commit successfully");
    }

    #[test]
    fn test_searcher_basic_query() {
        let index = create_index(None).expect("Failed to create index");
        let mut indexer = Indexer::new(index.clone()).expect("Failed to create indexer");

        // 添加测试文档
        let record1 = KnowledgeRecord {
            id: "doc-1".to_string(),
            title: "Rust Programming".to_string(),
            parent_doc_title: "Rust Book".to_string(),
            summary: "A comprehensive guide to Rust".to_string(),
            content: "Rust is a systems programming language".to_string(),
            source_path: "/rust.md".to_string(),
            keywords: vec![],
        };

        let record2 = KnowledgeRecord {
            id: "doc-2".to_string(),
            title: "Python Guide".to_string(),
            parent_doc_title: "Python Book".to_string(),
            summary: "Learn Python programming".to_string(),
            content: "Python is easy to learn".to_string(),
            source_path: "/python.md".to_string(),
            keywords: vec![],
        };

        indexer.add_doc(&record1).unwrap();
        indexer.add_doc(&record2).unwrap();
        indexer.commit().unwrap();

        // 创建搜索器
        let searcher = Searcher::new(index).expect("Failed to create searcher");

        // 搜索 "Rust"
        let results = searcher.search("Rust", 10).expect("Search should succeed");

        assert!(!results.is_empty(), "Should find results for 'Rust'");
        assert_eq!(
            results[0].title, "Rust Programming",
            "First result should be Rust doc"
        );
    }

    #[test]
    fn test_searcher_bm25_scoring() {
        let index = create_index(None).expect("Failed to create index");
        let mut indexer = Indexer::new(index.clone()).expect("Failed to create indexer");

        // 添加测试文档 - title 匹配应该获得更高分数
        let record1 = KnowledgeRecord {
            id: "doc-1".to_string(),
            title: "Rust Programming Language".to_string(),
            parent_doc_title: "Tech Docs".to_string(),
            summary: "Comprehensive guide to Rust programming".to_string(),
            content: "Learn Rust systems programming".to_string(),
            source_path: "/rust.md".to_string(),
            keywords: vec![],
        };

        let record2 = KnowledgeRecord {
            id: "doc-2".to_string(),
            title: "Python Tutorial".to_string(),
            parent_doc_title: "Tech Docs".to_string(),
            summary: "Brief mention of Rust in Python context".to_string(),
            content: "Python programming language tutorial".to_string(),
            source_path: "/python.md".to_string(),
            keywords: vec![],
        };

        indexer.add_doc(&record1).unwrap();
        indexer.add_doc(&record2).unwrap();
        indexer.commit().unwrap();

        let searcher = Searcher::new(index).expect("Failed to create searcher");
        let results = searcher.search("Rust", 10).expect("Search should succeed");

        // doc-1 应该排在第一位，因为它的 title 和 content 都包含 "Rust"
        assert_eq!(results[0].title, "Rust Programming Language");
        assert!(results[0].score > 0.0, "Score should be positive");

        // 第二个结果应该也有分数（因为 summary 包含 "Rust"）
        if results.len() > 1 {
            assert!(
                results[1].score > 0.0,
                "Second score should also be positive"
            );
        }
    }

    #[test]
    fn test_search_results_ordering() {
        let index = create_index(None).expect("Failed to create index");
        let mut indexer = Indexer::new(index.clone()).expect("Failed to create indexer");

        // 添加多个文档
        for i in 0..5 {
            let record = KnowledgeRecord {
                id: format!("doc-{}", i),
                title: format!("Document {}", i),
                parent_doc_title: "Parent".to_string(),
                summary: format!("Summary {}", i),
                content: "keyword content".repeat(i + 1).to_string(),
                source_path: format!("/doc{}.md", i),
                keywords: vec![],
            };
            indexer.add_doc(&record).unwrap();
        }
        indexer.commit().unwrap();

        let searcher = Searcher::new(index).expect("Failed to create searcher");
        let results = searcher
            .search("keyword", 10)
            .expect("Search should succeed");

        // 验证结果按分数降序排列
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be ordered by score descending"
            );
        }
    }

    #[test]
    fn test_searcher_empty_query() {
        let index = create_index(None).expect("Failed to create index");
        let searcher = Searcher::new(index).expect("Failed to create searcher");

        // 空查询应该返回空结果
        let results = searcher.search("", 10).expect("Search should succeed");
        assert!(results.is_empty(), "Empty query should return no results");

        // 仅空白的查询也应该返回空结果
        let results = searcher.search("   ", 10).expect("Search should succeed");
        assert!(
            results.is_empty(),
            "Whitespace query should return no results"
        );
    }

    #[test]
    #[ignore = "perf benchmark"]
    fn benchmark_search_latency() {
        use std::time::Instant;

        let index = create_index(None).expect("Failed to create index");
        let mut indexer = Indexer::new(index.clone()).expect("Failed to create indexer");

        // 插入 1000 个模拟文档
        for i in 0..1000 {
            let record = KnowledgeRecord {
                id: format!("doc-{:04}", i),
                title: format!("Document Title {}", i),
                parent_doc_title: "Parent Collection".to_string(),
                summary: format!("This is document number {} with some content", i),
                content: format!(
                    "Full content for document {}. Contains various keywords and text.",
                    i
                ),
                source_path: format!("/docs/doc{:04}.md", i),
                keywords: vec![],
            };
            indexer.add_doc(&record).unwrap();
        }
        indexer.commit().unwrap();

        let searcher = Searcher::new(index).expect("Failed to create searcher");

        // 测量搜索延迟
        let start = Instant::now();
        let results = searcher
            .search("document content", 100)
            .expect("Search should succeed");
        let elapsed = start.elapsed();

        // 验证：查询延迟应 < 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Search latency should be < 100ms, got {}ms",
            elapsed.as_millis()
        );

        // 验证：应该返回结果
        assert!(!results.is_empty(), "Should return results");

        println!(
            "Benchmark: 1000 documents, search took {}ms, returned {} results",
            elapsed.as_millis(),
            results.len()
        );
    }
}
