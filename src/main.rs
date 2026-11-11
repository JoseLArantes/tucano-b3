//! Entry point for tucano-b3.
//!
//! Loads configuration, initializes logging, and starts the HTTP server.

use std::path::Path;
use std::sync::Arc;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
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

    // Initialize cache
    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));

    // Initialize data layer
    let data_loader = Arc::new(
        DataLoader::new(&config.data, cache.clone())
            .map_err(|e| anyhow::anyhow!("Failed to load data: {}", e))?,
    );

    // Initialize API key registry
    let api_keys_path = Path::new("api_keys.toml");
    let api_keys = Arc::new(
        tucano_b3::middleware::ApiKeyRegistry::load(api_keys_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load API keys: {}", e);
            tucano_b3::middleware::ApiKeyRegistry::empty()
        }),
    );

    // Initialize rate limit state
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        port = config.server.port,
        "tucano-b3 iniciando"
    );

    // Run server with data loader, cache, and rate limit state
    server::run(config, data_loader, cache, rate_limit_state).await?;

    Ok(())
}
