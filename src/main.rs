//! Entry point for tucano-b3.
//!
//! Loads configuration, initializes logging, and starts the HTTP server.

use std::path::Path;
use tucano_b3::config::AppConfig;
use tucano_b3::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI args for config path
    let config_path = std::env::args()
        .skip(1)
        .position(|arg| arg == "--config")
        .and_then(|idx| std::env::args().nth(idx + 2))
        .map(|p| Path::new(Box::leak(p.into_boxed_str())).to_path_buf());

    // Load configuration
    let config = AppConfig::load(config_path.as_deref())
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    // Initialize structured logging
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.logging.level));

    match config.logging.format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .init();
        }
        "pretty" => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        port = config.server.port,
        "tucano-b3 iniciando"
    );

    // Run server
    server::run(config).await?;

    Ok(())
}
