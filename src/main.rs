use clap::{Parser, Subcommand, ValueEnum};
use std::{str::FromStr, sync::Arc};
use tokio::net::TcpListener;
use tucano_b3::{
    api::{create_router, AppState},
    config::B3Settings,
    db::{create_pool, enums::SourceFileType, run_migrations},
    mcp::McpServer,
    services::{
        jobs::{ExecutionLane, JobCommand, JobPriority, JobService},
        runtime::{discovery::DiscoveryService, RuntimeSettings},
        sync_service::B3SyncService,
    },
    worker::B3Worker,
};
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "tucano-b3",
    version,
    about = "Histórico oficial B3 com execução administrativa rastreada"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Lane {
    Maintenance,
    Heavy,
}

#[derive(Subcommand)]
enum Commands {
    Serve {
        #[arg(long, default_value = "0.0.0.0:8000")]
        bind: String,
    },
    Migrate,
    Worker {
        #[arg(long)]
        once: bool,
        #[arg(long, value_enum, default_value = "maintenance")]
        lane: Lane,
    },
    Sync {
        #[arg(short, long)]
        source_type: String,
        #[arg(short, long)]
        date: String,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    Enqueue {
        #[arg(long)]
        command_file: std::path::PathBuf,
        #[arg(long)]
        idempotency_key: String,
    },
    Discover,
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_filter = std::env::var("RUST_LOG")
        .or_else(|_| std::env::var("LOG_LEVEL"))
        .unwrap_or_else(|_| "info".to_owned());
    let log_filter = tracing_subscriber::EnvFilter::try_new(log_filter)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(log_filter)
        .init();
    let cli = Cli::parse();
    let settings = Arc::new(B3Settings::default());
    let pool = create_pool(&settings.database_url).await?;
    let runtime = RuntimeSettings::from_settings(&settings);
    runtime.validate()?;
    if !matches!(cli.command, Some(Commands::Migrate)) {
        tucano_b3::db::verify_schema(&pool).await?;
    }
    match cli.command.unwrap_or(Commands::Serve {
        bind: "0.0.0.0:8000".into(),
    }) {
        Commands::Migrate => {
            run_migrations(&pool).await?;
        }
        Commands::Serve { bind } => {
            settings.validate_api_credentials()?;
            let state = Arc::new(AppState {
                pool: pool.clone(),
                settings: settings.clone(),
                sync_service: Arc::new(B3SyncService::new(pool, (*settings).clone())),
            });
            axum::serve(TcpListener::bind(bind).await?, create_router(state)).await?;
        }
        Commands::Worker { once, lane } => {
            let worker = B3Worker::new(pool, runtime);
            if once {
                worker
                    .run_once(
                        match lane {
                            Lane::Maintenance => ExecutionLane::Maintenance,
                            Lane::Heavy => ExecutionLane::Heavy,
                        },
                        Uuid::new_v4(),
                    )
                    .await?;
            } else {
                worker.run_loop().await?;
            }
        }
        Commands::Mcp => McpServer::new(pool).run_stdio().await?,
        Commands::Sync {
            source_type,
            date,
            idempotency_key,
        } => {
            let date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
            let command = JobCommand::Ingest {
                source: SourceFileType::from_str(&source_type).map_err(anyhow::Error::msg)?,
                channel_id: None,
                from: date,
                to: date,
            };
            let id = JobService::new(pool)
                .submit(
                    &idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string()),
                    command,
                    JobPriority::Maintenance,
                    3,
                )
                .await?;
            println!("{}", serde_json::json!({"job_id":id,"status":"queued"}));
        }
        Commands::Enqueue {
            command_file,
            idempotency_key,
        } => {
            let command: JobCommand = serde_json::from_reader(std::fs::File::open(command_file)?)?;
            let id = JobService::new(pool)
                .submit(&idempotency_key, command, JobPriority::Maintenance, 3)
                .await?;
            println!("{}", serde_json::json!({"job_id":id,"status":"queued"}));
        }
        Commands::Discover => {
            DiscoveryService::new(pool, runtime)
                .scan(Uuid::new_v4(), false, chrono::Utc::now())
                .await?;
        }
    }
    Ok(())
}
