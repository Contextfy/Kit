use clap::{Parser, Subcommand};
mod commands;

use commands::{build, init, migrate, scout, serve};

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
    /// Migrate JSON data to LanceDB
    Migrate {
        /// Path to JSON file to migrate
        #[arg(short, long)]
        json: Option<std::path::PathBuf>,
        /// LanceDB connection URI
        #[arg(short = 'd', long)]
        lancedb_uri: Option<String>,
        /// Target table name (default: knowledge)
        #[arg(short, long)]
        table: Option<String>,
        /// Number of records to process per batch (default: 100)
        #[arg(short, long)]
        batch_size: Option<usize>,
        /// Skip malformed records instead of failing
        #[arg(short, long)]
        skip_errors: bool,
        /// Do not create backup of original JSON file
        #[arg(long)]
        no_backup: bool,
    },
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
        Commands::Migrate {
            json,
            lancedb_uri,
            table,
            batch_size,
            skip_errors,
            no_backup,
        } => {
            // Use MigrationConfig defaults which properly expand home directory
            let defaults = contextfy_core::migration::MigrationConfig::default();

            let json_path = json.unwrap_or_else(|| defaults.json_path);
            let db_uri = lancedb_uri.unwrap_or_else(|| defaults.lancedb_uri);

            migrate(
                json_path,
                db_uri,
                table,
                batch_size,
                Some(skip_errors),
                Some(!no_backup),
            )
            .await?;
        }
    }

    Ok(())
}
