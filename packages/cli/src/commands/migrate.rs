use anyhow::Result;
use contextfy_core::migration::{migrate_json_to_lancedb, MigrationConfig};
use std::path::PathBuf;

/// 执行 JSON 到 LanceDB 的数据迁移
///
/// 将旧版 JSON 缓存文件迁移到新的 LanceDB 向量数据库存储。
///
/// # Arguments
///
/// * `json_path` - JSON 文件路径
/// * `lancedb_uri` - LanceDB 连接 URI
/// * `table_name` - 目标表名（默认 "knowledge"）
/// * `batch_size` - 批处理大小（默认 100）
/// * `skip_errors` - 是否跳过错误（默认 false）
/// * `backup` - 是否创建备份（默认 true）
///
/// # Errors
///
/// 如果迁移失败，返回错误
///
/// # Examples
///
/// ```no_run
/// # use contextfy_cli::commands::migrate;
/// # use std::path::PathBuf;
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// migrate(
///     PathBuf::from("cache.json"),
///     "lancedb://~/.contextfy/db".to_string(),
///     Some("knowledge".to_string()),
///     Some(100),
///     Some(false),
///     Some(true)
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn migrate(
    json_path: PathBuf,
    lancedb_uri: String,
    table_name: Option<String>,
    batch_size: Option<usize>,
    skip_errors: Option<bool>,
    backup: Option<bool>,
) -> Result<()> {
    println!("🚀 Starting migration from JSON to LanceDB...");
    println!("📂 JSON file: {}", json_path.display());
    println!("🗄️  LanceDB URI: {}", lancedb_uri);

    let config = MigrationConfig {
        json_path,
        lancedb_uri,
        table_name: table_name.unwrap_or_else(|| "knowledge".to_string()),
        batch_size: batch_size.unwrap_or(100),
        skip_errors: skip_errors.unwrap_or(false),
        backup: backup.unwrap_or(true),
    };

    println!("⚙️  Configuration:");
    println!("   - Table name: {}", config.table_name);
    println!("   - Batch size: {}", config.batch_size);
    println!("   - Skip errors: {}", config.skip_errors);
    println!("   - Create backup: {}", config.backup);
    println!();

    let stats = migrate_json_to_lancedb(config).await?;

    println!();
    println!("✅ Migration completed successfully!");
    println!("📊 Statistics:");
    println!("   - Total processed: {}", stats.total_processed);
    println!("   - Successful: {}", stats.successful);
    println!("   - Failed: {}", stats.failed);
    println!("   - Skipped: {}", stats.skipped);
    println!("   - Success rate: {:.1}%", stats.success_rate() * 100.0);

    Ok(())
}
