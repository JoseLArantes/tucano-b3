use chrono::NaiveDate;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tucano_b3::{
    db::{enums::SourceFileType, run_migrations},
    services::jobs::{ExecutionLane, JobCommand, JobCompletion, JobPriority, JobService},
};
use uuid::Uuid;

async fn isolated_database() -> (PgPool, PgPool, String) {
    let url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL required for PostgreSQL integration");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let name = format!("release_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&url).unwrap();
    url.set_path(&name);
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(url.as_str())
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, admin, name)
}

async fn cleanup(pool: PgPool, admin: PgPool, name: String) {
    pool.close().await;
    // Only this test's generated database: drain server-side sessions still closing.
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

fn command() -> JobCommand {
    JobCommand::Ingest {
        source: SourceFileType::CotahistYearly,
        from: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        to: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
    }
}

#[tokio::test]
async fn job_submission_claim_fencing_and_atomic_confirmation() {
    let (pool, admin, name) = isolated_database().await;
    let service = JobService::new(pool.clone());
    let id = service
        .submit("year2025", command(), JobPriority::Backfill, 3)
        .await
        .unwrap();
    assert_eq!(
        service
            .submit("year2025", command(), JobPriority::Backfill, 3)
            .await
            .unwrap(),
        id
    );
    assert!(service
        .submit("another-key", command(), JobPriority::Backfill, 3)
        .await
        .is_err());
    assert!(service
        .submit("year2025", command(), JobPriority::Maintenance, 3)
        .await
        .is_err());
    assert!(service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .is_none());
    let (a, b) = tokio::join!(
        service.claim(Uuid::new_v4(), ExecutionLane::Heavy, 60),
        service.claim(Uuid::new_v4(), ExecutionLane::Heavy, 60)
    );
    let claims: Vec<_> = [a.unwrap(), b.unwrap()].into_iter().flatten().collect();
    assert_eq!(claims.len(), 1);
    let first = &claims[0];
    service.heartbeat(first, 60).await.unwrap();
    sqlx::query(
        "UPDATE b3_jobs SET lease_expires_at=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    let second = service
        .claim(Uuid::new_v4(), ExecutionLane::Heavy, 60)
        .await
        .unwrap()
        .unwrap();
    assert!(second.generation > first.generation);
    assert!(service.begin_confirmation(first).await.is_err());
    assert!(service.heartbeat(first, 60).await.is_err());
    // A job completion rolled back alongside data changes must remain runnable.
    let mut tx = service.begin_confirmation(&second).await.unwrap();
    JobService::complete(&mut tx, &second, JobCompletion::Succeeded)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "running");
    let mut tx = service.begin_confirmation(&second).await.unwrap();
    JobService::complete(&mut tx, &second, JobCompletion::SucceededWithIssues)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(service.begin_confirmation(&second).await.is_err());
    let attempts: Vec<String> = sqlx::query_scalar(
        "SELECT outcome FROM b3_job_attempts WHERE job_id=$1 ORDER BY generation",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(attempts, ["expired", "succeeded_with_issues"]);
    // A completed scope can be explicitly requested again without reusing its old idempotency key.
    assert_ne!(
        service
            .submit("next-observation", command(), JobPriority::Backfill, 1)
            .await
            .unwrap(),
        id
    );
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn cancellation_retry_and_exhaustion_are_durable() {
    let (pool, admin, name) = isolated_database().await;
    let service = JobService::new(pool.clone());
    let id = service
        .submit("cancel-queued", command(), JobPriority::Maintenance, 1)
        .await
        .unwrap();
    assert!(service.request_cancel(id).await.unwrap());
    assert!(service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .is_none());
    let id = service
        .submit("retry", command(), JobPriority::Maintenance, 2)
        .await
        .unwrap();
    let first = service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .unwrap();
    service.fail(&first, "HTTP_UNAVAILABLE", 0).await.unwrap();
    let second = service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.id, id);
    assert!(service.request_cancel(id).await.unwrap());
    assert!(service.begin_confirmation(&second).await.is_err());
    service.fail(&second, "CANCEL_REQUESTED", 0).await.unwrap();
    assert!(service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .is_none());
    let id = service
        .submit("exhaust", command(), JobPriority::Maintenance, 1)
        .await
        .unwrap();
    let claim = service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .unwrap();
    sqlx::query(
        "UPDATE b3_jobs SET lease_expires_at=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(service
        .claim(Uuid::new_v4(), ExecutionLane::Maintenance, 60)
        .await
        .unwrap()
        .is_none());
    assert!(service.begin_confirmation(&claim).await.is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
    cleanup(pool, admin, name).await;
}

#[test]
fn administrative_commands_reject_unknown_fields() {
    assert!(serde_json::from_value::<JobCommand>(serde_json::json!({"operation":"publish_revision", "revision_id":Uuid::new_v4(), "acknowledge_issues":false, "sql":"DELETE"})).is_err());
}

#[tokio::test]
async fn eligibility_is_fail_closed_and_purpose_specific() {
    use tucano_b3::services::eligibility::{require_eligible, SourcePurpose};
    let (pool, admin, name) = isolated_database().await;
    let source = SourceFileType::CotahistYearly;
    assert!(
        require_eligible(&pool, source, "manual", &[SourcePurpose::Rest])
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) VALUES('COTAHIST_YEARLY','manual','rest','eligible',true,'synthetic test evidence only',NOW())").execute(&pool).await.unwrap();
    assert!(
        require_eligible(&pool, source, "manual", &[SourcePurpose::Rest])
            .await
            .is_ok()
    );
    assert!(require_eligible(
        &pool,
        source,
        "manual",
        &[SourcePurpose::Rest, SourcePurpose::ThirdPartyDelivery]
    )
    .await
    .is_err());
    assert!(require_eligible(
        &pool,
        SourceFileType::CotahistDaily,
        "manual",
        &[SourcePurpose::Rest]
    )
    .await
    .is_err());
    assert!(require_eligible(&pool, source, "manual", &[])
        .await
        .is_err());
    assert!(
        require_eligible(&pool, source, "automated", &[SourcePurpose::Rest])
            .await
            .is_err()
    );
    cleanup(pool, admin, name).await;
}
