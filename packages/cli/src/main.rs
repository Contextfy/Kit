use clap::{Parser, Subcommand};
use contextfy_core::{parse_markdown, KnowledgeStore, Retriever};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "contextfy")]
#[command(about = "Contextfy Kit - AI Context Orchestration Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(short, long)]
        template: Option<String>,
    },
    Build,
    Scout {
        query: String,
    },
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { template } => {
            println!("Initializing Contextfy project...");
            if let Some(t) = template {
                println!("Using template: {}", t);
            }

            let manifest = r#"{
  "name": "contextfy-project",
  "version": "0.1.0",
  "description": "A Contextfy knowledge base project"
}"#;

            fs::write("contextfy.json", manifest)?;
            fs::create_dir_all("docs/examples")?;

            let example1 = r#"# Example Document 1

This is a sample markdown document for testing the Contextfy system.

## Features

- Feature 1
- Feature 2

This document demonstrates basic markdown parsing and storage.
"#;

            let example2 = r#"# Example Document 2

Another sample document with different content.

## Usage

You can use this document to test search functionality.

### Details

This contains more specific information that should be searchable.
"#;

            let example3 = r#"# API Reference

This document contains API documentation for testing.

## Methods

### getItems()

Returns a list of items from the system.

### createItem(name)

Creates a new item with the specified name.
"#;

            fs::write("docs/examples/doc1.md", example1)?;
            fs::write("docs/examples/doc2.md", example2)?;
            fs::write("docs/examples/doc3.md", example3)?;

            println!("✓ Created contextfy.json");
            println!("✓ Created docs/examples/ directory with sample documents");
            println!("✓ Project initialized successfully!");
        }
        Commands::Build => {
            println!("Building Contextfy knowledge base...");

            let store = KnowledgeStore::new(".contextfy/data")?;

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
                            // 显示存储结果：如果有切片，显示切片数量；否则显示文档 ID
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
        }
        Commands::Scout { query } => {
            println!("Scouting for: {}", query);

            let store = KnowledgeStore::new(".contextfy/data")?;
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
                    eprintln!("Error: {}", e);
                }
            }
        }
        Commands::Serve => {
            println!("Starting server on http://127.0.0.1:3000...");
            println!("Note: Use 'cargo run --bin contextfy-server' for the full server.");
            println!("The server needs to be built with: cargo build --bin contextfy-server");
        }
    }

    Ok(())
}
