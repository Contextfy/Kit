use anyhow::Result;
use std::fs;

pub fn init(template: Option<String>) -> Result<()> {
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

    Ok(())
}
