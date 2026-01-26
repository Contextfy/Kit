use anyhow::Result;
use contextfy_core::{parse_markdown, KnowledgeStore};
use std::fs;
use std::path::Path;

pub async fn build() -> Result<()> {
    println!("Building Contextfy knowledge base...");

    let store = KnowledgeStore::new(".contextfy/data").await?;

    let examples_dir = Path::new("docs/examples");
    if !examples_dir.exists() {
        anyhow::bail!("docs/examples directory not found. Run 'contextfy init' first.");
    }

    for entry in fs::read_dir(examples_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_path = path.to_str().unwrap();
            println!("Processing: {}", file_path);

            match parse_markdown(file_path) {
                Ok(doc) => {
                    let ids = store.add(&doc).await?;
                    if doc.sections.is_empty() {
                        println!("  → Stored: {} (ID: {})", doc.title, ids[0]);
                    } else {
                        println!("  → Stored: {} ({} slices)", doc.title, ids.len());
                        for (i, id) in ids.iter().enumerate() {
                            println!("      [{}] Slice ID: {}", i + 1, id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to parse {}: {}", file_path, e);
                }
            }
        }
    }

    println!("✓ Build complete!");
    Ok(())
}
