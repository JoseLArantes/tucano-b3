//! Synthetic end-to-end execution; these fixtures do not homologate official publications.
use chrono::NaiveDate;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::process::Stdio;
use tucano_b3::{
    db::enums::SourceFileType,
    db::run_migrations,
    services::jobs::{JobCommand, JobPriority, JobService},
};
use uuid::Uuid;

async fn database() -> (PgPool, PgPool, String, String) {
    let url = std::env::var("DATABASE_URL").expect("PostgreSQL required");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let name = format!("runtime_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&url).unwrap();
    url.set_path(&name);
    let url = url.to_string();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, admin, name, url)
}
async fn cleanup(pool: PgPool, admin: PgPool, name: String) {
    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

fn fixture(price: &str) -> Vec<u8> {
    let mut header = vec![b' '; 245];
    header[..2].copy_from_slice(b"00");
    let mut detail = vec![b' '; 245];
    for (start, value) in [
        (0, "01"),
        (2, "20250102"),
        (10, "02"),
        (12, "EXMP3"),
        (24, "010"),
        (52, "BRL"),
        (56, price),
        (202, "99991231"),
        (210, "0000001"),
        (242, "001"),
    ] {
        detail[start..start + value.len()].copy_from_slice(value.as_bytes());
    }
    let mut trailer = vec![b' '; 245];
    trailer[..2].copy_from_slice(b"99");
    trailer[31..42].copy_from_slice(b"00000000003");
    [header, detail, trailer]
        .into_iter()
        .flat_map(|mut r| {
            r.push(b'\n');
            r
        })
        .collect()
}

async fn worker(url: &str, root: &std::path::Path) -> tokio::process::Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_tucano-b3"))
        .args(["worker", "--once", "--lane", "maintenance"])
        .env("DATABASE_URL", url)
        .env("B3_RAW_DATA_DIR", root)
        .env("B3_DISCOVERY_ENABLED", "false")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[tokio::test]
async fn two_worker_processes_ingest_once_and_publication_is_explicit() {
    let (pool, admin, name, url) = database().await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("imports")).unwrap();
    std::fs::write(
        root.path().join("imports/year.txt"),
        fixture("0000000001234"),
    )
    .unwrap();
    sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,publication_key,format_profile,period_start,period_end) VALUES('fixture','COTAHIST_YEARLY','local','year.txt',true,'year2025','cotahist_latin1','2025-01-01','2025-12-31')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'COTAHIST_YEARLY','fixture',purpose,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['acquisition','raw_retention','processing','rest','mcp','third_party_delivery']) AS purpose").execute(&pool).await.unwrap();
    let jobs = JobService::new(pool.clone());
    let id = jobs
        .submit(
            "ingest",
            JobCommand::Ingest {
                source: SourceFileType::CotahistYearly,
                from: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            },
            JobPriority::Maintenance,
            1,
        )
        .await
        .unwrap();
    let a = worker(&url, root.path()).await;
    let b = worker(&url, root.path()).await;
    let (a, b) = tokio::join!(a.wait_with_output(), b.wait_with_output());
    assert!(a.unwrap().status.success());
    assert!(b.unwrap().status.success());
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_job_attempts WHERE job_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, 1);
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(active.is_none());
    let revision: String = sqlx::query_scalar(
        "SELECT output_id FROM b3_job_outputs WHERE job_id=$1 AND output_kind='revision'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let revision = Uuid::parse_str(&revision).unwrap();
    let publish = jobs
        .submit(
            "publish",
            JobCommand::PublishRevision {
                revision_id: revision,
                acknowledge_issues: false,
            },
            JobPriority::Maintenance,
            1,
        )
        .await
        .unwrap();
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(publish)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_history_revision_quotes WHERE revision_id=$1")
            .bind(revision)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    // Reparse uses immutable publication metadata and bytes, not today's catalog/import.
    let source_file: String = sqlx::query_scalar(
        "SELECT output_id FROM b3_job_outputs WHERE job_id=$1 AND output_kind='source_file'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let original_version: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM b3_history_revision_quotes WHERE revision_id=$1",
    )
    .bind(revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE b3_source_channels SET publication_key='CHANGED_CATALOG',period_start='2026-01-01',period_end='2026-12-31',format_profile='unreviewed' WHERE id='fixture'").execute(&pool).await.unwrap();
    std::fs::write(
        root.path().join("imports/year.txt"),
        b"broken new external file",
    )
    .unwrap();
    let replay = jobs
        .submit(
            "reparse",
            JobCommand::Reparse {
                source_file_id: Uuid::parse_str(&source_file).unwrap(),
            },
            JobPriority::Maintenance,
            1,
        )
        .await
        .unwrap();
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(replay)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    let replay_revision: String = sqlx::query_scalar(
        "SELECT output_id FROM b3_job_outputs WHERE job_id=$1 AND output_kind='revision'",
    )
    .bind(replay)
    .fetch_one(&pool)
    .await
    .unwrap();
    let replay_revision = Uuid::parse_str(&replay_revision).unwrap();
    let replay_version: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM b3_history_revision_quotes WHERE revision_id=$1",
    )
    .bind(replay_revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(original_version, replay_version);
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    // A structurally broken second member cannot replace the public revision.
    let file = std::fs::File::create(root.path().join("imports/mixed.zip")).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    use std::io::Write;
    zip.start_file("first.txt", options).unwrap();
    zip.write_all(&fixture("0000000001234")).unwrap();
    zip.start_file("second.txt", options).unwrap();
    zip.write_all(b"broken structural input").unwrap();
    zip.finish().unwrap();
    sqlx::query("UPDATE b3_source_channels SET publication_key='year2025',period_start='2025-01-01',period_end='2025-12-31',format_profile='cotahist_latin1',local_relative_path='mixed.zip' WHERE id='fixture'").execute(&pool).await.unwrap();
    let broken = jobs
        .submit(
            "broken",
            JobCommand::Ingest {
                source: SourceFileType::CotahistYearly,
                from: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            },
            JobPriority::Maintenance,
            1,
        )
        .await
        .unwrap();
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(broken)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    let states: Vec<String> =
        sqlx::query_scalar("SELECT status FROM b3_sync_runs WHERE id=ANY($1)")
            .bind(vec![
                id.to_string(),
                publish.to_string(),
                replay.to_string(),
                broken.to_string(),
            ])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        states.iter().filter(|s| s.as_str() == "succeeded").count(),
        3
    );
    assert_eq!(states.iter().filter(|s| s.as_str() == "failed").count(), 1);
    // A real process whose lease expires while waiting for publication cannot commit.
    let delayed = jobs
        .submit(
            "delayed-publish",
            JobCommand::PublishRevision {
                revision_id: replay_revision,
                acknowledge_issues: false,
            },
            JobPriority::Maintenance,
            2,
        )
        .await
        .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(7324031001)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let stale = tokio::process::Command::new(env!("CARGO_BIN_EXE_tucano-b3"))
        .args(["worker", "--once", "--lane", "maintenance"])
        .env("DATABASE_URL", &url)
        .env("B3_RAW_DATA_DIR", root.path())
        .env("B3_LEASE_SECONDS", "3")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let started: bool = sqlx::query_scalar("SELECT status='running' FROM b3_jobs WHERE id=$1")
            .bind(delayed)
            .fetch_one(&pool)
            .await
            .unwrap();
        if started {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    blocker.rollback().await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(10), stale.wait_with_output())
            .await
            .unwrap()
            .unwrap()
            .status
            .success()
    );
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(replay_revision));
    let attempt: i32 = sqlx::query_scalar("SELECT attempt FROM b3_jobs WHERE id=$1")
        .bind(delayed)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempt, 2);
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn tradable_security_list_stores_full_typed_records_before_explicit_publication() {
    use tucano_b3::services::query::{InstrumentFilters, QueryPurpose, QueryService};

    let (pool, admin, name, url) = database().await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("imports")).unwrap();
    let payload = r#"[{"Symbol":"EXMP3","SecurityID":"000123","SecurityIDSource":"8","SecurityExchange":"BVMF","ISINNumber":"BREXMPACNOR0","SecurityType":"CS","SecurityDesc":"EMPRESA EXEMPLO","MinPriceIncrement":"0.0100","NoApplIDs":[{"ApplID":"MBO1"}],"NoInstrAttrib":[{"InstrAttribType":"24","InstrAttribValue":"1"}],"NoLegs":[{"LegSymbol":"EXMP4","LegRatioQty":"-1.0000","LegSide":"2"}]}]"#;
    std::fs::write(root.path().join("imports/security.json"), payload).unwrap();
    let reference = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
    sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,publication_key,format_profile,period_start,period_end) VALUES('fixture-tsl','TRADABLE_SECURITY_LIST','local','security.json',true,'tsl-2026-01-02','tradable_b3_security_definition_v101_json',$1,$1)")
        .bind(reference)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'TRADABLE_SECURITY_LIST','fixture-tsl',purpose,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['acquisition','raw_retention','processing','rest','mcp','third_party_delivery']) AS purpose")
        .execute(&pool)
        .await
        .unwrap();
    let jobs = JobService::new(pool.clone());
    let ingest = jobs
        .submit(
            "ingest-tsl",
            JobCommand::Ingest {
                source: SourceFileType::TradableSecurityList,
                from: reference,
                to: reference,
            },
            JobPriority::Maintenance,
            1,
        )
        .await
        .unwrap();
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(ingest)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
    let snapshot: Uuid = sqlx::query_scalar("SELECT id FROM b3_registry_snapshots WHERE job_id=$1")
        .bind(ingest)
        .fetch_one(&pool)
        .await
        .unwrap();
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_security_definition_legs WHERE snapshot_id=$1")
            .bind(snapshot)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, 1);
    let visible_before_publish: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_registry_records WHERE snapshot_id=$1 AND instrument_id IS NOT NULL")
        .bind(snapshot)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(visible_before_publish, 0);

    jobs.submit(
        "publish-tsl",
        JobCommand::PublishRevision {
            revision_id: snapshot,
            acknowledge_issues: false,
        },
        JobPriority::Maintenance,
        1,
    )
    .await
    .unwrap();
    assert!(worker(&url, root.path())
        .await
        .wait_with_output()
        .await
        .unwrap()
        .status
        .success());
    let published = QueryService::new(pool.clone())
        .instruments(
            InstrumentFilters {
                ticker_symbol: Some("EXMP3".into()),
                source_file_type: Some(SourceFileType::TradableSecurityList),
                ..Default::default()
            },
            QueryPurpose::Rest,
        )
        .await
        .unwrap();
    assert_eq!(published.total, 1);
    assert_eq!(published.items[0].security_id.as_deref(), Some("000123"));
    assert_eq!(published.items[0].security_source, "8");
    cleanup(pool, admin, name).await;
}
