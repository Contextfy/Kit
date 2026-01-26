use anyhow::Result;
use contextfy_core::{KnowledgeStore, Retriever};

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
                    println!("\n[{}] {}", i + 1, result.title);
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
