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
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "tucano-b3 {}\n\
API REST de alta performance para dados de mercado da B3\n\n\
USO:\n    \
    tucano-b3 [OPÇÕES]\n\n\
OPÇÕES:\n    \
    --config <ARQUIVO>    Caminho para o arquivo de configuração [padrão: config.toml]\n    \
    -h, --help            Imprime informações de ajuda\n    \
    -V, --version         Imprime informações de versão",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("tucano-b3 {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Parse CLI args for config path
    let config_path = args
        .windows(2)
        .find(|w| w[0] == "--config")
        .map(|w| Path::new(&w[1]).to_path_buf());

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

    // Initialize metrics
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();

    // Update initial data metrics
    let query_service = data_loader.get_query_service().await;
    metrics.update_data_metrics(
        query_service.cotahist_file_count(),
        query_service.total_cotahist_rows(),
        query_service.ibov_row_count(),
        data_loader.ticker_registry.count(),
    );

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        port = config.server.port,
        "tucano-b3 iniciando"
    );

    // Run server with data loader, cache, rate limit state, metrics, and start time
    server::run(
        config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    )
    .await?;

    Ok(())
}
