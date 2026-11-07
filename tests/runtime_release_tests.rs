//! Synthetic end-to-end execution; these fixtures do not homologate official publications.
use chrono::NaiveDate;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::process::Stdio;
use tucano_b3::{
    db::enums::SourceFileType,
    db::history::HistoryRepository,
    db::run_migrations,
    services::{
        jobs::{ExecutionLane, JobCommand, JobCompletion, JobPriority, JobService},
        query::{QueryPurpose, QueryService},
    },
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
    fixture_rows(&[("20250102", price)])
}

fn fixture_rows(rows: &[(&str, &str)]) -> Vec<u8> {
    let mut header = vec![b' '; 245];
    header[..2].copy_from_slice(b"00");
    let details = rows
        .iter()
        .map(|(trade_date, price)| {
            let mut detail = vec![b' '; 245];
            for (start, value) in [
                (0, "01"),
                (2, *trade_date),
                (10, "02"),
                (12, "EXMP3"),
                (24, "010"),
                (52, "BRL"),
                (56, *price),
                (202, "99991231"),
                (210, "0000001"),
                (242, "001"),
            ] {
                detail[start..start + value.len()].copy_from_slice(value.as_bytes());
            }
            detail
        })
        .collect::<Vec<_>>();
    let mut trailer = vec![b' '; 245];
    trailer[..2].copy_from_slice(b"99");
    let record_count = format!("{:011}", rows.len());
    trailer[31..42].copy_from_slice(record_count.as_bytes());
    std::iter::once(header)
        .chain(details)
        .chain(std::iter::once(trailer))
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
async fn two_worker_processes_ingest_once_and_publish_a_clean_candidate_automatically() {
    let (pool, admin, name, url) = database().await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("imports")).unwrap();
    std::fs::write(
        root.path().join("imports/year.txt"),
        fixture_rows(&[("20250102", "0000000001234"), ("20250103", "0000000005678")]),
    )
    .unwrap();
    sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,publication_key,format_profile,period_start,period_end) VALUES('fixture','COTAHIST_YEARLY','local','year.txt',true,'year2025','cotahist_latin1','2025-01-01','2025-12-31')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'COTAHIST_YEARLY','fixture',purpose,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['acquisition','raw_retention','processing']) AS purpose").execute(&pool).await.unwrap();
    let jobs = JobService::new(pool.clone());
    let id = jobs
        .submit(
            "ingest",
            JobCommand::Ingest {
                source: SourceFileType::CotahistYearly,
                from: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
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
    let revision: String = sqlx::query_scalar(
        "SELECT output_id FROM b3_job_outputs WHERE job_id=$1 AND output_kind='revision'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let revision = Uuid::parse_str(&revision).unwrap();
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    assert!(QueryService::new(pool.clone())
        .resolve_revision(None, QueryPurpose::Rest)
        .await
        .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_history_revision_quotes WHERE revision_id=$1")
            .bind(revision)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let (detail_read, out_of_scope): (i64, i64) = sqlx::query_as(
        "SELECT detail_read,out_of_scope FROM b3_history_revision_members WHERE revision_id=$1",
    )
    .bind(revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((detail_read, out_of_scope), (2, 1));
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
    assert_eq!(active, Some(replay_revision));
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
    assert_eq!(active, Some(replay_revision));
    let states: Vec<String> =
        sqlx::query_scalar("SELECT status FROM b3_sync_runs WHERE id=ANY($1)")
            .bind(vec![id.to_string(), replay.to_string(), broken.to_string()])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        states.iter().filter(|s| s.as_str() == "succeeded").count(),
        2
    );
    assert_eq!(states.iter().filter(|s| s.as_str() == "failed").count(), 1);
    // A worker waiting for the publication lock does not hold the job row. A replacement
    // generation can take ownership, and the old owner must fail its post-lock fence check.
    let delayed = jobs
        .submit(
            "delayed-publish",
            JobCommand::RollbackRevision {
                revision_id: revision,
            },
            JobPriority::Maintenance,
            2,
        )
        .await
        .unwrap();
    let first_owner = Uuid::new_v4();
    let first_claim = jobs
        .claim(first_owner, ExecutionLane::Maintenance, 1)
        .await
        .unwrap()
        .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(7324031001)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let history = HistoryRepository::new(pool.clone());
    let stale_claim = first_claim.clone();
    let stale = tokio::spawn(async move { history.rollback(&stale_claim, revision).await });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND NOT granted)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    let second_claim = jobs
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 30)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second_claim.id, delayed);
    assert_eq!(second_claim.generation, first_claim.generation + 1);
    blocker.rollback().await.unwrap();
    let stale_result = stale.await.unwrap();
    assert!(
        stale_result.is_err(),
        "expired generation published a revision"
    );
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(replay_revision));
    let history = HistoryRepository::new(pool.clone());
    history.rollback(&second_claim, revision).await.unwrap();
    let mut tx = jobs.begin_confirmation(&second_claim).await.unwrap();
    JobService::complete(&mut tx, &second_claim, JobCompletion::Succeeded)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let active: Option<Uuid> =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, Some(revision));
    let attempt: i32 = sqlx::query_scalar("SELECT attempt FROM b3_jobs WHERE id=$1")
        .bind(delayed)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempt, 2);
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn a_date_cut_batches_overlapping_files_and_withdrawal_is_reversible() {
    let (pool, admin, name, url) = database().await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("imports")).unwrap();
    let content = fixture_rows(&[("20250102", "0000000001234"), ("20250103", "0000000005678")]);
    std::fs::write(root.path().join("imports/annual.txt"), &content).unwrap();
    std::fs::write(root.path().join("imports/cumulative.txt"), &content).unwrap();
    for (channel, file) in [("annual", "annual.txt"), ("cumulative", "cumulative.txt")] {
        sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,publication_key,format_profile,period_start,period_end) VALUES($1,'COTAHIST_YEARLY','local',$2,true,$1,'cotahist_latin1','2025-01-01','2025-12-31')")
            .bind(channel)
            .bind(file)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'COTAHIST_YEARLY',$1,purpose,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['acquisition','raw_retention','processing','rest','mcp','third_party_delivery']) purpose")
            .bind(channel)
            .execute(&pool)
            .await
            .unwrap();
    }
    let jobs = JobService::new(pool.clone());
    let ingest = jobs
        .submit(
            "window-import",
            JobCommand::Ingest {
                source: SourceFileType::CotahistYearly,
                from: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
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
    let revision: Uuid = sqlx::query_scalar(
        "SELECT output_id::uuid FROM b3_job_outputs WHERE job_id=$1 AND output_kind='revision'",
    )
    .bind(ingest)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (quotes, source_files, expected_members, detail_read, out_of_scope): (i64, i64, i32, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM b3_history_revision_quotes WHERE revision_id=$1),(SELECT count(*) FROM b3_history_revision_sources WHERE revision_id=$1),expected_members,(SELECT coalesce(sum(detail_read),0)::bigint FROM b3_history_revision_members WHERE revision_id=$1),(SELECT coalesce(sum(out_of_scope),0)::bigint FROM b3_history_revision_members WHERE revision_id=$1) FROM b3_history_revisions WHERE id=$1",
    )
    .bind(revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            quotes,
            source_files,
            expected_members,
            detail_read,
            out_of_scope
        ),
        (1, 2, 2, 4, 2)
    );
    let fact_count_before: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_cotahist_facts")
        .fetch_one(&pool)
        .await
        .unwrap();
    let version_count_before: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_cotahist_versions")
        .fetch_one(&pool)
        .await
        .unwrap();

    let withdrawal = jobs
        .submit(
            "withdraw-window",
            JobCommand::WithdrawWindow {
                from: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
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
    let active_after_withdrawal: Uuid =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(active_after_withdrawal, revision);
    let visible_after_withdrawal: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_history_revision_quotes WHERE revision_id=$1")
            .bind(active_after_withdrawal)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(visible_after_withdrawal, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b3_cotahist_facts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        fact_count_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b3_cotahist_versions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        version_count_before
    );

    let reimport = jobs
        .submit(
            "reimport-window",
            JobCommand::Ingest {
                source: SourceFileType::CotahistYearly,
                from: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                to: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
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
    let reimport_revision: Uuid = sqlx::query_scalar(
        "SELECT output_id::uuid FROM b3_job_outputs WHERE job_id=$1 AND output_kind='revision'",
    )
    .bind(reimport)
    .fetch_one(&pool)
    .await
    .unwrap();
    let restored_version: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM b3_history_revision_quotes WHERE revision_id=$1",
    )
    .bind(reimport_revision)
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
    assert_eq!(restored_version, original_version);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b3_cotahist_facts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        fact_count_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b3_cotahist_versions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        version_count_before
    );

    let rollback = jobs
        .submit(
            "restore-window",
            JobCommand::RollbackRevision {
                revision_id: revision,
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
    let active_after_rollback: Uuid =
        sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active_after_rollback, revision);
    let final_statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=ANY($1)")
            .bind(vec![ingest, withdrawal, reimport, rollback])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(final_statuses.iter().all(|status| status == "succeeded"));
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn registry_import_is_rejected_until_records_support_internal_date_cuts() {
    let (pool, admin, name, _) = database().await;
    let jobs = JobService::new(pool.clone());
    let from = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();

    let capabilities = tucano_b3::services::query::QueryService::new(pool.clone())
        .capabilities(Default::default())
        .await
        .unwrap();
    for source in [
        SourceFileType::Bvbg028InstrumentsFile,
        SourceFileType::TradableSecurityList,
    ] {
        let capability = capabilities
            .items
            .iter()
            .find(|item| item.source_file_type == source)
            .unwrap();
        assert!(!capability.implemented);
        assert!(!capability.enabled);
        assert_eq!(
            serde_json::to_value(&capability.status).unwrap(),
            serde_json::json!("deferred")
        );
    }

    for source in [
        SourceFileType::Bvbg028InstrumentsFile,
        SourceFileType::TradableSecurityList,
    ] {
        let error = jobs
            .submit(
                &format!("deferred-{source}"),
                JobCommand::Ingest {
                    source,
                    from,
                    to: from,
                },
                JobPriority::Maintenance,
                1,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("cortes às datas internas"));
    }

    let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, 0);
    cleanup(pool, admin, name).await;
}
