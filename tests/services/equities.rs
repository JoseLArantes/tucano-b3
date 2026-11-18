use chrono::{Datelike, NaiveDate};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tucano_b3::config::DataConfig;
use tucano_b3::data::query_service::QueryService;
use tucano_b3::data::reader::discover_files;
use tucano_b3::data::ticker_registry::TickerRegistry;
use tucano_b3::ingest::cotahist_parser::CotahistRecord;
use tucano_b3::ingest::parquet_writer::write_cotahist_parquet;
use tucano_b3::services::cotahist::CotahistService;

fn record(
    ticker: &str,
    isin: &str,
    date: &str,
    spec: &str,
    market_type: u8,
    raw_close: u64,
    factor: u32,
) -> CotahistRecord {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    let market_code = format!("{market_type:03}");
    CotahistRecord {
        ticker: ticker.to_string(),
        raw_ticker: ticker.to_string(),
        bdi_code: Some("02".to_string()),
        market_code,
        security_specification: Some(spec.to_string()),
        currency_code: Some("BRL".to_string()),
        quotation_factor: Some(factor),
        pre_ult_raw: Some(raw_close),
        isin: Some(isin.to_string()),
        company_name: Some("Fixture issuer".to_string()),
        date,
        year: date.year(),
        market_type: Some(market_type),
        pre_abr: Some(raw_close as f64 / 100.0),
        pre_max: Some(raw_close as f64 / 100.0),
        pre_min: Some(raw_close as f64 / 100.0),
        pre_med: Some(raw_close as f64 / 100.0),
        pre_ult: Some(raw_close as f64 / 100.0),
        pre_ofe_compra: None,
        pre_ofe_venda: None,
        tot_neg: Some(1),
        quat_tot: Some(100),
        vol_tot: Some(100.0),
        turnover_cents: Some(10_000),
        source_line: 2,
    }
}

fn create_registry(path: &Path) -> TickerRegistry {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE tickers (
                ticker TEXT PRIMARY KEY,
                isin TEXT,
                company_name TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                is_active INTEGER NOT NULL
            );",
        )
        .unwrap();
    let current = [
        ("PETR3", "BRPETRACNOR9"),
        ("PETR4", "BRPETRACNPR6"),
        ("SANB11", "BRSANBCDAM13"),
        ("VALE5", "BRVALEACNOR0"),
        ("REUSE3", "BRABEVACNOR1"),
        ("BOVA11", "BRBOVACTF001"),
    ];
    for (ticker, isin) in current {
        connection
            .execute(
                "INSERT INTO tickers VALUES (?1, ?2, 'Fixture issuer', '2023-01-01', '2024-09-30', 1)",
                (ticker, isin),
            )
            .unwrap();
    }
    drop(connection);
    TickerRegistry::load(path).unwrap()
}

async fn fixture() -> (tempfile::TempDir, QueryService, TickerRegistry) {
    let directory = tempdir().unwrap();
    let cotahist_dir = directory.path().join("processed/cotahist");
    let records = vec![
        record("PETR3", "BRPETRACNOR9", "2024-09-30", "ON NM", 10, 4750, 1),
        // Odd-lot market 020 has a deliberately different close and must not leak into the spot quote.
        record("PETR3", "BRPETRACNOR9", "2024-09-30", "ON NM", 20, 9999, 1),
        record("PETR4", "BRPETRACNPR6", "2024-09-30", "PN N2", 10, 3821, 1),
        // FATCOT=1000 converts R$ 1,234.56 per quotation lot into R$ 1.23456 per unit.
        record(
            "SANB11",
            "BRSANBCDAM13",
            "2024-09-30",
            "UNT N2",
            10,
            123456,
            1000,
        ),
        record("BOVA11", "BRBOVACTF001", "2024-09-30", "CI", 10, 13000, 1),
        record("OLD3", "BRVALEACNOR0", "2023-01-02", "ON NM", 10, 6000, 1),
        record("VALE5", "BRVALEACNOR0", "2024-09-30", "ON NM", 10, 6100, 1),
        record("REUSE3", "BRVALEACNOR0", "2023-01-02", "ON NM", 10, 5000, 1),
        record("REUSE3", "BRABEVACNOR1", "2024-09-30", "ON NM", 10, 1500, 1),
    ];
    write_cotahist_parquet(&records, &cotahist_dir).unwrap();
    let config = DataConfig {
        dir: directory.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let discovered = discover_files(&config).unwrap();
    assert_eq!(discovered.cotahist_files.len(), 2);
    assert_eq!(discovered.equity_files.len(), 2);
    let query_service = QueryService::new(discovered).unwrap();
    let registry = create_registry(&directory.path().join("ledger/tucano.db"));
    (directory, query_service, registry)
}

#[tokio::test]
async fn picker_filters_spot_equities_and_uses_current_ticker_isin_pairs() {
    let (_directory, query_service, registry) = fixture().await;
    let (rows, total, coverage) =
        CotahistService::list_equities(&query_service, &registry, None, true, 1, 200)
            .await
            .unwrap();
    let tickers = rows
        .iter()
        .map(|row| row.ticker.as_str())
        .collect::<Vec<_>>();
    assert_eq!(total, 5);
    assert_eq!(tickers, ["PETR3", "PETR4", "REUSE3", "SANB11", "VALE5"]);
    assert!(!tickers.contains(&"BOVA11"));
    assert_eq!(
        coverage.map(|value| (value.from, value.to)),
        Some(("2023-01-02".to_string(), "2024-09-30".to_string()))
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.ticker == "SANB11")
            .unwrap()
            .security_specification,
        "UNT N2"
    );

    let (all_rows, all_total, _) =
        CotahistService::list_equities(&query_service, &registry, Some("OLD"), false, 1, 200)
            .await
            .unwrap();
    assert_eq!(all_total, 1);
    assert_eq!(all_rows[0].ticker, "OLD3");

    let (reused, reused_total, _) =
        CotahistService::list_equities(&query_service, &registry, Some("REUSE3"), false, 1, 200)
            .await
            .unwrap();
    assert_eq!(reused_total, 2);
    assert_eq!(reused.iter().filter(|row| row.is_active).count(), 1);
    assert_eq!(
        reused.iter().find(|row| row.is_active).unwrap().isin,
        "BRABEVACNOR1"
    );
}

#[tokio::test]
async fn petra_shares_and_unit_quotes_use_spot_closes_and_fatcot() {
    let (_directory, query_service, _registry) = fixture().await;
    let petr3 = CotahistService::equity_quotes_by_isin(
        &query_service,
        "BRPETRACNOR9",
        "2024-09-30",
        "2024-09-30",
        20,
    )
    .await
    .unwrap();
    assert_eq!(petr3.len(), 1);
    assert_eq!(petr3[0].ticker, "PETR3");
    assert_eq!(petr3[0].close_per_share, "47.5");
    assert_eq!(petr3[0].market_code, "010");
    assert!(!petr3[0].adjusted_for_corporate_actions);

    let petr4 = CotahistService::latest_equity_quote_by_isin(&query_service, "BRPETRACNPR6")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(petr4.ticker, "PETR4");
    assert_eq!(petr4.close_per_share, "38.21");

    let sanb = CotahistService::latest_equity_quote_by_isin(&query_service, "BRSANBCDAM13")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sanb.ticker, "SANB11");
    assert_eq!(sanb.close_per_share, "1.23456");
    assert_eq!(sanb.quotation_factor, 1000);
}
