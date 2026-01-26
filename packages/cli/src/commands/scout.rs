use anyhow::Result;
use contextfy_core::{KnowledgeStore, Retriever};

/// 搜索知识库
///
/// 使用两阶段检索策略快速搜索知识库。首先返回匹配结果的摘要和评分，
/// 用户可以根据摘要决定是否加载完整内容。
///
/// # Arguments
///
/// * `query` - 搜索查询字符串
///
/// # Errors
///
/// 如果知识库打开失败或搜索失败，返回错误
///
/// # Examples
///
/// ```no_run
/// # use contextfy_cli::commands::scout;
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// scout("如何创建自定义方块?".to_string()).await?;
/// # Ok(())
/// # }
/// ```
pub async fn scout(query: String) -> Result<()> {
    let store = KnowledgeStore::new(".contextfy/data").await?;
    let retriever = Retriever::new(&store);

    match retriever.scout(&query).await {
        Ok(briefs) => {
            if briefs.is_empty() {
                println!("No results found.");
            } else {
                println!("\nFound {} result(s):", briefs.len());
                for (i, result) in briefs.iter().enumerate() {
                    let display_title = if result.parent_doc_title == result.title {
                        result.title.clone()
                    } else {
                        format!("[{}] {}", result.parent_doc_title, result.title)
                    };
                    println!("\n[{}] {}", i + 1, display_title);
                    println!("    ID: {}", result.id);
                    println!("    Summary: {}", result.summary);
                }
            }
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    Ok(())
}
