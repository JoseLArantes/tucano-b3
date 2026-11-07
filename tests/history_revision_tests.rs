use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tucano_b3::{
    db::{
        history::{
            CandidateSpec, HistoryRepository, LocatedQuote, LocatedRejection, MemberOutcome,
        },
        run_migrations,
    },
    parsers::{bvbg_028::Bvbg028Row, cotahist::CotahistRow},
    services::jobs::{ExecutionLane, JobClaim, JobCommand, JobPriority, JobService},
};
use uuid::Uuid;

struct Fixture {
    pool: PgPool,
    admin: PgPool,
    name: String,
}
impl Fixture {
    async fn new() -> Self {
        let url = std::env::var("DATABASE_URL").expect("PostgreSQL DATABASE_URL required");
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let name = format!("history_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {name}"))
            .execute(&admin)
            .await
            .unwrap();
        let mut url = url::Url::parse(&url).unwrap();
        url.set_path(&name);
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(url.as_str())
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT family,'fixture',purpose,'eligible',TRUE,'synthetic test evidence',NOW() FROM unnest(ARRAY['COTAHIST_YEARLY','COTAHIST_DAILY','BVBG_028_INSTRUMENTS_FILE']) family CROSS JOIN unnest(ARRAY['processing','raw_retention','rest','mcp','third_party_delivery']) purpose").execute(&pool).await.unwrap();
        Self { pool, admin, name }
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(&format!("DROP DATABASE {} WITH(FORCE)", self.name))
            .execute(&self.admin)
            .await
            .unwrap();
        self.admin.close().await;
    }
    async fn claim(&self) -> JobClaim {
        let jobs = JobService::new(self.pool.clone());
        let id = Uuid::new_v4();
        jobs.submit(
            &id.to_string(),
            JobCommand::Reparse { source_file_id: id },
            JobPriority::Backfill,
            3,
        )
        .await
        .unwrap();
        jobs.claim(Uuid::new_v4(), ExecutionLane::Heavy, 3600)
            .await
            .unwrap()
            .unwrap()
    }
    async fn file(&self, family: &str) -> (String, String) {
        let file = Uuid::new_v4().to_string();
        let member = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO b3_source_files(id,source_file_type,sha256,acquisition_channel) VALUES($1,$2,repeat('a',64),'fixture')",
        )
        .bind(&file)
        .bind(family)
        .execute(&self.pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO b3_source_file_members(id,source_file_id,member_name,sha256) VALUES($1,$2,'nested/quotes.txt',repeat('b',64))").bind(&member).bind(&file).execute(&self.pool).await.unwrap();
        (file, member)
    }
    async fn candidate(
        &self,
        repo: &HistoryRepository,
        claim: &JobClaim,
        file: String,
        key: &str,
        edition: Option<i64>,
    ) -> Uuid {
        sqlx::query("UPDATE b3_source_files SET period_start=$2,period_end=$2,publication_key=$3,official_revision=$4,replaces_publication=$5 WHERE id=$1")
            .bind(&file)
            .bind(date())
            .bind(key)
            .bind(edition)
            .bind(edition.is_some())
            .execute(&self.pool)
            .await
            .unwrap();
        repo.create_candidate(
            claim,
            CandidateSpec {
                source_file_id: file,
                channel: "fixture".into(),
                expected_members: 1,
                period_start: date(),
                period_end: date(),
                publication_key: key.into(),
                official_revision: edition,
                replaces_publication: edition.is_some(),
            },
        )
        .await
        .unwrap()
    }
}
fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()
}
fn quote(price: i64) -> LocatedQuote {
    LocatedQuote {
        record_index: 2,
        row: CotahistRow {
            trade_date: Some(date()),
            ticker_symbol: Some("EXMP3".into()),
            market_type: Some("010".into()),
            distribution_number: Some("001".into()),
            last_price: Some(Decimal::new(price, 2)),
            currency: Some("BRL".into()),
            ..Default::default()
        },
    }
}
async fn seal(repo: &HistoryRepository, claim: &JobClaim, id: Uuid, member: &str, count: i64) {
    repo.complete_member(
        claim,
        id,
        member,
        &MemberOutcome {
            detail_read: count,
            rejected: 0,
            out_of_scope: 0,
            structural_valid: true,
        },
    )
    .await
    .unwrap();
    repo.seal_candidate(claim, id).await.unwrap();
}

#[tokio::test]
async fn immutable_facts_complete_revision_correction_and_rollback() {
    let f = Fixture::new().await;
    let repo = HistoryRepository::new(f.pool.clone());
    let (file, member) = f.file("COTAHIST_YEARLY").await;
    let claim = f.claim().await;
    let first = f
        .candidate(&repo, &claim, file, "annual:2025", Some(1))
        .await;
    let stats = repo
        .stage_batch(&claim, first, &member, &[quote(1000)])
        .await
        .unwrap();
    assert_eq!(stats.inserted, 1);
    assert!(repo.active_revision().await.unwrap().is_none());
    assert!(repo.publish(&claim, first, false).await.is_err());
    seal(&repo, &claim, first, &member, 1).await;
    repo.publish(&claim, first, false).await.unwrap();
    let before=sqlx::query("SELECT q.fact_id,q.version_id,v.data_changed_at FROM b3_history_revision_quotes q JOIN b3_cotahist_versions v ON v.id=q.version_id WHERE revision_id=$1").bind(first).fetch_one(&f.pool).await.unwrap();
    let fact: Uuid = before.get("fact_id");
    let version: Uuid = before.get("version_id");
    let (file2, member2) = f.file("COTAHIST_YEARLY").await;
    let claim2 = f.claim().await;
    let same = f
        .candidate(&repo, &claim2, file2, "annual:2025", Some(1))
        .await;
    let stats = repo
        .stage_batch(&claim2, same, &member2, &[quote(1000)])
        .await
        .unwrap();
    assert_eq!(stats.unchanged, 1);
    seal(&repo, &claim2, same, &member2, 1).await;
    repo.publish(&claim2, same, false).await.unwrap();
    let observed: Uuid = sqlx::query_scalar(
        "SELECT version_id FROM b3_history_revision_quotes WHERE revision_id=$1 AND fact_id=$2",
    )
    .bind(same)
    .bind(fact)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(version, observed);
    let (file3, member3) = f.file("COTAHIST_YEARLY").await;
    let claim3 = f.claim().await;
    let changed = f
        .candidate(&repo, &claim3, file3, "annual:2025", Some(2))
        .await;
    let stats = repo
        .stage_batch(&claim3, changed, &member3, &[quote(1100)])
        .await
        .unwrap();
    assert_eq!(stats.changed, 1);
    assert_eq!(repo.active_revision().await.unwrap(), Some(same));
    seal(&repo, &claim3, changed, &member3, 1).await;
    repo.publish(&claim3, changed, false).await.unwrap();
    let versions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM b3_cotahist_versions WHERE fact_id=$1")
            .bind(fact)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(versions, 2);
    let changes: Vec<String> =
        sqlx::query_scalar("SELECT field_name FROM b3_field_changes WHERE history_revision_id=$1")
            .bind(changed)
            .fetch_all(&f.pool)
            .await
            .unwrap();
    assert_eq!(changes, ["last_price"]);
    let rollback = f.claim().await;
    repo.rollback(&rollback, first).await.unwrap();
    assert_eq!(repo.active_revision().await.unwrap(), Some(first));
    f.close().await;
}

#[tokio::test]
async fn repeated_fact_in_one_revision_is_counted_as_an_in_scope_duplicate() {
    let f = Fixture::new().await;
    let repo = HistoryRepository::new(f.pool.clone());
    let (file, member) = f.file("COTAHIST_YEARLY").await;
    let claim = f.claim().await;
    let revision = f
        .candidate(&repo, &claim, file, "annual:2025-duplicate", Some(1))
        .await;

    let first = repo
        .stage_batch(&claim, revision, &member, &[quote(1000)])
        .await
        .unwrap();
    assert_eq!(first.inserted, 1);

    let mut repeated_quote = quote(1000);
    repeated_quote.record_index = 3;
    let repeated = repo
        .stage_batch(&claim, revision, &member, &[repeated_quote])
        .await
        .unwrap();
    assert_eq!(repeated.inserted, 0);
    assert_eq!(repeated.changed, 0);
    assert_eq!(repeated.unchanged, 0);
    assert_eq!(repeated.duplicate_in_scope, 1);

    seal(&repo, &claim, revision, &member, 2).await;
    repo.publish(&claim, revision, false).await.unwrap();
    let (quotes, occurrences): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM b3_history_revision_quotes WHERE revision_id=$1),(SELECT count(*) FROM b3_cotahist_occurrences WHERE revision_id=$1)",
    )
    .bind(revision)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(quotes, 1);
    assert_eq!(occurrences, 2);
    let (raw, inserted, duplicates): (Option<i32>, Option<i32>, i64) = sqlx::query_as(
        "SELECT row_count_raw,row_count_inserted,row_count_duplicate FROM b3_source_file_members WHERE id=$1",
    )
    .bind(member)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(raw, Some(2));
    assert_eq!(inserted, Some(1));
    assert_eq!(duplicates, 1);
    f.close().await;
}

#[tokio::test]
async fn conflicts_rejections_and_invalid_structure_never_silently_replace() {
    let f = Fixture::new().await;
    let repo = HistoryRepository::new(f.pool.clone());
    let (file, member) = f.file("COTAHIST_YEARLY").await;
    let claim = f.claim().await;
    let base = f.candidate(&repo, &claim, file, "annual", None).await;
    repo.stage_batch(&claim, base, &member, &[quote(1000)])
        .await
        .unwrap();
    seal(&repo, &claim, base, &member, 1).await;
    repo.publish(&claim, base, false).await.unwrap();
    let (file2, member2) = f.file("COTAHIST_DAILY").await;
    let claim2 = f.claim().await;
    let conflict = f.candidate(&repo, &claim2, file2, "daily", None).await;
    repo.stage_batch(&claim2, conflict, &member2, &[quote(1200)])
        .await
        .unwrap();
    seal(&repo, &claim2, conflict, &member2, 1).await;
    assert!(repo.publish(&claim2, conflict, true).await.is_err());
    assert_eq!(repo.active_revision().await.unwrap(), Some(base));
    let (file3, member3) = f.file("COTAHIST_YEARLY").await;
    let claim3 = f.claim().await;
    let invalid = f.candidate(&repo, &claim3, file3, "invalid", None).await;
    repo.stage_batch(&claim3, invalid, &member3, &[quote(1000)])
        .await
        .unwrap();
    repo.complete_member(
        &claim3,
        invalid,
        &member3,
        &MemberOutcome {
            detail_read: 1,
            rejected: 0,
            out_of_scope: 0,
            structural_valid: false,
        },
    )
    .await
    .unwrap();
    assert!(repo.publish(&claim3, invalid, true).await.is_err());
    let (file4, member4) = f.file("COTAHIST_YEARLY").await;
    let claim4 = f.claim().await;
    let degraded = f.candidate(&repo, &claim4, file4, "annual", None).await;
    repo.stage_batch(&claim4, degraded, &member4, &[quote(1000)])
        .await
        .unwrap();
    repo.quarantine_batch(
        &claim4,
        degraded,
        &member4,
        &[LocatedRejection {
            record_index: 3,
            byte_offset: Some(492),
            raw_payload: "invalid".into(),
            reason_code: "BAD_DECIMAL".into(),
            message: "Decimal inválido".into(),
        }],
    )
    .await
    .unwrap();
    repo.complete_member(
        &claim4,
        degraded,
        &member4,
        &MemberOutcome {
            detail_read: 2,
            rejected: 1,
            out_of_scope: 0,
            structural_valid: true,
        },
    )
    .await
    .unwrap();
    repo.seal_candidate(&claim4, degraded).await.unwrap();
    assert!(repo.publish(&claim4, degraded, false).await.is_err());
    repo.publish(&claim4, degraded, true).await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM b3_quarantine_records WHERE history_revision_id=$1",
    )
    .bind(degraded)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    f.close().await;
}

#[tokio::test]
async fn registry_snapshot_identity_rebuild_preserves_market_versions() {
    let f = Fixture::new().await;
    let repo = HistoryRepository::new(f.pool.clone());
    let (file, member) = f.file("COTAHIST_YEARLY").await;
    let claim = f.claim().await;
    let base = f.candidate(&repo, &claim, file, "annual", None).await;
    repo.stage_batch(&claim, base, &member, &[quote(1000)])
        .await
        .unwrap();
    seal(&repo, &claim, base, &member, 1).await;
    repo.publish(&claim, base, false).await.unwrap();
    let (file2, member2) = f.file("BVBG_028_INSTRUMENTS_FILE").await;
    let claim2 = f.claim().await;
    let snapshot = repo
        .create_registry_snapshot(
            &claim2,
            &file2,
            "fixture",
            date(),
            tucano_b3::db::enums::SourceFileType::Bvbg028InstrumentsFile,
            1,
        )
        .await
        .unwrap();
    repo.stage_registry_batch(
        &claim2,
        snapshot,
        &member2,
        &[(
            2,
            Bvbg028Row {
                security_id: "123456".into(),
                security_source: Some("B3".into()),
                market_code: Some("010".into()),
                ticker_symbol: Some("EXMP3".into()),
                ..Default::default()
            },
        )],
    )
    .await
    .unwrap();
    repo.complete_registry_member(&claim2, snapshot, &member2, true)
        .await
        .unwrap();
    repo.seal_registry_snapshot(&claim2, snapshot)
        .await
        .unwrap();
    repo.publish(&claim2, snapshot, false).await.unwrap();
    let rebuild = f.claim().await;
    repo.rebuild_identity(&rebuild, date(), date())
        .await
        .unwrap();
    assert_eq!(repo.active_revision().await.unwrap(), Some(base));
    let candidate: Uuid = sqlx::query_scalar("SELECT id FROM b3_history_revisions WHERE job_id=$1")
        .bind(rebuild.id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let result=sqlx::query("SELECT b.version_id=b2.version_id same_version,b2.resolution_status FROM b3_history_revision_quotes b JOIN b3_history_revision_quotes b2 ON b.fact_id=b2.fact_id WHERE b.revision_id=$1 AND b2.revision_id=$2").bind(base).bind(candidate).fetch_one(&f.pool).await.unwrap();
    assert!(result.get::<bool, _>("same_version"));
    assert_eq!(result.get::<String, _>("resolution_status"), "resolved");
    f.close().await;
}
