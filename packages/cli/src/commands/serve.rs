use anyhow::Result;

pub fn serve() -> Result<()> {
    println!("Starting server on http://127.0.0.1:3000...");
    println!("Note: Use 'cargo run --bin contextfy-server' for full server.");
    println!("The server needs to be built with: cargo build --bin contextfy-server");
    Ok(())
}
