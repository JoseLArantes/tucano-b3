use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tucano_b3::config::B3Settings;
use tucano_b3::db::enums::{SourceFileType, SyncRunStatus};
use tucano_b3::db::{create_pool, run_migrations};
use tucano_b3::services::sync_service::B3SyncService;

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
async fn test_granular_field_audit_and_idempotency() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping integration test");
            return;
        }
    };

    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    // Clean up test data
    let _ = sqlx::query("DELETE FROM b3_source_files WHERE reference_date = $1")
        .bind(date)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM b3_instruments WHERE security_id = 'MGLU3'")
        .execute(&pool)
        .await;
    let _ = sqlx::query(
        "DELETE FROM b3_field_changes WHERE record_pk = 'MGLU3' OR table_name = 'b3_instruments'",
    )
    .execute(&pool)
    .await;

    let settings = Arc::new(B3Settings::default());
    let sync_service = B3SyncService::new(pool.clone(), (*settings).clone());

    let xml_v1 = r#"<?xml version="1.0" encoding="utf-8"?>
<BizData>
    <Instrm>
        <TckrSymb>MGLU3</TckrSymb>
        <ISIN>BRMGLUACNOR2</ISIN>
        <Sgmt>EQUITY</Sgmt>
        <Desc>MAGAZINE LUIZA ON</Desc>
        <MktDesc>MERCADO A VISTA</MktDesc>
    </Instrm>
</BizData>"#;

    // 1. Initial ingestion -> Insert
    let res1 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(xml_v1.as_bytes().to_vec()),
            true,
            true,
            Some("test_upsert".to_string()),
        )
        .await
        .expect("First sync failed");

    assert_eq!(res1.status, SyncRunStatus::Success);
    assert_eq!(res1.inserted, 1);

    // Initial audit log count for this instrument should be 0 (inserts are not field changes)
    let initial_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM b3_field_changes WHERE table_name = 'b3_instruments' AND field_name = 'asset_description'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count audits");
    assert_eq!(initial_audits, 0);

    // 2. Duplicate ingestion -> Identical row hash, 0 changed, 1 unchanged
    let res2 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(xml_v1.as_bytes().to_vec()),
            true,
            true,
            Some("test_upsert".to_string()),
        )
        .await
        .expect("Second sync failed");

    assert_eq!(res2.status, SyncRunStatus::Success);
    assert_eq!(res2.unchanged, 1);
    assert_eq!(res2.changed, 0);

    // 3. Mutated field ingestion: change description
    let xml_v2 = r#"<?xml version="1.0" encoding="utf-8"?>
<BizData>
    <Instrm>
        <TckrSymb>MGLU3</TckrSymb>
        <ISIN>BRMGLUACNOR2</ISIN>
        <Sgmt>EQUITY</Sgmt>
        <Desc>MAGAZINE LUIZA S.A. ON</Desc>
        <MktDesc>MERCADO A VISTA</MktDesc>
    </Instrm>
</BizData>"#;

    let res3 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(xml_v2.as_bytes().to_vec()),
            true,
            true,
            Some("test_upsert".to_string()),
        )
        .await
        .expect("Third sync failed");

    assert_eq!(res3.status, SyncRunStatus::Success);
    assert_eq!(res3.changed, 1);

    // Verify audit entry in b3_field_changes
    let audit_row = sqlx::query(
        "SELECT table_name, field_name, new_value_text FROM b3_field_changes WHERE table_name = 'b3_instruments' AND field_name = 'asset_description' ORDER BY changed_at DESC LIMIT 1"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch audit change");

    assert_eq!(audit_row.get::<String, _>("table_name"), "b3_instruments");
    assert_eq!(
        audit_row.get::<String, _>("field_name"),
        "asset_description"
    );
    assert_eq!(
        audit_row.get::<Option<String>, _>("new_value_text"),
        Some("MAGAZINE LUIZA S.A. ON".to_string())
    );
}
