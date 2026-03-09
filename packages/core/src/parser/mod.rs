use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// 智能提取内容摘要
///
/// 提取内容的摘要
///
/// 提取逻辑：
/// 1. 如果以代码块开始（```），包含整个代码块
/// 2. 查找第一个双换行符（\n\n）作为段落分隔
/// 3. 硬截断保护：超过 1000 字符强制截断
/// 4. 回退：无段落分隔时截取前 200 字符
/// 5. 清理首尾空白
///
/// # 性能优化
///
/// 如果调用者确保传入**已 trim 的内容**，可以跳过内部 trim，提升性能。
/// 函数会自动检测是否需要 trim，只在必要时才执行。
///
/// # 参数
///
/// - `content`: 内容字符串（建议已 trim 以获得最佳性能）
///
/// # 返回
///
/// 提取的摘要字符串
pub fn extract_summary(content: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 1000;
    const FALLBACK_CHARS: usize = 200;

    // 性能优化：只在需要时才 trim（避免不必要的遍历）
    // 如果内容首尾有空白字符，才执行 trim；否则直接使用原内容
    let content = if content.starts_with(|c: char| c.is_whitespace())
        || content.ends_with(|c: char| c.is_whitespace())
    {
        content.trim()
    } else {
        content
    };

    if content.is_empty() {
        return String::new();
    }

    // 检查是否以代码块开始
    // 由于已执行（或确认不需要）trim，可以安全地使用 starts_with() 检测
    let in_code_block = content.starts_with("```");

    // 查找合适的截断点
    let end_pos = if in_code_block {
        // 如果在代码块中，找到代码块结束标记
        find_code_block_end(content)
    } else {
        // 否则查找第一个 \n\n
        content.find("\n\n")
    };

    // 提取内容
    let extracted = match end_pos {
        Some(pos) => &content[..pos],
        None => content,
    };

    // 硬截断保护
    let truncated = if extracted.chars().count() > MAX_SUMMARY_CHARS {
        smart_truncate(extracted, MAX_SUMMARY_CHARS)
    } else if extracted.chars().count() > FALLBACK_CHARS && end_pos.is_none() {
        // 回退机制：无段落分隔且超过 200 字符，截取前 200 字符
        smart_truncate(extracted, FALLBACK_CHARS)
    } else {
        extracted.to_string()
    };

    truncated.trim().to_string()
}

/// 从 Markdown 内容中提取代码块关键词
///
/// 严格限制在 ``` 代码块内部提取标识符，避免误匹配正文。
///
/// # 提取逻辑
///
/// 1. 使用 pulldown-cmark 解析器识别代码块（``` 围栏内容）
/// 2. 对每个代码块应用正则模式提取标识符：
///    - 函数名：`fn function_name(`、`def function_name(`、`function function_name(`
///    - 类/类型名：`class ClassName`、`struct StructName`、`interface InterfaceName`
///    - CamelCase/PascalCase 标识符
/// 3. 过滤常见编程语言关键字和短标识符
/// 4. 去重并排序返回
///
/// # 性能优化
///
/// 所有正则表达式使用 `OnceLock` 缓存，避免重复编译。
///
/// # 参数
///
/// - `content`: Markdown 内容字符串
///
/// # 返回
///
/// 去重排序后的关键词列表
///
/// # 示例
///
/// ```ignore
/// let content = r#"
/// ```rust
/// pub fn create_item() -> Result<Item> {
///     let item = Item::new();
///     Ok(item)
/// }
/// ```
/// "#;
/// let keywords = extract_code_block_keywords(content);
/// assert!(keywords.contains(&"create_item".to_string()));
/// assert!(keywords.contains(&"Item".to_string()));
/// ```
pub fn extract_code_block_keywords(content: &str) -> Vec<String> {
    // 零拷贝优化：使用 &str 而非 String，避免循环内的 .to_string() 堆分配
    // 但由于 code_block_content 在循环中被重用，我们需要在每次代码块处理后立即转换为 String
    let mut keywords: HashSet<String> = HashSet::new();

    // 使用缓存的正则表达式
    let function_regex = get_function_regex();
    let class_regex = get_class_regex();
    let camelcase_regex = get_camelcase_regex();
    let snakecase_regex = get_snakecase_regex();

    // 解析 Markdown，提取代码块
    let parser = Parser::new(content);
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_content.clear();
            }
            Event::End(Tag::CodeBlock(_)) => {
                // 提取当前代码块的关键词（临时 HashSet，零拷贝）
                let mut block_keywords: HashSet<&str> = HashSet::new();
                extract_identifiers_from_code(
                    &code_block_content,
                    function_regex,
                    class_regex,
                    camelcase_regex,
                    snakecase_regex,
                    &mut block_keywords,
                );

                // 立即转换为拥有所有权的 String，避免引用失效
                for kw in block_keywords {
                    if !is_language_keyword(kw) && kw.len() >= 3 {
                        keywords.insert(kw.to_string());
                    }
                }

                in_code_block = false;
                code_block_content.clear();
            }
            Event::Text(text) | Event::Code(text) if in_code_block => {
                code_block_content.push_str(&text);
            }
            _ => {}
        }
    }

    // 转换为排序后的 Vec
    let mut result: Vec<String> = keywords.into_iter().collect();
    result.sort();
    result
}

/// 从代码块内容中提取标识符（零拷贝版本）
fn extract_identifiers_from_code<'a>(
    code: &'a str,
    function_regex: &Regex,
    class_regex: &Regex,
    camelcase_regex: &Regex,
    snakecase_regex: &Regex,
    keywords: &mut HashSet<&'a str>,
) {
    // 提取函数名（零拷贝：直接插入 &str）
    for caps in function_regex.captures_iter(code) {
        if let Some(name) = caps.get(2) {
            keywords.insert(name.as_str());
        }
    }

    // 提取类/类型名（零拷贝：直接插入 &str）
    for caps in class_regex.captures_iter(code) {
        if let Some(name) = caps.get(2) {
            keywords.insert(name.as_str());
        }
    }

    // 提取 CamelCase/PascalCase 标识符（零拷贝：直接插入 &str）
    for caps in camelcase_regex.captures_iter(code) {
        if let Some(name) = caps.get(0) {
            let word = name.as_str();
            // 过滤常见的大写单词
            if !is_common_camelcase_word(word) {
                keywords.insert(word);
            }
        }
    }

    // 提取 snake_case 标识符（零拷贝：直接插入 &str）
    for caps in snakecase_regex.captures_iter(code) {
        if let Some(name) = caps.get(0) {
            keywords.insert(name.as_str());
        }
    }
}

/// 获取缓存的函数名正则表达式
fn get_function_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // 匹配：fn name(、function name(、def name(
        Regex::new(r"\b(fn|function|def)\s+(\w+)\s*\(").unwrap()
    })
}

/// 获取缓存的类/类型名正则表达式
fn get_class_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // 匹配：class Name、struct Name、interface Name、type Name、enum Name
        Regex::new(r"\b(class|struct|interface|type|enum)\s+(\w+)").unwrap()
    })
}

/// 获取缓存的 CamelCase/PascalCase 正则表达式
fn get_camelcase_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // 匹配以大写字母开头的单词
        Regex::new(r"\b[A-Z][a-zA-Z0-9]*\b").unwrap()
    })
}

/// 获取缓存的 snake_case 正则表达式
fn get_snakecase_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // 匹配 snake_case 标识符和普通小写标识符
        // 至少 3 个字符，支持纯小写或带下划线的命名
        Regex::new(r"\b[a-z][a-z0-9]*(?:_[a-z0-9]+)*\b").unwrap()
    })
}

/// 检查是否是编程语言关键字（O(log N) 二分查找）
fn is_language_keyword(word: &str) -> bool {
    // 常见的编程语言关键字（按字母序排序，支持二分查找）
    // 包含常用编程语言关键字和常见英文单词
    const KEYWORDS: &[&str] = &[
        "and",
        "as",
        "assert",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "def",
        "default",
        "del",
        "defer",
        "do",
        "elif",
        "else",
        "enum",
        "except",
        "export",
        "extends",
        "fallthrough",
        "false",
        "finally",
        "fn",
        "for",
        "from",
        "func",
        "global",
        "goto",
        "if",
        "impl",
        "import",
        "in",
        "instanceof",
        "interface",
        "is",
        "lambda",
        "let",
        "loop",
        "map",
        "match",
        "mod",
        "move",
        "mut",
        "new",
        "nonlocal",
        "not",
        "null",
        "of",
        "or",
        "package",
        "pass",
        "pub",
        "raise",
        "range",
        "ref",
        "return",
        "select",
        "static",
        "struct",
        "super",
        "switch",
        "trait",
        "true",
        "try",
        "type",
        "typeof",
        "use",
        "var",
        "void",
        "where",
        "while",
        "with",
        "yield",
    ];

    KEYWORDS.binary_search(&word).is_ok()
}

/// 检查是否是常见的 CamelCase 单词（应该被过滤）
///
/// 这些是英文句子中常见的大写单词，不应该作为关键词提取
/// 使用 O(log N) 二分查找
fn is_common_camelcase_word(word: &str) -> bool {
    // 常见的句首大写单词（按字母序排序，支持二分查找）
    const COMMON_WORDS: &[&str] = &[
        "A", "An", "And", "Are", "As", "At", "Be", "Been", "Being", "But", "By", "Can", "Could",
        "Did", "Do", "Does", "Example", "For", "From", "Had", "Has", "Have", "How", "In", "Input",
        "Is", "It", "Its", "May", "Might", "Must", "Not", "Note", "Of", "On", "Or", "Output",
        "Shall", "Should", "Than", "That", "The", "Then", "These", "This", "Those", "To", "Usage",
        "Was", "Were", "What", "When", "Which", "Who", "Why", "Will", "With", "Would",
    ];

    COMMON_WORDS.binary_search(&word).is_ok()
}

/// 查找代码块结束位置
///
/// 扫描内容，找到第一个关闭的 ``` 标记
/// 返回代码块结束后的位置（包含 ``` 标记本身）
///
/// # 返回值
///
/// 返回**字节偏移量** (byte offset)，而非字符索引
/// 这样可以安全地用于字符串切片操作
fn find_code_block_end(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    // 跳过开始的 ```
    while i < len {
        if bytes[i] == b'`' {
            let backtick_count = count_backticks_bytes(&bytes[i..]);
            if backtick_count >= 3 {
                i += backtick_count;
                break;
            }
        }
        i += 1;
    }

    // 查找关闭的 ```
    while i < len {
        if bytes[i] == b'`' {
            let backtick_count = count_backticks_bytes(&bytes[i..]);
            if backtick_count >= 3 {
                return Some(i + backtick_count);
            }
        }
        i += 1;
    }

    None
}

/// 计算连续的反引号数量（基于字节）
///
/// # 参数
///
/// * `bytes` - 字节切片
///
/// # 返回值
///
/// 返回连续反引号的数量
fn count_backticks_bytes(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|&&b| b == b'`').count()
}

/// 智能截断：在最后一个句子结束符处截断
///
/// 如果能在限制内找到句子结束符（. ! ?），在此处截断
/// 否则在限制处截断并添加 ...
fn smart_truncate(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let limit = max_chars.min(chars.len());

    // 从限制位置向前查找最后一个句子结束符
    let mut sentence_end = None;
    for i in (0..limit).rev() {
        if chars[i] == '.' || chars[i] == '!' || chars[i] == '?' {
            // 确保句子结束符后面是空格或换行
            if i + 1 < chars.len() && (chars[i + 1].is_whitespace() || i + 1 == limit) {
                sentence_end = Some(i + 1);
                break;
            }
        }
    }

    match sentence_end {
        Some(pos) => chars[..pos].iter().collect(),
        None => {
            // 找不到句子结束符，在限制处截断
            let truncated: String = chars[..limit].iter().collect();
            // 添加省略号（如果文本被截断了）
            if limit < chars.len() {
                format!("{}...", truncated.trim_end())
            } else {
                truncated
            }
        }
    }
}

/// 解析后的 Markdown 文档
///
/// # 字段
///
/// * `path` - 原始文件路径
/// * `title` - 文档标题（通常是 H1 标题或文件名）
/// * `summary` - 文档摘要（智能提取首段或代码块，最多 1000 字符）
/// * `content` - 完整的 Markdown 内容
/// * `sections` - 按 H2 标题切片的片段列表（拥有所有权）
///
/// # 所有权设计
///
/// 为了简化生命周期管理并避免复杂的借用关系，
/// `ParsedDoc` 中的 `sections` 拥有数据的所有权（而非借用）。
///
/// 权衡：
/// - ❌ 失去零拷贝优势（需要复制切片数据）
/// - ✅ 简化 API 和生命周期（ParsedDoc 无需生命周期参数）
/// - ✅ 更容易序列化和存储（虽然 ParsedDoc 本身不序列化）
///
/// 这个选择是基于实用主义的考虑：在存储层（JSON 序列化）零拷贝优势无法体现。
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub sections: Vec<SlicedSection>, // 拥有所有权的切片
}

/// 拥有所有权的文档切片（用于 ParsedDoc）
///
/// 与 `SlicedDoc<'a>` 不同，这个结构体拥有所有数据的所有权，
/// 不需要生命周期参数。
#[derive(Debug, Clone)]
pub struct SlicedSection {
    pub section_title: String,
    pub content: String,
    pub parent_doc_title: String,
    pub summary: String,
}

/// 表示一个按 H2 标题切片后的文档片段（零拷贝版本）
///
/// # 字段
///
/// * `section_title` - H2 标题文本（拥有所有权）
/// * `content` - 该 H2 下的完整内容（**不包含 H2 标题本身**，从标题之后到下一个 H2 之前，借用切片）
/// * `parent_doc_title` - 父文档的 H1 标题（借用切片）
/// * `summary` - 切片摘要（智能提取首段或代码块，拥有所有权）
///
/// # 零拷贝设计
///
/// `content` 和 `parent_doc_title` 使用借用切片，避免复制数据。
/// `section_title` 和 `summary` 拥有所有权，因为它们需要从解析的多个事件中拼接或计算。
///
/// # 方案 D 优化
///
/// 利用 pulldown-cmark AST 特性，切片从 `Event::End(Heading).range.end` 开始，
/// 因此 `content` 字段**不包含 H2 标题本身**（如 `## Title`）。
///
/// 优势：
/// - 避免 Header Pollution（标题污染摘要）
/// - 节省 Token 开支（存储和 Embedding 不重复标题）
/// - 数据结构更清晰（标题在 `section_title`，内容在 `content`）
#[derive(Debug, Clone)]
pub struct SlicedDoc<'a> {
    pub section_title: String,
    pub content: &'a str,
    pub parent_doc_title: &'a str,
    pub summary: String,
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

    // 性能优化：先 trim，再调用 extract_summary，避免重复 trim
    // 这样可以确保 extract_summary 接收的是已清理的内容，内部无需再次 trim
    let content_cleaned = content.trim().to_string();
    let summary = extract_summary(&content_cleaned);

    // 调用零拷贝切片函数，然后将结果转换为拥有所有权的版本
    // 性能考虑：这里需要复制数据，但权衡是简化了生命周期管理
    let zero_copy_slices = slice_by_headers(&content_cleaned, &title);
    let sections: Vec<SlicedSection> = zero_copy_slices
        .into_iter()
        .map(|slice| SlicedSection {
            section_title: slice.section_title,
            content: slice.content.to_string(), // 借用 → 拥有所有权
            parent_doc_title: slice.parent_doc_title.to_string(),
            summary: slice.summary, // 已经拥有所有权，直接移动
        })
        .collect();

    Ok(ParsedDoc {
        path: file_path.to_string(),
        title,
        summary,
        // 存储原始内容，不进行 trim
        // 这样 ParsedDoc.content 保存的是完整的原始文档
        // trim 仅在生成 summary 和 sections 时使用
        content,
        sections,
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
/// - 零拷贝实现：`content` 和 `parent_doc_title` 使用借用切片
/// - **方案 D 优化**：切片从 `Event::End(Heading).range.end` 开始，**不包含 H2 标题本身**
/// - 空切片（标题后无内容）会被自动跳过
///
/// # 方案 D 的优势
///
/// - 避免 Header Pollution：切片内容不含标题，摘要提取更准确
/// - 节省 Token 开支：标题已存储在 `section_title`，无需在 `content` 中重复
/// - 数据结构清晰：标题与内容分离
///
/// # 示例
///
/// ```ignore
/// let content = "# Doc\n\n## Section 1\nContent 1\n\n## Section 2\nContent 2";
/// let slices = slice_by_headers(content, "Doc");
/// assert_eq!(slices.len(), 2);
/// assert_eq!(slices[0].section_title, "Section 1");
/// assert!(!slices[0].content.contains("## Section 1")); // 切片不包含标题
/// assert!(slices[0].content.contains("Content 1"));       // 只包含实际内容
/// ```
pub fn slice_by_headers<'a>(content: &'a str, parent_title: &'a str) -> Vec<SlicedDoc<'a>> {
    let mut slices = Vec::new();

    let parser = Parser::new(content);
    let mut h2_start_indices: Vec<usize> = Vec::new(); // 存储 H2 标题开始位置
    let mut h2_end_indices: Vec<usize> = Vec::new();
    let mut h2_titles: Vec<String> = Vec::new();
    let mut current_h2_title: Option<String> = None;

    // 第一遍遍历：收集所有 H2 标题的结束位置和文本
    // 利用 AST 特性：使用 Event::End(Heading).range.end 作为切片起点
    // 这样切片内容不包含 H2 标题本身，避免 Header Pollution
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading(HeadingLevel::H2, ..)) => {
                // 记录 H2 标题的开始位置（用于计算切片结束边界）
                h2_start_indices.push(range.start);
                current_h2_title = Some(String::new());
            }
            Event::End(Tag::Heading(HeadingLevel::H2, ..)) => {
                // 安全修复：保证 h2_titles 和 h2_end_indices 长度一致
                // 即使标题为空，也添加占位符，避免数组索引错位
                let title = current_h2_title.take().unwrap_or_default();
                h2_titles.push(title);
                // 关键改动：存储 range.end（标题结束位置）
                // 这样切片从标题之后开始，不包含标题本身
                h2_end_indices.push(range.end);
            }
            Event::Text(text) | Event::Code(text) => {
                // 处理文本和行内代码
                if let Some(title) = &mut current_h2_title {
                    title.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                // 处理换行，转换为空格
                if let Some(title) = &mut current_h2_title {
                    title.push(' ');
                }
            }
            _ => {}
        }
    }

    // 如果没有 H2 标题，返回空向量
    if h2_end_indices.is_empty() {
        return slices;
    }

    // 第二阶段：根据 H2 结束位置进行切片
    for (i, &end_idx) in h2_end_indices.iter().enumerate() {
        // 计算切片的结束位置
        // 关键修复：使用 h2_start_indices[i + 1] 而不是 h2_end_indices[i + 1]
        // h2_end_indices 存储的是标题结束位置，使用下一个 H2 的结束位置会导致当前切片包含下一个 H2 标题
        // 正确做法是使用下一个 H2 的开始位置，确保切片在下一个 H2 标题之前结束
        let slice_end = if i + 1 < h2_start_indices.len() {
            h2_start_indices[i + 1]
        } else {
            content.len()
        };

        // 跳过标题后的所有空白字符（换行、空格、制表符等），找到实际内容的起始位置
        let after_title = &content[end_idx..];
        let content_start = skip_leading_whitespace(after_title);

        // 计算实际内容的起始偏移量
        let skipped_bytes = after_title.len() - content_start.len();
        let start_byte_offset = end_idx + skipped_bytes;

        // 检查是否是空切片：使用精确的边界计算
        // slice_end 已经通过 h2_start_indices[i+1] 精确界定
        // 如果 start_byte_offset >= slice_end，说明没有实际内容
        let is_empty = start_byte_offset >= slice_end;

        if is_empty {
            continue;
        }

        let slice_content = &content[start_byte_offset..slice_end];

        // 性能优化（方案D）：只 trim 一次，复用结果
        // 1. 检查是否为空切片（安全修复）
        // 2. 传入 extract_summary，避免内部再次 trim
        let slice_content_trimmed = slice_content.trim();

        // 安全修复：跳过空切片（只有空白字符的内容）
        if slice_content_trimmed.is_empty() {
            continue;
        }

        let mut section_title = h2_titles.get(i).cloned().unwrap_or_default();

        // 智能标题生成：如果标题为空，从内容自动生成有意义的标题
        // 这避免了数据丢失，同时保持标题的可读性
        if section_title.is_empty() {
            section_title = generate_smart_title(slice_content_trimmed);
        }

        // 性能优化：传入已 trim 的内容，避免 extract_summary 内部重复 trim
        let summary = extract_summary(slice_content_trimmed);

        slices.push(SlicedDoc {
            section_title,
            content: slice_content,
            parent_doc_title: parent_title,
            summary,
        });
    }

    slices
}

/// 从切片内容生成智能标题
///
/// 当 H2 标题为空时，从内容的前几个字符生成有意义的标题。
///
/// # 逻辑
/// 1. 提取内容的前几个词（最多 30 个字符）
/// 2. 查找第一个句子结束符（。！！？.!?）作为截断点
/// 3. 如果是代码块开头，使用 "Code: ..." 前缀
/// 4. 清理空白和换行
/// 5. 如果太长，添加省略号
///
/// # 参数
/// - `content`: 切片内容（应该已 trim）
///
/// # 返回
/// 生成的智能标题（如果内容为空，返回 "Untitled Section"）
fn generate_smart_title(content: &str) -> String {
    const MAX_TITLE_CHARS: usize = 30;

    if content.is_empty() {
        return "Untitled Section".to_string();
    }

    // 检查是否以代码块开始
    let is_code_block = content.starts_with("```");

    // 提取前几个字符作为标题基础
    let content_start = if is_code_block {
        // 对于代码块，跳过 ``` 后提取
        let after_backticks = &content[3..];
        let first_line = after_backticks.lines().next().unwrap_or("");
        format!("Code: {}", first_line.trim())
    } else {
        // 提取第一行或前 MAX_TITLE_CHARS 个字符
        let first_line = content.lines().next().unwrap_or("");
        if first_line.chars().count() > MAX_TITLE_CHARS {
            // 尝试在句子边界截断
            find_sentence_break(first_line, MAX_TITLE_CHARS)
        } else {
            first_line.to_string()
        }
    };

    // 清理标题
    let title = content_start
        .trim()
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // 如果仍然太长，强制截断
    if title.chars().count() > MAX_TITLE_CHARS {
        let truncated: String = title.chars().take(MAX_TITLE_CHARS - 3).collect();
        format!("{}...", truncated)
    } else {
        title
    }
}

/// 在句子结束符处截断文本
///
/// 查找最后一个句子结束符（。！！？.!?）并在此处截断
fn find_sentence_break(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let limit = max_chars.min(chars.len());

    // 从限制位置向前查找最后一个句子结束符
    for i in (0..limit).rev() {
        let c = chars[i];
        if c == '。' || c == '！' || c == '？' || c == '.' || c == '!' || c == '?' {
            return chars[..=i].iter().collect();
        }
    }

    // 没找到句子结束符，在限制处截断
    let truncated: String = chars[..limit].iter().collect();
    format!("{}...", truncated.trim_end())
}

/// 跳过字符串开头的所有空白字符（换行、空格、制表符等）
///
/// 用于去除切片标题后可能存在的空行和空白
///
/// # 示例
/// - `"\n\nContent"` → `"Content"`
/// - `"  \t\nContent"` → `"Content"`
/// - `"\n"` → `""`
/// - `"  \t  "` → `""`
fn skip_leading_whitespace(s: &str) -> &str {
    s.trim_start_matches(|c: char| c.is_whitespace())
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

    // Summary extraction tests
    #[test]
    fn test_extract_summary_normal_paragraph() {
        let content = "这是第一段。\n\n这是第二段...";
        let summary = extract_summary(content);
        assert_eq!(summary, "这是第一段。");
    }

    #[test]
    fn test_extract_summary_with_code_block() {
        let content = "```rust\npub fn foo() -> Bar\n```\n\n一些说明文字...";
        let summary = extract_summary(content);
        assert!(summary.contains("```rust"));
        assert!(summary.contains("pub fn foo() -> Bar"));
        assert!(summary.contains("```"));
    }

    #[test]
    fn test_extract_summary_code_block_with_newlines() {
        let content = "```rust\npub fn foo(\n    x: i32\n) -> Bar\n```\n\n说明";
        let summary = extract_summary(content);
        // 应该包含完整的代码块，即使内部有换行
        assert!(summary.contains("```rust"));
        assert!(summary.contains("pub fn foo("));
        assert!(summary.contains(") -> Bar"));
        assert!(summary.contains("```"));
    }

    #[test]
    fn test_extract_summary_no_paragraph_break() {
        let content = "短文本或没有双换行的长文本...";
        let summary = extract_summary(content);
        // 应该返回原内容（短于 200 字符）
        assert_eq!(summary, content);
    }

    #[test]
    fn test_extract_summary_long_without_break() {
        let content = "a".repeat(300);
        let summary = extract_summary(&content);
        // 应该截断到 200 字符左右
        assert!(summary.len() <= 203); // 200 + "..."
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_extract_summary_wall_of_text() {
        let content = "这是一个超长的段落，用户从不换行。".repeat(100);
        let summary = extract_summary(&content);
        // 应该截断到 1000 字符左右
        assert!(summary.chars().count() <= 1003);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_extract_summary_empty_content() {
        let summary = extract_summary("");
        assert_eq!(summary, "");
    }

    #[test]
    fn test_extract_summary_whitespace_only() {
        let summary = extract_summary("   \n\n   ");
        assert_eq!(summary, "");
    }

    #[test]
    fn test_extract_summary_sentence_truncation() {
        let content = "这是第一句话。这是第二句话。这是第三句话。这是第四句话。".repeat(10);
        let summary = extract_summary(&content);
        // 应该在句子结束处截断
        assert!(summary.chars().count() <= 1003);
        // 检查是否在句子边界截断（以 . ! ? 结尾）
        let last_char = summary.chars().last().unwrap();
        if summary.ends_with("...") {
            // 如果添加了 ...，前面的内容可能不在句子边界
            assert!(summary.chars().count() <= 1003);
        } else {
            assert!(last_char == '.' || last_char == '!' || last_char == '?');
        }
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

        // 关键验证：切片内容不包含 H2 标题（方案 D 的核心改进）
        assert!(!slices[0].content.contains("## Section One"));
        assert!(!slices[1].content.contains("## Section Two"));
        assert!(!slices[2].content.contains("## Section Three"));

        // 切片内容应该只包含实际内容
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

        // 方案 D：切片内容不包含 H2 标题
        assert!(!slices[0].content.contains("## Section One"));

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

        // 方案 D：空切片被跳过（Section One 后面直接是 Section Two，没有实际内容）
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].section_title, "Section Two");
        assert!(slices[0].content.contains("Some content."));
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

    // 关键词提取测试
    #[test]
    fn test_extract_function_names() {
        let content = r#"
```rust
pub fn create_item() -> Result<Item> {
    let item = Item::new();
    Ok(item)
}

fn process_data(data: &str) -> String {
    data.to_string()
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        assert!(keywords.contains(&"create_item".to_string()));
        assert!(keywords.contains(&"process_data".to_string()));
    }

    #[test]
    fn test_extract_class_names() {
        let content = r#"
```typescript
class BlockCustomComponent {
    render() {
        return null;
    }
}

interface UserProfile {
    name: string;
}

struct Config {
    debug: bool,
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        assert!(keywords.contains(&"BlockCustomComponent".to_string()));
        assert!(keywords.contains(&"UserProfile".to_string()));
        assert!(keywords.contains(&"Config".to_string()));
    }

    #[test]
    fn test_extract_deduplication() {
        let content = r#"
```rust
fn process() {
    let process = Process::new();
    process.run();
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 去重后应该只有一个 "process"
        let process_count = keywords.iter().filter(|k| *k == "process").count();
        assert_eq!(process_count, 1);
    }

    #[test]
    fn test_extract_filters_keywords() {
        let content = r#"
```rust
fn main() {
    let mut value = 42;
    const MAX: i32 = 100;
    return value;
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 应该过滤掉 fn、let、mut、const、return 等关键字
        assert!(!keywords.contains(&"fn".to_string()));
        assert!(!keywords.contains(&"let".to_string()));
        assert!(!keywords.contains(&"mut".to_string()));
        assert!(!keywords.contains(&"const".to_string()));
        assert!(!keywords.contains(&"return".to_string()));
        // 应该保留函数名 main 和常量名 MAX（CamelCase）
        assert!(keywords.contains(&"main".to_string()));
        assert!(keywords.contains(&"MAX".to_string()));
        // 注意：value 不会被提取，因为它不是函数名、类名，也不是 CamelCase
    }

    #[test]
    fn test_extract_filters_short_identifiers() {
        let content = r#"
```rust
fn foo() {
    let a = 1;
    let b = 2;
    let id = 123;
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 应该过滤掉短标识符（< 3 字符）
        assert!(!keywords.contains(&"a".to_string()));
        assert!(!keywords.contains(&"b".to_string()));
        assert!(!keywords.contains(&"id".to_string()));
        // 但应该保留 foo 和其他 >= 3 字符的标识符
        assert!(keywords.contains(&"foo".to_string()));
    }

    #[test]
    fn test_extract_filters_common_camelcase_words() {
        let content = r#"
```rust
// The function handles the data
fn process_data() {
    // This is a comment about Item
    let item = Item::new();
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 应该过滤掉 The、This 等常见大写单词
        assert!(!keywords.contains(&"The".to_string()));
        assert!(!keywords.contains(&"This".to_string()));
        // 但应该保留 Item（类型名）
        assert!(keywords.contains(&"Item".to_string()));
    }

    #[test]
    fn test_extract_no_code_blocks() {
        let content = r#"
This is a regular paragraph.

The function createItem is mentioned in plain text.

Another paragraph with some words.
"#;

        let keywords = extract_code_block_keywords(content);
        // 没有代码块，应该返回空列表
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_extract_multiple_code_blocks() {
        let content = r#"
```rust
fn create_item() -> Item {
    Item::new()
}
```

Some text in between.

```javascript
function buildServer() {
    return new Server();
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 应该从两个代码块中提取关键词
        assert!(keywords.contains(&"create_item".to_string()));
        assert!(keywords.contains(&"Item".to_string()));
        assert!(keywords.contains(&"buildServer".to_string()));
        assert!(keywords.contains(&"Server".to_string()));
    }

    #[test]
    fn test_extract_sorted_output() {
        let content = r#"
```rust
fn zebra() {}
fn apple() {}
fn banana() {}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 结果应该是排序的
        let mut sorted_keywords = keywords.clone();
        sorted_keywords.sort();
        assert_eq!(keywords, sorted_keywords);
    }

    #[test]
    fn test_extract_snake_case_identifiers() {
        let content = r#"
```rust
fn create_item() {
    let map = HashMap::new();
    let range = 0..10;
    process_data(&map, &range);
}
```
"#;

        let keywords = extract_code_block_keywords(content);
        // 应该提取 snake_case 标识符
        assert!(keywords.contains(&"create_item".to_string()));
        assert!(keywords.contains(&"process_data".to_string()));
        // 应该提取 HashMap（CamelCase 类型名）
        assert!(keywords.contains(&"HashMap".to_string()));
        // 关键修复：map, new, range 应该被过滤（它们是常见编程语言关键字）
        // 这避免了搜索噪音污染，确保只有有意义的标识符被提取
        assert!(
            !keywords.contains(&"map".to_string()),
            "map 应该被过滤，它是常见关键字"
        );
        assert!(
            !keywords.contains(&"new".to_string()),
            "new 应该被过滤，它是常见关键字"
        );
        assert!(
            !keywords.contains(&"range".to_string()),
            "range 应该被过滤，它是常见关键字"
        );
    }
}

#[test]
fn test_edge_cases_empty_h2() {
    // 测试空的 H2 标题
    let content = "# Parent\n\n##\n\nContent after empty header.";
    let slices = slice_by_headers(content, "Parent");

    // 智能标题生成：空标题会从内容生成有意义的标题
    // 避免数据丢失，保持内容可搜索
    assert_eq!(
        slices.len(),
        1,
        "Empty H2 titles should generate smart titles from content"
    );

    // 验证智能标题包含内容的前几个词
    assert!(
        slices[0].section_title.contains("Content after"),
        "Smart title should be generated from content"
    );

    // 验证内容被保留
    assert!(
        slices[0].content.contains("Content after empty header"),
        "Content should be preserved"
    );
}

#[test]
fn test_edge_cases_h2_at_eof() {
    // 测试 H2 后面直接 EOF（没有内容）
    let content = "# Parent\n\n## Section One";
    let slices = slice_by_headers(content, "Parent");

    // 方案 D：空切片被跳过
    assert_eq!(slices.len(), 0);
}

#[test]
fn test_edge_cases_consecutive_h2() {
    // 测试连续的 H2（中间没有内容）
    let content = "# Parent\n\n## First\n## Second\n## Third\n\nContent.";
    let slices = slice_by_headers(content, "Parent");

    // 方案 D：只有包含实际内容的切片会被保留
    // First 和 Second 后面直接是下一个标题，所以被跳过
    // Third 后面有 "Content."，所以被保留
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].section_title, "Third");
    assert!(slices[0].content.contains("Content."));
}
