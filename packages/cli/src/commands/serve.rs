use anyhow::Result;

/// 启动开发服务器
///
/// 启动本地开发服务器用于测试和预览。当前为占位实现，
/// 需要使用 `cargo run --bin contextfy-server` 获取完整服务器功能。
///
/// # Errors
///
/// 当前版本不会返回错误
///
/// # Examples
///
/// ```no_run
/// # use contextfy_cli::commands::serve;
/// # fn main() -> anyhow::Result<()> {
/// serve()?;
/// # Ok(())
/// # }
/// ```
pub fn serve() -> Result<()> {
    println!("Starting server on http://127.0.0.1:3000...");
    println!("Note: Use 'cargo run --bin contextfy-server' for full server.");
    println!("The server needs to be built with: cargo build --bin contextfy-server");
    Ok(())
}
