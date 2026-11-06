pub mod enums;
pub mod history;
pub mod locks;
pub mod models;
pub mod resolver;
pub mod upsert;

use crate::error::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let max_connections = match std::env::var("B3_DB_MAX_CONNECTIONS") {
        Ok(value) => value
            .parse::<u32>()
            .ok()
            .filter(|v| (1..=32).contains(v))
            .ok_or_else(|| {
                crate::error::AppError::Validation(
                    "B3_DB_MAX_CONNECTIONS deve estar entre 1 e 32.".into(),
                )
            })?,
        Err(_) => 8,
    };
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET statement_timeout='30s'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Runtime processes only inspect schema; DDL belongs to the migration command.
pub async fn verify_schema(pool: &PgPool) -> Result<()> {
    let migrator = sqlx::migrate!("./migrations");
    for migration in migrator.iter() {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=$1 AND success AND checksum=$2)")
            .bind(migration.version).bind(migration.checksum.as_ref()).fetch_one(pool).await?;
        if !valid {
            return Err(crate::error::AppError::Validation(
                "Schema incompatível; execute o comando migrate com credencial própria.".into(),
            ));
        }
    }
    Ok(())
}
