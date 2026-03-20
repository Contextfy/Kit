use anyhow::Result;
use colored::Colorize;
use contextfy_core::SearchEngine;

/// 搜索知识库
///
/// 使用混合检索策略（BM25 + Vector）快速搜索知识库。
/// 返回匹配结果的摘要和评分。
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
    let engine = SearchEngine::new(
        Some(std::path::Path::new(".contextfy/data/bm25_index")),
        ".contextfy/data/lancedb",
        "knowledge",
    )
    .await?;

    match engine.search(&query, 10).await {
        Ok(hits) => {
            if hits.is_empty() {
                println!("No results found.");
            } else {
                println!("\nFound {} result(s):", hits.len());
                for (i, hit) in hits.iter().enumerate() {
                    // 根据分数使用不同颜色高亮
                    let score_display = format!("{:.2}", hit.score.value());
                    let colored_score = if hit.score.value() >= 0.8 {
                        score_display.green().bold()
                    } else if hit.score.value() >= 0.5 {
                        score_display.yellow().bold()
                    } else {
                        score_display.white().dimmed()
                    };

                    println!(
                        "\n[{}] {} | ID: {}",
                        i + 1,
                        format!("Score: {}", colored_score).cyan(),
                        hit.id
                    );

                    // Try to get document details for title and summary
                    match engine.get_document(&hit.id).await {
                        Ok(Some(doc)) => {
                            println!("    Title: {}", doc.title);
                            println!("    Summary: {}", doc.summary);
                        }
                        Ok(None) => {
                            println!("    (Document details not available)");
                        }
                        Err(_) => {
                            println!("    (Failed to load document details)");
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}
