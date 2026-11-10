use std::path::PathBuf;
use tucano_b3::config::DataConfig;
use tucano_b3::data::query_service::QueryService;
use tucano_b3::data::reader::discover_files;
use tucano_b3::data::ticker_registry::TickerRegistry;
use tucano_b3::models::pagination::PaginationParams;
use tucano_b3::services::cotahist::CotahistService;

#[test]
fn test_validate_date_valid() {
    assert!(CotahistService::validate_date("2026-11-10").is_ok());
}

#[test]
fn test_validate_date_invalid() {
    assert!(CotahistService::validate_date("not-a-date").is_err());
}

#[test]
fn test_validate_date_wrong_format() {
    assert!(CotahistService::validate_date("10/11/2026").is_err());
}

#[test]
fn test_pagination_offset_calculation() {
    let params = PaginationParams {
        pagina: 2,
        por_pagina: 50,
    };
    assert_eq!(params.offset(), 50);
    assert_eq!(params.limit(), 50);
}

#[tokio::test]
async fn test_cotahist_service_operations() {
    let fixture_dir = PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        return;
    }

    let config = DataConfig {
        dir: fixture_dir.clone(),
        reload_interval_seconds: 0,
    };

    let discovered = discover_files(&config).expect("failed to discover");
    let query_service = QueryService::new(discovered).expect("failed to build query service");
    let ledger_path = fixture_dir.join("ledger").join("tucano.db");
    let registry = TickerRegistry::load(&ledger_path).expect("failed to load registry");

    // 1. List tickers
    let (tickers, total) =
        CotahistService::list_tickers(&registry, 1, 10, None, true).expect("list_tickers failed");
    assert_eq!(total, 3);
    assert_eq!(tickers.len(), 3);

    // Filter by search prefix
    let (tickers_filtered, total_filtered) =
        CotahistService::list_tickers(&registry, 1, 10, Some("VALE"), true)
            .expect("list_tickers with search failed");
    assert_eq!(total_filtered, 1);
    assert_eq!(tickers_filtered[0].ticker, "VALE3");

    // 2. Ticker detail
    let detail = CotahistService::get_ticker_detail(&query_service, &registry, "VALE3")
        .await
        .expect("get_ticker_detail failed");
    assert_eq!(detail.ticker, "VALE3");
    assert_eq!(detail.isin.as_deref(), Some("BRVALEACNOR0"));
    assert_eq!(detail.total_trading_days, Some(2));
    assert!(detail.latest_price.is_some());

    // Unknown ticker detail
    let err = CotahistService::get_ticker_detail(&query_service, &registry, "UNKNOWN")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        tucano_b3::error::AppError::TickerNotFound { .. }
    ));

    // 3. Quotes for ticker
    let params = PaginationParams {
        pagina: 1,
        por_pagina: 10,
    };
    let (quotes, total_quotes) = CotahistService::get_quotes(
        &query_service,
        &registry,
        "VALE3",
        None,
        None,
        None,
        "asc",
        &params,
    )
    .await
    .expect("get_quotes failed");
    assert_eq!(total_quotes, 2);
    assert_eq!(quotes.len(), 2);
    assert_eq!(quotes[0].ticker, "VALE3");

    // 4. Latest quote
    let latest = CotahistService::get_latest_quote(&query_service, &registry, "VALE3")
        .await
        .expect("get_latest_quote failed");
    assert_eq!(latest.ticker, "VALE3");
    assert!(latest.pre_ult.is_some());

    // 5. Quotes by date
    let (date_quotes, total_date) =
        CotahistService::get_quotes_by_date(&query_service, "2026-01-01", &params)
            .await
            .expect("get_quotes_by_date failed");
    assert_eq!(total_date, 3); // VALE3, PETR4, ITUB4 on 2026-01-01
    assert_eq!(date_quotes.len(), 3);
}
