use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
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

/// 表示一个按 H2 标题切片后的文档片段
///
/// # 字段
///
/// * `section_title` - H2 标题文本
/// * `content` - 该 H2 下的完整内容（从 H2 开始到下一个 H2 之前）
/// * `parent_doc_title` - 父文档的 H1 标题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicedDoc {
    pub section_title: String,
    pub content: String,
    pub parent_doc_title: String,
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

/// 根据 H2 标题将 Markdown 内容切片为多个片段
///
/// # 参数
///
/// * `content` - 要切片的 Markdown 内容
/// * `parent_title` - 父文档的标题（通常是 H1）
///
/// # 返回值
///
/// 返回一个 `SlicedDoc` 向量，每个元素代表一个 H2 标题及其内容。
/// 如果文档中没有 H2 标题，则返回空向量。
///
/// # 行为
///
/// - 忽略第一个 H2 标题之前的所有内容
/// - H3/H4 等子标题作为当前 H2 片段的内容的一部分
/// - 使用 AST 解析，代码块中的 `##` 不会被误认为 H2 标题
/// - 零拷贝实现，使用 `pulldown-cmark` 的 `into_offset_iter()`
///
/// # 示例
///
/// ```ignore
/// let content = "# Doc\n\n## Section 1\nContent 1\n\n## Section 2\nContent 2";
/// let slices = slice_by_headers(content, "Doc");
/// assert_eq!(slices.len(), 2);
/// assert_eq!(slices[0].section_title, "Section 1");
/// ```
pub fn slice_by_headers(content: &str, parent_title: &str) -> Vec<SlicedDoc> {
    let mut slices = Vec::new();

    let parser = Parser::new(content);
    let mut h2_start_indices: Vec<usize> = Vec::new();
    let mut h2_titles: Vec<String> = Vec::new();
    let mut current_h2_title: Option<String> = None;

    // 第一遍遍历：收集所有 H2 标题的位置和文本
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading(HeadingLevel::H2, ..)) => {
                h2_start_indices.push(range.start);
                current_h2_title = Some(String::new());
            }
            Event::End(Tag::Heading(HeadingLevel::H2, ..)) => {
                if let Some(title) = current_h2_title.take() {
                    h2_titles.push(title);
                }
            }
            Event::Text(text) => {
                // 如果正在解析 H2 标题，追加文本到当前标题
                if let Some(title) = &mut current_h2_title {
                    title.push_str(&text);
                }
            }
            _ => {}
        }
    }

    // 如果没有 H2 标题，返回空向量
    if h2_start_indices.is_empty() {
        return slices;
    }

    // 第二阶段：根据 H2 位置进行切片
    for (i, &start_idx) in h2_start_indices.iter().enumerate() {
        let end_idx = if i + 1 < h2_start_indices.len() {
            // 下一个 H2 的开始位置
            h2_start_indices[i + 1]
        } else {
            // 文档结尾
            content.len()
        };

        let slice_content = content[start_idx..end_idx].to_string();
        let section_title = h2_titles.get(i).cloned().unwrap_or_default();

        slices.push(SlicedDoc {
            section_title,
            content: slice_content,
            parent_doc_title: parent_title.to_string(),
        });
    }

    slices
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

    // Slicing tests
    #[test]
    fn test_slice_standard_three_h2() {
        let content = r#"# Parent Doc

Some preamble text.

## Section One

Content for section one.

## Section Two

Content for section two.

## Section Three

Content for section three.
"#;

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 3);
        assert_eq!(slices[0].section_title, "Section One");
        assert_eq!(slices[1].section_title, "Section Two");
        assert_eq!(slices[2].section_title, "Section Three");
        assert!(slices[0].content.contains("Content for section one."));
        assert!(slices[1].content.contains("Content for section two."));
        assert!(slices[2].content.contains("Content for section three."));
    }

    #[test]
    fn test_slice_no_headers() {
        let content = r#"# Parent Doc

Just some content without any H2 headers.
"#;

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 0);
    }

    #[test]
    fn test_slice_nested_h3() {
        let content = r#"# Parent Doc

## Main Section

Some content.

### Subsection A

Subsection content.

### Subsection B

More subsection content.

End of main section.
"#;

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].section_title, "Main Section");
        // H3 应该包含在切片内容中
        assert!(slices[0].content.contains("### Subsection A"));
        assert!(slices[0].content.contains("Subsection content."));
        assert!(slices[0].content.contains("### Subsection B"));
    }

    #[test]
    fn test_slice_code_block_trap() {
        let content = "# Parent Doc\n\n## Section One\n\nRegular content.\n\n```\nThis is a code block.\nIt contains ## which should NOT be a header.\nEnd of code.\n```\n\nMore content.\n";

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].section_title, "Section One");
        // 代码块应该完整包含在切片中
        assert!(slices[0].content.contains("```"));
        assert!(slices[0]
            .content
            .contains("## which should NOT be a header"));
    }

    #[test]
    fn test_slice_empty_content_between_headers() {
        let content = r#"# Parent Doc

## Section One

## Section Two

Some content.
"#;

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].section_title, "Section One");
        assert_eq!(slices[1].section_title, "Section Two");
        // 第一个切片的内容可能只有标题，或为空
        assert!(slices[0].content.contains("## Section One"));
    }

    #[test]
    fn test_slice_unicode_and_emoji() {
        let content = r#"# 父文档

## 简介 🚀

这是一个包含中文和 Emoji 的测试。

## 功能特性

- 特性一
- 特性二 ✨
"#;

        let slices = slice_by_headers(content, "父文档");
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].section_title, "简介 🚀");
        assert_eq!(slices[1].section_title, "功能特性");
        assert!(slices[0].content.contains("中文和 Emoji"));
        assert!(slices[1].content.contains("✨"));
    }

    #[test]
    fn test_slice_inline_formatting() {
        let content = "# Parent Doc\n\n## Section **One**\n\nContent for section one.\n\n## Section *Two*\n\nContent for section two.\n";

        let slices = slice_by_headers(content, "Parent Doc");
        assert_eq!(slices.len(), 2);
        // 应该包含完整的内联格式
        assert_eq!(slices[0].section_title, "Section One");
        assert_eq!(slices[1].section_title, "Section Two");
        assert!(slices[0].content.contains("Content for section one"));
        assert!(slices[1].content.contains("Content for section two"));
    }
}
