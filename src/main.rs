use clap::{Parser, Subcommand};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use tucano_b3::api::{create_router, AppState};
use tucano_b3::config::B3Settings;
use tucano_b3::db::enums::SourceFileType;
use tucano_b3::db::{create_pool, run_migrations};
use tucano_b3::mcp::McpServer;
use tucano_b3::services::sync_service::B3SyncService;
use tucano_b3::worker::B3Worker;

#[derive(Parser)]
#[command(name = "tucano-b3", version, about = "Tucano B3 Exchange Data Engine (Rust Edition)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Axum HTTP REST API Server (default)
    Serve {
        #[arg(short, long, default_value = "0.0.0.0:8000")]
        bind: String,
    },
    /// Start the background synchronization worker daemon
    Worker,
    /// Run a manual synchronization for a source file type and date
    Sync {
        #[arg(short, long)]
        source_type: String,
        #[arg(short, long)]
        date: String,
    },
    /// Start the read-only Model Context Protocol (MCP) server over stdio
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let settings = Arc::new(B3Settings::default());

    let pool = create_pool(&settings.database_url).await?;
    info!("Running database migrations...");
    run_migrations(&pool).await?;
    info!("Migrations verified.");

    let sync_service = Arc::new(B3SyncService::new(pool.clone(), (*settings).clone()));

    match cli.command.unwrap_or(Commands::Serve { bind: "0.0.0.0:8000".to_string() }) {
        Commands::Serve { bind } => {
            info!("Starting Tucano B3 HTTP REST API Server on {}...", bind);
            let app_state = Arc::new(AppState {
                pool,
                settings,
                sync_service,
            });

            let router = create_router(app_state);
            let listener = TcpListener::bind(&bind).await?;
            info!("Server listening on http://{}", bind);
            axum::serve(listener, router).await?;
        }
        Commands::Worker => {
            info!("Starting Tucano B3 Worker daemon...");
            let worker = B3Worker::new(settings, sync_service);
            worker.run_loop().await;
        }
        Commands::Mcp => {
            info!("Starting Tucano B3 MCP Server (stdio)...");
            let mcp = McpServer::new(pool);
            mcp.run_stdio().await?;
        }
        Commands::Sync { source_type, date } => {
            let sft = SourceFileType::from_str(&source_type).map_err(|e| anyhow::anyhow!(e))?;
            let ref_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
            info!("Executing manual sync for {} on {}", sft, ref_date);
            let outcome = sync_service.run_manual_sync(
                sft,
                ref_date,
                ref_date,
                None,
                None,
                false,
                false,
                Some("cli_user".to_string()),
            ).await?;
            println!("Sync result: {:?}", outcome);
        }
    }

    Ok(())
}
