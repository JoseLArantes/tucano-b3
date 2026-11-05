use chrono::NaiveDate;
use sqlx::PgPool;
use tucano_b3::db::enums::ResolutionStatus;
use tucano_b3::db::resolver::RelationshipResolver;
use tucano_b3::db::{create_pool, run_migrations};

async fn get_test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());

    match create_pool(&url).await {
        Ok(pool) => {
            if run_migrations(&pool).await.is_ok() {
                Some(pool)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

#[tokio::test]
async fn test_temporal_identifier_window_and_priority() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping test");
            return;
        }
    };

    let resolver = RelationshipResolver::new(&pool);

    // Setup dummy source file for foreign key
    let source_file_id = "sf-test-temporal-001";
    let _ = sqlx::query(
        r#"
        INSERT INTO b3_source_files (
            id, source_file_type, source_name, reference_date, sha256,
            byte_size, status, created_at, updated_at
        ) VALUES ($1, 'BVBG_028_INSTRUMENTS_FILE', 'test', '2026-01-01', 'dummyhash1', 100, 'ingested', NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(source_file_id)
    .execute(&pool)
    .await;

    let inst_id = "inst-temporal-test-1";
    let _ = sqlx::query(
        r#"
        INSERT INTO b3_instruments (
            id, security_id, source_file_id, source_row_hash, created_at, updated_at, last_seen_at, data_changed_at
        ) VALUES ($1, 'SEC-TEMP-1', $2, 'hash1', NOW(), NOW(), NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(inst_id)
    .bind(source_file_id)
    .execute(&pool)
    .await;

    // Window 1: Old ticker 'OLDT3', valid from 2026-01-01 to 2026-01-31
    let d_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let d_end = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
    let _ = sqlx::query(
        r#"
        INSERT INTO b3_instrument_identifiers (
            id, instrument_id, identifier_type, identifier_value, valid_from, valid_to,
            source_file_id, source_row_hash, created_at, updated_at, last_seen_at, data_changed_at
        ) VALUES ('ident-w1', $1, 'ticker', 'OLDT3', $2, $3, $4, 'h1', NOW(), NOW(), NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(inst_id)
    .bind(d_start)
    .bind(d_end)
    .bind(source_file_id)
    .execute(&pool)
    .await;

    // Window 2: New ticker 'NEWT3', valid from 2026-02-01 onwards (valid_to is NULL)
    let d_w2_start = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
    let _ = sqlx::query(
        r#"
        INSERT INTO b3_instrument_identifiers (
            id, instrument_id, identifier_type, identifier_value, valid_from, valid_to,
            source_file_id, source_row_hash, created_at, updated_at, last_seen_at, data_changed_at
        ) VALUES ('ident-w2', $1, 'ticker', 'NEWT3', $2, NULL, $3, 'h2', NOW(), NOW(), NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(inst_id)
    .bind(d_w2_start)
    .bind(source_file_id)
    .execute(&pool)
    .await;

    // Test 1: Querying 'OLDT3' on 2026-01-15 resolves to inst_id
    let res_jan = resolver
        .resolve_instrument(
            None,
            Some("OLDT3"),
            None,
            None,
            Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(res_jan.status, ResolutionStatus::Resolved);
    assert_eq!(res_jan.instrument_id, Some(inst_id.to_string()));

    // Test 2: Querying 'OLDT3' on 2026-02-15 is unresolved (window has expired)
    let res_feb_old = resolver
        .resolve_instrument(
            None,
            Some("OLDT3"),
            None,
            None,
            Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(res_feb_old.status, ResolutionStatus::Unresolved);

    // Test 3: Querying 'NEWT3' on 2026-02-15 resolves to inst_id
    let res_feb_new = resolver
        .resolve_instrument(
            None,
            Some("NEWT3"),
            None,
            None,
            Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(res_feb_new.status, ResolutionStatus::Resolved);
    assert_eq!(res_feb_new.instrument_id, Some(inst_id.to_string()));

    // Test 4: Priority 1 - security_id resolves directly regardless of dates
    let res_sec = resolver
        .resolve_instrument(Some("SEC-TEMP-1"), None, None, None, None)
        .await
        .unwrap();
    assert_eq!(res_sec.status, ResolutionStatus::Resolved);
    assert_eq!(res_sec.instrument_id, Some(inst_id.to_string()));
}

#[tokio::test]
async fn test_ambiguous_collision_detection() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping test");
            return;
        }
    };

    let resolver = RelationshipResolver::new(&pool);

    let source_file_id = "sf-test-ambiguous-001";
    let _ = sqlx::query(
        r#"
        INSERT INTO b3_source_files (
            id, source_file_type, source_name, reference_date, sha256,
            byte_size, status, created_at, updated_at
        ) VALUES ($1, 'BVBG_028_INSTRUMENTS_FILE', 'test', '2026-01-01', 'dummyhash2', 100, 'ingested', NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(source_file_id)
    .execute(&pool)
    .await;

    // Create two conflicting instruments with the same ticker identifier in the same time window
    let inst_a = "inst-collision-a";
    let inst_b = "inst-collision-b";

    for (inst, sec) in [(inst_a, "SEC-COL-A"), (inst_b, "SEC-COL-B")] {
        let _ = sqlx::query(
            r#"
            INSERT INTO b3_instruments (
                id, security_id, source_file_id, source_row_hash, created_at, updated_at, last_seen_at, data_changed_at
            ) VALUES ($1, $2, $3, 'hcol', NOW(), NOW(), NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(inst)
        .bind(sec)
        .bind(source_file_id)
        .execute(&pool)
        .await;
    }

    let shared_ticker = "AMBIG4";
    for (ident_id, inst) in [("id-col-1", inst_a), ("id-col-2", inst_b)] {
        let _ = sqlx::query(
            r#"
            INSERT INTO b3_instrument_identifiers (
                id, instrument_id, identifier_type, identifier_value, valid_from, valid_to,
                source_file_id, source_row_hash, created_at, updated_at, last_seen_at, data_changed_at
            ) VALUES ($1, $2, 'ticker', $3, '2026-01-01', NULL, $4, 'hcol', NOW(), NOW(), NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(ident_id)
        .bind(inst)
        .bind(shared_ticker)
        .bind(source_file_id)
        .execute(&pool)
        .await;
    }

    // Resolving by shared_ticker without security_id must result in Ambiguous
    let res = resolver
        .resolve_instrument(
            None,
            Some(shared_ticker),
            None,
            None,
            Some(NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
        )
        .await
        .unwrap();

    assert_eq!(res.status, ResolutionStatus::Ambiguous);
    assert!(
        res.instrument_id.is_none(),
        "Ambiguous resolution must not link to any single instrument"
    );
}
