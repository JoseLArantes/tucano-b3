use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use tucano_b3::config::B3Settings;
use tucano_b3::db::enums::{ResolutionStatus, SourceFileType, SyncRunStatus};
use tucano_b3::db::locks::try_acquire_pg_advisory_lock;
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
async fn test_db_migration_and_advisory_lock() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping integration test");
            return;
        }
    };

    let locked = try_acquire_pg_advisory_lock(&pool, "test:lock:key")
        .await
        .expect("Advisory lock query failed");
    assert!(locked, "Advisory lock should be acquired");
}

#[tokio::test]
async fn test_sync_bvbg_028_idempotency_and_audit() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping integration test");
            return;
        }
    };

    let date = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();

    // Clean up test data for this date
    let _ = sqlx::query("DELETE FROM b3_source_files WHERE reference_date = $1")
        .bind(date)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM b3_instruments WHERE security_id = 'PETR4'")
        .execute(&pool)
        .await;

    let settings = B3Settings::default();
    let sync_service = B3SyncService::new(pool.clone(), settings);

    let sample_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<BizData>
    <Instrm>
        <TckrSymb>PETR4</TckrSymb>
        <ISIN>BRPETRACNPR6</ISIN>
        <Sgmt>EQUITY</Sgmt>
        <Desc>PETROBRAS PN</Desc>
        <SpcfctnCd>PN</SpcfctnCd>
        <CrpnNm>PETROLEO BRASILEIRO S.A.</CrpnNm>
        <TradgStartDt>2000-01-01</TradgStartDt>
    </Instrm>
</BizData>"#;

    // 1st sync: Insert
    let res1 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(sample_xml.as_bytes().to_vec()),
            true,
            true,
            Some("test_suite".to_string()),
        )
        .await
        .expect("First sync failed");

    assert_eq!(res1.status, SyncRunStatus::Success);
    assert_eq!(res1.inserted, 1);

    // Verify instrument in database
    let row = sqlx::query(
        "SELECT security_id, asset_description FROM b3_instruments WHERE security_id = 'PETR4'",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch instrument");
    assert_eq!(row.get::<String, _>("security_id"), "PETR4");

    let ident = sqlx::query(
        "SELECT identifier_value FROM b3_instrument_identifiers WHERE identifier_value = 'PETR4'",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch identifier");
    assert_eq!(ident.get::<String, _>("identifier_value"), "PETR4");

    // 2nd sync: Duplicate, should be unchanged
    let res2 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(sample_xml.as_bytes().to_vec()),
            true,
            true,
            Some("test_suite".to_string()),
        )
        .await
        .expect("Second sync failed");

    assert_eq!(res2.status, SyncRunStatus::Success);
    assert_eq!(res2.unchanged, 1);
    assert_eq!(res2.inserted, 0);
    assert_eq!(res2.changed, 0);

    // 3rd sync: Modify description to trigger update and field audit
    let sample_xml_modified = r#"<?xml version="1.0" encoding="utf-8"?>
<BizData>
    <Instrm>
        <TckrSymb>PETR4</TckrSymb>
        <ISIN>BRPETRACNPR6</ISIN>
        <Sgmt>EQUITY</Sgmt>
        <Desc>PETROBRAS PN UPDATED</Desc>
        <SpcfctnCd>PN</SpcfctnCd>
        <CrpnNm>PETROLEO BRASILEIRO S.A.</CrpnNm>
        <TradgStartDt>2000-01-01</TradgStartDt>
    </Instrm>
</BizData>"#;

    let res3 = sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(sample_xml_modified.as_bytes().to_vec()),
            true,
            true,
            Some("test_suite".to_string()),
        )
        .await
        .expect("Third sync failed");

    assert_eq!(res3.status, SyncRunStatus::Success);
    assert_eq!(res3.changed, 1);

    // Verify audit log in b3_field_changes
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM b3_field_changes WHERE table_name = 'b3_instruments' AND field_name = 'asset_description'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query audit changes");

    assert!(
        audit_count >= 1,
        "Field change audit log should record changed description"
    );
}

#[tokio::test]
async fn test_sync_cotahist_and_resolution() {
    let pool = match get_test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("Postgres not reachable, skipping integration test");
            return;
        }
    };

    let date = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();

    // Clean up test data for this date
    let _ = sqlx::query("DELETE FROM b3_cotahist_quotes WHERE trade_date = $1")
        .bind(date)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM b3_source_files WHERE reference_date = $1")
        .bind(date)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM b3_instruments WHERE security_id = 'VALE3'")
        .execute(&pool)
        .await;

    let settings = B3Settings::default();
    let sync_service = B3SyncService::new(pool.clone(), settings);

    // First ensure instrument is ingested
    let sample_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<BizData>
    <Instrm>
        <TckrSymb>VALE3</TckrSymb>
        <ISIN>BRVALEACNOR0</ISIN>
        <Sgmt>EQUITY</Sgmt>
        <Desc>VALE ON</Desc>
    </Instrm>
</BizData>"#;

    sync_service
        .run_manual_sync(
            SourceFileType::Bvbg028InstrumentsFile,
            date,
            date,
            None,
            Some(sample_xml.as_bytes().to_vec()),
            true,
            true,
            Some("test_suite".to_string()),
        )
        .await
        .expect("Instrument sync failed");

    let fit = |val: &str, width: usize| -> String { format!("{:<width$}", val, width = width) };
    let fit_num =
        |val: &str, width: usize| -> String { format!("{:0>width$}", val, width = width) };

    let header = format!("00{: <243}\n", "");
    let parts = vec![
        "01".to_string(),
        "20260202".to_string(),
        "02".to_string(),
        fit("VALE3", 12),
        "010".to_string(),
        fit("VALE", 12),
        fit("ON", 10),
        fit_num("0", 3),
        fit("BRL", 4),
        fit_num("6000", 13),
        fit_num("6150", 13),
        fit_num("5950", 13),
        fit_num("6050", 13),
        fit_num("6100", 13),
        fit_num("6090", 13),
        fit_num("6110", 13),
        fit_num("123", 5),
        fit_num("10000000", 18),
        fit_num("61000000000", 18),
        fit_num("0", 13),
        "0".to_string(),
        "99991231".to_string(),
        fit_num("1", 7),
        fit_num("0", 13),
        fit("BRVALEACNOR0", 12),
        fit_num("123", 3),
    ];
    let record_line = format!("{}\n", parts.join(""));
    let trailer = format!("{:<245}\n", format!("99{: <29}{:0>11}", "", 1));
    let file_content = format!("{}{}{}", header, record_line, trailer);

    let res = sync_service
        .run_manual_sync(
            SourceFileType::CotahistDaily,
            date,
            date,
            None,
            Some(file_content.into_bytes()),
            true,
            true,
            Some("test_suite".to_string()),
        )
        .await
        .expect("Cotahist sync failed");

    assert_eq!(res.status, SyncRunStatus::Success);
    assert_eq!(res.inserted, 1);

    // Verify quote in database
    let row = sqlx::query(
        "SELECT ticker_symbol, resolution_status, instrument_id FROM b3_cotahist_quotes WHERE ticker_symbol = 'VALE3'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query cotahist quote");

    let status: String = row.get("resolution_status");
    assert_eq!(status, ResolutionStatus::Resolved.as_str());
    let inst_id: Option<String> = row.get("instrument_id");
    assert!(
        inst_id.is_some(),
        "Instrument ID should be resolved to VALE3"
    );
}
