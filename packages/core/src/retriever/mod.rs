use crate::KnowledgeStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 默认搜索结果数量限制
pub(crate) const DEFAULT_SEARCH_LIMIT: usize = 100;

/// 知识记录的简要信息
///
/// 用于搜索结果展示，包含记录的核心元数据。
///
/// # 字段
///
/// * `id` - 记录的唯一标识符
/// * `title` - 记录标题
/// * `parent_doc_title` - 父文档的标题
/// * `summary` - 内容摘要（智能提取首段或代码块，最多 1000 字符）
/// * `score` - 相关性分数（scout 纯文本检索返回 BM25 分数，hybrid_scout 混合检索返回融合后的最终分数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    pub id: String,
    pub title: String,
    pub parent_doc_title: String,
    pub summary: String,
    pub score: f32,
}

/// 知识记录的详细信息
///
/// 用于查看完整的记录内容。
///
/// # 字段
///
/// * `id` - 记录的唯一标识符
/// * `title` - 记录标题
/// * `content` - 完整内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Details {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// 知识检索器
///
/// 提供高层次的知识检索 API，用于搜索和查看知识记录。
///
/// # 示例
///
/// ```ignore
/// let store = KnowledgeStore::new(".contextfy/data")?;
/// let retriever = Retriever::new(&store);
///
/// // 搜索记录
/// let results = retriever.scout("Rust").await?;
///
/// // 查看详情
/// if let Some(details) = retriever.inspect(&id).await? {
///     println!("{}", details.content);
/// }
/// ```
pub struct Retriever<'a> {
    store: &'a KnowledgeStore,
}

impl<'a> Retriever<'a> {
    /// 创建新的检索器
    ///
    /// # 参数
    ///
    /// * `store` - 知识存储引用
    pub fn new(store: &'a KnowledgeStore) -> Self {
        Retriever { store }
    }

    /// 搜索知识记录（返回简要信息）
    ///
    /// 根据查询字符串搜索标题和摘要中包含关键词的记录。
    ///
    /// # 参数
    ///
    /// * `query` - 搜索关键词（不区分大小写）
    ///
    /// # 返回值
    ///
    /// 返回匹配的记录列表（包含简要信息和 BM25 相关性分数）。
    pub async fn scout(&self, query: &str) -> Result<Vec<Brief>> {
        let records = self.store.search(query, DEFAULT_SEARCH_LIMIT).await?;
        Ok(records
            .into_iter()
            .map(|(r, score)| Brief {
                id: r.id,
                title: r.title,
                parent_doc_title: r.parent_doc_title,
                summary: r.summary,
                score,
            })
            .collect())
    }

    /// 混合检索知识记录（返回简要信息）
    ///
    /// 结合 BM25 关键词匹配和向量语义相似度，使用加权融合提供更精准的排序结果。
    ///
    /// # 参数
    ///
    /// * `query` - 搜索关键词（不区分大小写）
    ///
    /// # 返回值
    ///
    /// 返回匹配的记录列表（包含简要信息和融合后的相关性分数）。
    pub async fn hybrid_scout(&self, query: &str) -> Result<Vec<Brief>> {
        let results = self
            .store
            .hybrid_search(query, DEFAULT_SEARCH_LIMIT)
            .await?;
        Ok(results
            .into_iter()
            .map(|r| Brief {
                id: r.record.id,
                title: r.record.title,
                parent_doc_title: r.record.parent_doc_title,
                summary: r.record.summary,
                score: r.final_score,
            })
            .collect())
    }

    /// 获取知识记录的详细信息
    ///
    /// 根据记录 ID 获取完整的记录内容。
    ///
    /// # 参数
    ///
    /// * `id` - 记录的唯一标识符
    ///
    /// # 返回值
    ///
    /// 如果找到记录，返回 `Some(Details)`；否则返回 `None`。
    pub async fn inspect(&self, id: &str) -> Result<Option<Details>> {
        let record = self.store.get(id).await?;

        match record {
            Some(r) => Ok(Some(Details {
                id: r.id,
                title: r.title,
                content: r.content,
            })),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParsedDoc;

    /// 测试 scout 方法（BM25 搜索）
    #[tokio::test]
    async fn test_retriever_scout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加测试文档
        let doc = ParsedDoc {
            path: "/test/rust.md".to_string(),
            title: "Rust Programming".to_string(),
            summary: "A comprehensive guide to Rust".to_string(),
            content: "Rust is a systems programming language".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        let retriever = Retriever::new(&store);

        let results = retriever.scout("Rust").await.unwrap();

        assert!(!results.is_empty(), "Should find results for 'Rust'");
        assert_eq!(results[0].title, "Rust Programming");
        assert!(results[0].score > 0.0, "BM25 score should be positive");
    }

    /// 测试 hybrid_scout 方法（混合检索）
    #[tokio::test]
    async fn test_retriever_hybrid_scout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加测试文档
        let doc = ParsedDoc {
            path: "/test/rust.md".to_string(),
            title: "Rust Programming".to_string(),
            summary: "A comprehensive guide to Rust".to_string(),
            content: "Rust is a systems programming language".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        let retriever = Retriever::new(&store);

        let results = retriever.hybrid_scout("Rust").await.unwrap();

        assert!(!results.is_empty(), "Should find results for 'Rust'");
        assert_eq!(results[0].title, "Rust Programming");
        assert!(
            results[0].score >= 0.0 && results[0].score <= 1.0,
            "Final score should be in [0, 1] range"
        );
    }

    /// 测试 inspect 方法（查看详情）
    #[tokio::test]
    async fn test_retriever_inspect() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加测试文档
        let doc = ParsedDoc {
            path: "/test/rust.md".to_string(),
            title: "Rust Programming".to_string(),
            summary: "A comprehensive guide to Rust".to_string(),
            content: "Rust is a systems programming language".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        let retriever = Retriever::new(&store);

        // 先搜索获取一个 ID
        let results = retriever.scout("Rust").await.unwrap();
        assert!(!results.is_empty());

        let id = &results[0].id;

        // 查看详情
        let details = retriever.inspect(id).await.unwrap();
        assert!(details.is_some(), "Should find details for valid ID");

        let details = details.unwrap();
        assert_eq!(details.id, *id);
        assert_eq!(details.title, "Rust Programming");
        assert!(!details.content.is_empty());
    }

    /// 测试 inspect 方法（不存在的 ID）
    #[tokio::test]
    async fn test_retriever_inspect_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        let retriever = Retriever::new(&store);

        let details = retriever.inspect("non-existent-id").await.unwrap();
        assert!(details.is_none(), "Should return None for non-existent ID");
    }

    /// 测试边界条件（空查询）
    #[tokio::test]
    async fn test_retriever_empty_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加测试文档
        let doc = ParsedDoc {
            path: "/test/rust.md".to_string(),
            title: "Rust Programming".to_string(),
            summary: "A comprehensive guide to Rust".to_string(),
            content: "Rust is a systems programming language".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        let retriever = Retriever::new(&store);

        // scout 应该返回空结果
        let results = retriever.scout("").await.unwrap();
        assert!(results.is_empty(), "Empty query should return no results");

        // hybrid_scout 应该返回空结果
        let results = retriever.hybrid_scout("").await.unwrap();
        assert!(results.is_empty(), "Empty query should return no results");
    }
}
