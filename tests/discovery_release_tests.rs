use chrono::{TimeZone, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::path::PathBuf;
use tucano_b3::{
    db::run_migrations,
    services::runtime::{discovery::DiscoveryService, RuntimeSettings},
};
use uuid::Uuid;

async fn database() -> (PgPool, PgPool, String) {
    let url = std::env::var("DATABASE_URL").expect("PostgreSQL required for discovery tests");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let name = format!("discovery_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut database_url = url::Url::parse(&url).unwrap();
    database_url.set_path(&name);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(database_url.as_str())
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, admin, name)
}

async fn cleanup(pool: PgPool, admin: PgPool, name: String) {
    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn discovery_uses_evidence_deduplicates_pending_and_revalidates_acknowledgements() {
    let (pool, admin, name) = database().await;
    let root = tempfile::tempdir().unwrap();
    let imports = root.path().join("imports");
    std::fs::create_dir(&imports).unwrap();
    std::fs::write(imports.join("current.txt"), b"current synthetic file").unwrap();
    std::fs::write(imports.join("previous.txt"), b"previous synthetic file").unwrap();

    let current_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
    sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,discovery_enabled,publication_key,format_profile,period_start,period_end,revalidation_seconds) VALUES('current','COTAHIST_YEARLY','local','current.txt',true,true,'current','cotahist_latin1',$1,$1,60),('previous','COTAHIST_YEARLY','local','previous.txt',true,true,'previous','cotahist_latin1','2025-01-01','2025-12-31',60)")
        .bind(current_date)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'COTAHIST_YEARLY',channel,'acquisition','eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['current','previous']) AS channel")
        .execute(&pool)
        .await
        .unwrap();
    let worker_id = Uuid::new_v4();
    sqlx::query("INSERT INTO b3_worker_heartbeats(worker_id,lane) VALUES($1,'maintenance')")
        .bind(worker_id)
        .execute(&pool)
        .await
        .unwrap();
    let service = DiscoveryService::new(
        pool.clone(),
        RuntimeSettings {
            raw_root: PathBuf::from(root.path()),
            allowed_hosts: vec![],
            batch_records: 100,
            batch_bytes: 4096,
            lease_seconds: 120,
        },
    );
    let now = Utc.with_ymd_and_hms(2026, 11, 6, 15, 0, 0).unwrap();

    service.scan(worker_id, true, now).await.unwrap();
    let current_outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM b3_discovery_checks WHERE channel_id='current' AND scheduled ORDER BY checked_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let previous_outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM b3_discovery_checks WHERE channel_id='previous' AND scheduled ORDER BY checked_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current_outcome, "calendar_unknown");
    assert_eq!(previous_outcome, "available");
    let pending_before_manual: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_pending_updates WHERE status='pending'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pending_before_manual, 1);
    let calendar_evidence = Uuid::new_v4();
    sqlx::query("INSERT INTO b3_calendar_evidence(id,source_reference,sha256) VALUES($1,'SYNTHETIC TEST ONLY',$2)")
        .bind(calendar_evidence)
        .bind("b".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO b3_trading_calendar(calendar_date,is_trading_day,evidence_id,session_close) VALUES($1,true,$2,'17:00')")
        .bind(current_date)
        .bind(calendar_evidence)
        .execute(&pool)
        .await
        .unwrap();
    service.scan(worker_id, true, now).await.unwrap();
    let current_after_evidence: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM b3_discovery_checks WHERE channel_id='current' AND scheduled AND outcome='available')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(current_after_evidence);
    let scheduled_health: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT last_discovery_at FROM b3_worker_heartbeats WHERE worker_id=$1")
            .bind(worker_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    service.scan(worker_id, false, now).await.unwrap();
    let manual_health: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT last_discovery_at FROM b3_worker_heartbeats WHERE worker_id=$1")
            .bind(worker_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(manual_health, scheduled_health);
    let pending_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_pending_updates WHERE status='pending'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pending_count, 2);

    let pending: String = sqlx::query_scalar(
        "SELECT id FROM b3_pending_updates WHERE channel_id='current' AND status='pending'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let file = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO b3_source_files(id,source_file_type,sha256,acquisition_channel,status,ingested_at) VALUES($1,'COTAHIST_YEARLY',$2,'current','ingested',NOW())")
        .bind(&file)
        .bind("a".repeat(64))
        .execute(&pool)
        .await
        .unwrap();
    service
        .acknowledge(
            Uuid::parse_str(&pending).unwrap(),
            Uuid::parse_str(&file).unwrap(),
            "Revisado contra o arquivo publicado de teste.",
        )
        .await
        .unwrap();
    sqlx::query("UPDATE b3_channel_baselines SET acknowledged_at=NOW()-INTERVAL '61 seconds' WHERE channel_id='current'")
        .execute(&pool)
        .await
        .unwrap();
    service.scan(worker_id, false, now).await.unwrap();
    let current_pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM b3_pending_updates WHERE channel_id='current' AND status='pending'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current_pending, 1);
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0, "discovery must not enqueue ingestion work");
    cleanup(pool, admin, name).await;
}
