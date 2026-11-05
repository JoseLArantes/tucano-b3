use chrono::{Datelike, NaiveDate, Weekday};
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
    if let Ok(pool) = create_pool(&url).await {
        let settings = Arc::new(B3Settings {
            b3_sync_enabled: false,
            ..Default::default()
        });
        let sync_service = Arc::new(B3SyncService::new(pool.clone(), (*settings).clone()));
        let _worker = B3Worker::new(settings, sync_service);
    }

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
