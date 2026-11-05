pub mod enums;
pub mod models;
pub mod locks;
pub mod upsert;
pub mod resolver;

use sqlx::{postgres::PgPoolOptions, PgPool};
use crate::error::Result;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;
    Ok(())
}

