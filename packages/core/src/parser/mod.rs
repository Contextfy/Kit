use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDoc {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

pub fn parse_markdown(file_path: &str) -> Result<ParsedDoc> {
    if !Path::new(file_path).exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let content = fs::read_to_string(file_path)?;
    let parser = Parser::new(&content);

    let mut title = String::new();
    let mut in_h1 = false;

    for event in parser {
        match event {
            Event::Start(pulldown_cmark::Tag::Heading(HeadingLevel::H1, ..)) => {
                in_h1 = true;
            }
            Event::End(pulldown_cmark::Tag::Heading(HeadingLevel::H1, ..)) => {
                in_h1 = false;
            }
            Event::Text(text) if in_h1 && title.is_empty() => {
                title = text.to_string();
            }
            _ => {}
        }
    }

    if title.is_empty() {
        title = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
    }

    let summary = content.chars().take(200).collect::<String>();
    let content_cleaned = content.trim().to_string();

    Ok(ParsedDoc {
        path: file_path.to_string(),
        title,
        summary,
        content: content_cleaned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_h1() {
        let result = parse_markdown("test_data/sample_with_h1.md");
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.title, "Test Document");
    }

    #[test]
    fn test_parse_without_h1() {
        let result = parse_markdown("test_data/sample_without_h1.md");
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.title, "sample_without_h1");
    }
}
