use axum::{routing::get, Json, Router};
use serde_json::json;
use tokio::net::TcpListener;
use tracing::info;
use tucano_b3::config::B3Settings;
use tucano_b3::db::{create_pool, run_migrations};

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "app": "tucano-b3",
        "version": "0.1.0"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let settings = B3Settings::default();
    info!("Starting tucano-b3 application (Rust Edition)...");

    match create_pool(&settings.database_url).await {
        Ok(pool) => {
            info!("Running database migrations...");
            if let Err(e) = run_migrations(&pool).await {
                tracing::error!("Migration failed: {:?}", e);
            } else {
                info!("Migrations applied successfully.");
            }
        }
        Err(e) => {
            tracing::warn!("Database connection unavailable on boot: {:?}", e);
        }
    }

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/api/v1/health", get(health));

    let bind_addr = "0.0.0.0:8000";
    info!("tucano-b3 listening on http://{}", bind_addr);
    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
