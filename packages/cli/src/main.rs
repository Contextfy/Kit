use clap::{Parser, Subcommand};
mod commands;

use commands::{build, init, scout, serve};

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
            init(template)?;
        }
        Commands::Build => {
            build().await?;
        }
        Commands::Scout { query } => {
            scout(query).await?;
        }
        Commands::Serve => {
            serve()?;
        }
    }

    Ok(())
}
