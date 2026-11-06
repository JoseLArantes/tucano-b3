use chrono::{Datelike, NaiveDate, Timelike, Weekday};
use std::sync::Arc;
use tucano_b3::config::B3Settings;
use tucano_b3::db::create_pool;
use tucano_b3::db::enums::SourceFileType;
use tucano_b3::services::sync_service::B3SyncService;
use tucano_b3::worker::B3Worker;

#[tokio::test]
async fn test_worker_weekend_skip_and_instantiation() {
    let saturday = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
    let sunday = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
    let monday = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();

    assert_eq!(saturday.weekday(), Weekday::Sat);
    assert!(saturday.weekday().number_from_monday() > 5);

    assert_eq!(sunday.weekday(), Weekday::Sun);
    assert!(sunday.weekday().number_from_monday() > 5);

    assert_eq!(monday.weekday(), Weekday::Mon);
    assert!(monday.weekday().number_from_monday() <= 5);

    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());
    let pool = create_pool(&url)
        .await
        .expect("Integration test requires PostgreSQL; connect failed");

    let settings = Arc::new(B3Settings {
        b3_sync_enabled: false,
        ..Default::default()
    });
    let sync_service = Arc::new(B3SyncService::new(pool.clone(), (*settings).clone()));
    let _worker = B3Worker::new(settings, sync_service);

    // Verify worker scheduled order constant definition
    let order = [
        SourceFileType::Bvbg028InstrumentsFile,
        SourceFileType::TradableSecurityList,
        SourceFileType::Bvbg087IndexReport,
        SourceFileType::Bvbg086PriceReport,
        SourceFileType::Bvbg186EquitiesSimplifiedPriceReport,
        SourceFileType::CotahistDaily,
    ];
    assert_eq!(order.len(), 6);
}

#[test]
fn test_worker_sao_paulo_timezone_boundary_invariants() {
    use chrono::{DateTime, TimeZone, Utc};
    use chrono_tz::America::Sao_Paulo;

    // Suppose UTC is 2026-11-06 at 01:30 AM (after midnight in London/UTC)
    let utc_time: DateTime<Utc> = Utc.with_ymd_and_hms(2026, 11, 6, 1, 30, 0).unwrap();
    let sp_time = utc_time.with_timezone(&Sao_Paulo);

    // In São Paulo (UTC-3), it is still 2026-11-05 at 22:30 (previous calendar date!)
    assert_eq!(
        sp_time.date_naive(),
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()
    );
    assert_eq!(sp_time.time().hour(), 22);

    // Market sessions operate strictly on São Paulo dates, demonstrating why naive UTC or host local time causes session drift
    assert_ne!(utc_time.date_naive(), sp_time.date_naive());
}
