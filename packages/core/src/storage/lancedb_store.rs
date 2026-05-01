//! LanceDB 存储模块
//!
//! 提供向量数据库连接和 Schema 定义，为未来数据迁移做准备。
//!
//! **DEPRECATED**: 此模块将被 slices::vector 完全替代

#![allow(dead_code)]

use anyhow::{Context, Result};
use arrow::datatypes::{DataType, Field, Schema};
use lancedb::connection::Connection as LanceConnection;
use std::sync::Arc;

// Import the rigorous schema validation from the vector slice
use crate::slices::vector::schema::validate_knowledge_schema;

/// 向量维度常量
///
/// 根据实际使用的嵌入模型调整此值：
/// - BGE-small-en-v1.5: 384
/// - BGE-base-en-v1.5: 768
/// - text-embedding-ada-002 (OpenAI): 1536
/// - bge-m3 (multilingual): 1024
///
/// 当前值适配 BGE-small-en-v1.5 模型（本项目默认）
///
/// **注意**：使用 `i32` 类型以匹配 Arrow DataType::FixedSizeList 的要求
pub const VECTOR_DIM: i32 = 384;

/// 定义知识记录的 Arrow Schema
///
/// 该 Schema 与 `KnowledgeRecord` 结构对应，包含以下字段：
/// - id: 记录的唯一标识符（Utf8，非空）
/// - title: 记录标题（Utf8，非空）
/// - summary: 内容摘要（用于 Scout 检索，Utf8，非空）
/// - content: 完整内容（用于 Inspect 检索，Utf8，非空）
/// - vector: 向量嵌入（384 维 FixedSizeList(Float32)，非空）
/// - keywords: JSON 序列化的关键词数组（Utf8，可空）
/// - source_path: 原始文件路径（Utf8，非空）
pub fn knowledge_record_schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("summary", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        // vector: 384 维 Float32 固定大小列表
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), VECTOR_DIM),
            false,
        ),
        // keywords: JSON 序列化数组（字符串格式存储）
        Field::new("keywords", DataType::Utf8, true),
        Field::new("source_path", DataType::Utf8, false),
    ])
}

/// 连接到 LanceDB 数据库
///
/// # 参数
///
/// * `uri` - 数据库连接字符串（如 "lance://data/db" 或本地目录路径）
///
/// # 返回
///
/// 成功返回 `Connection` 对象，失败返回错误
///
/// # 错误
///
/// - 如果 URI 格式无效，返回 "Failed to connect to LanceDB"
/// - 如果数据库不可访问，返回描述性错误
pub async fn connect_lancedb(uri: &str) -> Result<LanceConnection> {
    lancedb::connect(uri)
        .execute()
        .await
        .context("Failed to connect to LanceDB")
}

/// 创建表（如果不存在）
///
/// # 参数
///
/// * `conn` - LanceDB 连接对象
/// * `table_name` - 表名
///
/// # 返回
///
/// 成功返回 ()，失败返回错误
///
/// # 行为
///
/// - 如果表已存在，直接返回成功
/// - 如果表不存在，使用 `knowledge_record_schema()` 创建新表
///
/// # 错误
///
/// - 如果表已存在但 Schema 不匹配，返回 "Table exists but schema mismatch"
/// - 如果创建表失败，返回 "Failed to create table"
pub async fn create_table_if_not_exists(conn: &LanceConnection, table_name: &str) -> Result<()> {
    // 获取现有表名列表
    let existing_tables = conn
        .table_names()
        .execute()
        .await
        .context("Failed to get table names from database")?;

    // 检查表是否已存在
    if existing_tables.contains(&table_name.to_string()) {
        // 表已存在，尝试打开以验证 Schema
        let existing_table = conn
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open existing table: {}", table_name))?;

        // 验证 Schema 是否匹配
        let existing_schema = existing_table
            .schema()
            .await
            .with_context(|| format!("Failed to get schema for existing table: {}", table_name))?;

        // 使用 rigorous schema validation from vector slice
        validate_knowledge_schema(&existing_schema).map_err(|e| {
            anyhow::anyhow!(
                "Table '{}' exists but schema validation failed: {}",
                table_name,
                e
            )
        })?;

        return Ok(());
    }

    // 表不存在，创建新表
    // LanceDB 0.26.2 的 create_empty_table API 接受 Arc<Schema>
    let schema = Arc::new(knowledge_record_schema());
    let new_table = conn
        .create_empty_table(table_name, schema)
        .execute()
        .await
        .with_context(|| format!("Failed to create table: {}", table_name))?;

    // 验证表已创建
    let created_schema = new_table.schema().await.with_context(|| {
        format!(
            "Failed to get schema for newly created table: {}",
            table_name
        )
    })?;

    // 使用 rigorous schema validation from vector slice
    validate_knowledge_schema(&created_schema).map_err(|e| {
        anyhow::anyhow!(
            "Newly created table '{}' failed schema validation: {}",
            table_name,
            e
        )
    })?;

    Ok(())
}

/// 初始化 LanceDB 数据库（辅助函数）
///
/// # 参数
///
/// * `uri` - 数据库连接字符串
/// * `table_name` - 表名
///
/// # 返回
///
/// 成功返回 ()，失败返回错误
///
/// # 错误
///
/// - 如果连接失败，返回 "Failed to connect to LanceDB"
/// - 如果表创建或验证失败，返回描述性错误
///
/// # 示例
///
/// ```text
/// // Note: This module is deprecated. Use slices::vector instead.
/// // Example flow (for reference):
/// let conn = connect_lancedb("data/db").await?;
/// create_table_if_not_exists(&conn, "knowledge").await?;
/// ```
pub async fn initialize_lancedb_db(uri: &str, table_name: &str) -> Result<()> {
    let conn = connect_lancedb(uri).await?;
    create_table_if_not_exists(&conn, table_name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_knowledge_record_schema() {
        let schema = knowledge_record_schema();

        // 验证 Schema 包含 7 个字段
        assert_eq!(schema.fields().len(), 7);

        // 验证字段名称
        let field_names: Vec<_> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            field_names,
            vec![
                "id",
                "title",
                "summary",
                "content",
                "vector",
                "keywords",
                "source_path"
            ]
        );

        // 验证 id 字段类型
        let id_field = schema.field(0);
        assert_eq!(id_field.data_type(), &DataType::Utf8);
        assert!(!id_field.is_nullable());

        // 验证 vector 字段类型和维度
        let vector_field = schema.field(4);
        assert!(!vector_field.is_nullable());
        match vector_field.data_type() {
            DataType::FixedSizeList(field, size) => {
                assert_eq!(*size, VECTOR_DIM);
                assert_eq!(field.data_type(), &DataType::Float32);
            }
            _ => panic!("vector field should be FixedSizeList"),
        }

        // 验证 keywords 字段可空
        let keywords_field = schema.field(5);
        assert!(keywords_field.is_nullable());
    }

    #[tokio::test]
    async fn test_connect_lancedb_temp_dir() {
        // 创建临时目录
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");

        // 测试连接
        let conn = connect_lancedb(db_uri)
            .await
            .expect("Failed to connect to LanceDB");

        // 验证连接成功（仅检查连接对象存在）
        assert!(!conn.database().to_string().is_empty());

        // 临时目录会在 drop 时自动清理
    }

    /// 真实的表创建测试
    ///
    /// 测试完整的建表流程：
    /// 1. 在临时目录中创建数据库
    /// 2. 调用 initialize_lancedb_db 创建表
    /// 3. 验证表已创建且 Schema 正确
    /// 4. 验证 vector 字段为 FixedSizeList(Float32, 384)
    #[tokio::test]
    async fn test_initialize_lancedb_db_creates_table() {
        // 创建临时目录用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_knowledge";

        // 初始化数据库（创建表）
        initialize_lancedb_db(db_uri, table_name)
            .await
            .expect("Failed to initialize LanceDB database");

        // 重新连接并验证表已创建
        let conn = connect_lancedb(db_uri)
            .await
            .expect("Failed to reconnect to LanceDB");

        // 获取表名列表，验证表已创建
        let table_names = conn
            .table_names()
            .execute()
            .await
            .expect("Failed to get table names");
        assert!(
            table_names.contains(&table_name.to_string()),
            "Table '{}' should exist in table names: {:?}",
            table_name,
            table_names
        );

        // 打开表并验证 Schema
        let table = conn
            .open_table(table_name)
            .execute()
            .await
            .expect("Failed to open table");

        let schema = table.schema().await.expect("Failed to get table schema");

        // 验证字段数量
        assert_eq!(schema.fields().len(), 7, "Schema should have 7 fields");

        // 验证 vector 字段
        let vector_field = schema
            .field_with_name("vector")
            .expect("Schema should have 'vector' field");

        match vector_field.data_type() {
            DataType::FixedSizeList(field, size) => {
                assert_eq!(*size, VECTOR_DIM, "Vector dimension should be {}", VECTOR_DIM);
                assert_eq!(
                    field.data_type(),
                    &DataType::Float32,
                    "Vector element type should be Float32"
                );
            }
            _ => panic!(
                "Vector field should be FixedSizeList, got {:?}",
                vector_field.data_type()
            ),
        }

        // 验证 keywords 字段可空
        let keywords_field = schema
            .field_with_name("keywords")
            .expect("Schema should have 'keywords' field");
        assert!(
            keywords_field.is_nullable(),
            "keywords field should be nullable"
        );

        // 临时目录会在 drop 时自动清理
    }

    /// 表已存在时的幂等性测试
    ///
    /// 验证多次调用 initialize_lancedb_db 不会报错
    #[tokio::test]
    async fn test_initialize_lancedb_db_idempotent() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_uri = temp_dir.path().to_str().expect("Invalid path");
        let table_name = "test_idempotent";

        // 第一次初始化
        initialize_lancedb_db(db_uri, table_name)
            .await
            .expect("Failed to initialize LanceDB database (first time)");

        // 第二次初始化（表已存在）
        initialize_lancedb_db(db_uri, table_name)
            .await
            .expect("Failed to initialize LanceDB database (second time)");

        // 验证表仍然存在且 Schema 正确
        let conn = connect_lancedb(db_uri).await.expect("Failed to reconnect");
        let table = conn
            .open_table(table_name)
            .execute()
            .await
            .expect("Failed to open table");

        let schema = table.schema().await.expect("Failed to get schema");
        assert_eq!(schema.fields().len(), 7);
    }
}
