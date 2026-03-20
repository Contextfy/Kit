//! Storage layer - deprecated, use slices instead
//!
//! **DEPRECATED**: This module is deprecated. Use `slices::vector` instead.

pub mod lancedb_store;

use serde::{Deserialize, Serialize};

/// 知识库中的一条记录
///
/// # 字段
///
/// * `id` - 记录的唯一标识符（UUID）
/// * `title` - 记录标题（对于切片文档，这是 H2 标题）
/// * `parent_doc_title` - 父文档的标题（H1 标题或文件名）
/// * `summary` - 内容摘要（前 200 个字符）
/// * `content` - 完整内容
/// * `source_path` - 原始文件路径，用于追溯源文件
/// * `keywords` - 关键词列表（用于全文搜索，为 Issue #10 打桩）
/// * `embedding` - 向量嵌入（384 维浮点数组，用于语义搜索）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub parent_doc_title: String,
    pub summary: String,
    pub content: String,
    #[serde(default)]
    pub source_path: String, // 新增字段：记录原始文件路径，向后兼容旧版 JSON
    #[serde(default)]
    pub keywords: Vec<String>, // 关键词列表（为 Issue #10 打桩）
    #[serde(default)]
    pub embedding: Option<Vec<f32>>, // 向量嵌入（用于语义搜索，向后兼容旧版 JSON）
}
