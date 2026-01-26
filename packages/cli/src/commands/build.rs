use anyhow::Result;
use contextfy_core::{parse_markdown, KnowledgeStore};
use std::fs;
use std::path::Path;

pub async fn build() -> Result<()> {
    let store = KnowledgeStore::new(".contextfy/data").await?;

    let examples_dir = Path::new("docs/examples");
    if !examples_dir.exists() {
        anyhow::bail!("docs/examples directory not found. Run 'contextfy init' first.");
    }

    let mut documents_count = 0;
    let mut sections_count = 0;

    for entry in fs::read_dir(examples_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_path = path.to_string_lossy();
            println!("Processing: {}", file_path);

            match parse_markdown(&file_path) {
                Ok(doc) => {
                    let ids = store.add(&doc).await?;
                    documents_count += 1;
                    sections_count += doc.sections.len();
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

    println!("\n✓ Build complete!");
    println!("Found {} documents, {} sections", documents_count, sections_count);
    Ok(())
}
