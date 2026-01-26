use crate::KnowledgeStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 知识记录的简要信息
///
/// 用于搜索结果展示，包含记录的核心元数据。
///
/// # 字段
///
/// * `id` - 记录的唯一标识符
/// * `title` - 记录标题
/// * `parent_doc_title` - 父文档的标题
/// * `summary` - 内容摘要（前 200 个字符）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    pub id: String,
    pub title: String,
    pub parent_doc_title: String,
    pub summary: String,
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
    /// 返回匹配的记录列表（包含简要信息）。
    pub async fn scout(&self, query: &str) -> Result<Vec<Brief>> {
        let records = self.store.search(query).await?;
        Ok(records
            .into_iter()
            .map(|r| Brief {
                id: r.id,
                title: r.title,
                parent_doc_title: r.parent_doc_title,
                summary: r.summary,
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
