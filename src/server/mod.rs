//! HTTP server module for tucano-b3.
//!
//! Uses axum with tower middleware for routing, compression, caching, rate limiting, and metrics.
//! Supports graceful shutdown via SIGTERM/SIGINT.

use crate::api::openapi::ApiDoc;
use crate::cache::store::CacheStore;
use crate::config::AppConfig;
use crate::data::reload::DataLoader;
use crate::error::AppError;
use crate::metrics::AppMetrics;
use crate::middleware::{
    AuthLayer, CacheLayer, MetricsLayer, RateLimitLayer, RateLimitState, RequestIdLayer,
    RequestLoggingLayer,
};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

const REDOC_HTML: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <title>tucano-b3 API - ReDoc</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="shortcut icon" href="https://redocly.com/favicon.png">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>
      body {
        margin: 0;
        padding: 0;
      }
    </style>
  </head>
  <body>
    <div id="redoc-container"></div>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
    <script>
      Redoc.init('/api/docs/openapi.json', {
        scrollYOffset: 50,
        expandResponses: "200,201",
        requiredPropsFirst: true,
        sortPropsAlphabetically: true
      }, document.getElementById('redoc-container'))
    </script>
  </body>
</html>"#;

async fn redoc_handler() -> axum::response::Html<&'static str> {
    axum::response::Html(REDOC_HTML)
}

/// Build the axum Router with all routes and middleware.
///
/// This function is called once at startup and returns a fully-configured router.
pub fn build_router(
    config: &AppConfig,
    data_loader: Arc<DataLoader>,
    cache: Arc<CacheStore>,
    rate_limit_state: Arc<RateLimitState>,
    metrics: Arc<AppMetrics>,
    start_time: std::time::Instant,
) -> Router {
    let health_state = Arc::new(crate::api::health::AppState::new(
        data_loader.clone(),
        cache.clone(),
    ));

    let api_routes = crate::api::routes(data_loader, cache.clone())
        .merge(crate::api::metrics::routes(metrics.clone(), start_time));

    Router::new()
        // Public health check alias at root
        .route(
            "/health",
            axum::routing::get(crate::api::health::health_handler).with_state(health_state),
        )
        // Interactive API documentation via ReDoc
        .route("/redoc", axum::routing::get(redoc_handler))
        .route("/api/redoc", axum::routing::get(redoc_handler))
        // API routes nested under /api/v1
        .nest("/api/v1", api_routes)
        // Standalone metrics endpoint (standard Prometheus scrape /metrics)
        .merge(crate::api::metrics::routes(metrics.clone(), start_time))
        // Swagger UI (also serves /api/docs/openapi.json)
        .merge(
            SwaggerUi::new("/api/docs")
                .url("/api/docs/openapi.json", ApiDoc::openapi())
                .config(utoipa_swagger_ui::Config::default().persist_authorization(true)),
        )
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.server.request_timeout_seconds),
        ))
        .layer(CorsLayer::permissive())
        .layer(CacheLayer::new(cache, metrics.clone()))
        .layer(CompressionLayer::new())
        .layer(RateLimitLayer::new(rate_limit_state, metrics.clone()))
        .layer(AuthLayer::new(config.auth.clone()))
        .layer(RequestLoggingLayer::new())
        .layer(RequestIdLayer::new())
        .layer(MetricsLayer::new(metrics))
}

/// Start the HTTP server and block until shutdown signal.
///
/// Handles SIGTERM and SIGINT for graceful shutdown.
/// All in-flight requests are completed before the process exits.
pub async fn run(
    config: AppConfig,
    data_loader: Arc<DataLoader>,
    cache: Arc<CacheStore>,
    rate_limit_state: Arc<RateLimitState>,
    metrics: Arc<AppMetrics>,
    start_time: std::time::Instant,
) -> Result<(), AppError> {
    let router = build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    );

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid bind address: {}", e)))?;

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "Iniciando servidor tucano-b3"
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to bind: {}", e)))?;

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Server error: {}", e)))?;

    tracing::info!("Servidor finalizado com sucesso");
    Ok(())
}

/// Wait for SIGTERM or SIGINT signal.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            tracing::error!("Failed to install CTRL+C signal handler: {}", err);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::error!("Failed to install SIGTERM signal handler: {}", err);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Recebido sinal SIGINT, iniciando encerramento gracioso...");
        }
        _ = terminate => {
            tracing::info!("Recebido sinal SIGTERM, iniciando encerramento gracioso...");
        }
    }
}
