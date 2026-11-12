//! Integration tests for COTAHIST endpoints with fixture data.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Create a test app with fixture data.
///
/// If fixtures don't exist, tests are skipped gracefully.
fn test_app_with_fixtures() -> Option<axum::Router> {
    let fixture_dir = std::path::PathBuf::from("tests/fixtures");
    if !fixture_dir.join("processed/cotahist").exists() {
        return None;
    }

    let config = tucano_b3::config::AppConfig {
        data: tucano_b3::config::DataConfig {
            dir: fixture_dir,
            reload_interval_seconds: 0,
        },
        ..Default::default()
    };

    let cache = std::sync::Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader = std::sync::Arc::new(
        tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone()).ok()?,
    );
    let api_keys = std::sync::Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = std::sync::Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));

    let metrics = std::sync::Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();

    Some(tucano_b3::server::build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    ))
}

#[tokio::test]
async fn test_list_tickers_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["data"]["tickers"].is_array());
    assert!(json["data"]["paginacao"]["total"].is_number());
    assert!(json["meta"]["request_id"].is_string());

    let tickers = json["data"]["tickers"].as_array().unwrap();
    assert_eq!(tickers.len(), 3);
}

#[tokio::test]
async fn test_get_ticker_detail_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["ticker"], "VALE3");
    assert_eq!(json["data"]["company_name"], "VALE S.A.");
    assert_eq!(json["data"]["total_trading_days"], 2);
    assert_eq!(json["data"]["latest_price"], 62.30);
}

#[tokio::test]
async fn test_get_quotes_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/quotes?dias=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["ticker"], "VALE3");
    let quotes = json["data"]["quotes"].as_array().unwrap();
    assert_eq!(quotes.len(), 2);
}

#[tokio::test]
async fn test_get_latest_quote_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["ticker"], "VALE3");
    assert_eq!(json["data"]["pre_ult"], 62.30);
    assert!(json["data"]["var_pct"].is_number());
    assert!(json["data"]["var_abs"].is_number());
}

#[tokio::test]
async fn test_get_series_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/series?desde=2026-01-01&ate=2026-01-02")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["ticker"], "VALE3");
    assert_eq!(json["data"]["desde"], "2026-01-01");
    let series = json["data"]["series"].as_array().unwrap();
    assert_eq!(series.len(), 2);
    assert!(series[0]["date_ms"].is_number());
    assert!(series[0]["pre_ult"].is_number());
    assert!(series[0]["vol_tot"].is_number());
    assert!(series[0]["tot_neg"].is_number());
    assert!(series[0]["date_ms"].as_i64().unwrap() < series[1]["date_ms"].as_i64().unwrap());
}

#[tokio::test]
async fn test_get_pre_ult_min_max_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let min_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/pre-ult/min?desde=2026-01-01&ate=2026-01-02")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(min_response.status(), StatusCode::OK);
    let min_body = axum::body::to_bytes(min_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let min_json: serde_json::Value = serde_json::from_slice(&min_body).unwrap();
    assert_eq!(min_json["data"]["ticker"], "VALE3");
    assert!(min_json["data"]["pre_ult"].is_number());
    assert!(min_json["data"]["data"].is_string());

    let max_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/pre-ult/max?desde=2026-01-01&ate=2026-01-02")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(max_response.status(), StatusCode::OK);
    let max_body = axum::body::to_bytes(max_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let max_json: serde_json::Value = serde_json::from_slice(&max_body).unwrap();
    assert_eq!(max_json["data"]["pre_ult"], 62.30);
    assert!(
        min_json["data"]["pre_ult"].as_f64().unwrap()
            <= max_json["data"]["pre_ult"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn test_get_date_quotes_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/dates/2026-01-01/quotes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["date"], "2026-01-01");
    let quotes = json["data"]["quotes"].as_array().unwrap();
    assert_eq!(quotes.len(), 3);
}

#[tokio::test]
async fn test_response_envelope_structure() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify envelope structure
    assert!(json["data"].is_object());
    assert!(json["meta"].is_object());
    assert!(json["meta"]["request_id"].is_string());
    assert!(json["meta"]["timestamp"].is_string());
    assert!(json["meta"]["cached"].is_boolean());
    assert!(json["meta"]["query_time_ms"].is_number());
}

#[tokio::test]
async fn test_error_response_structure() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no fixtures");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify error structure
    assert!(json["error"].is_object());
    assert_eq!(json["error"]["code"], "TICKER_NOT_FOUND");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("não encontrado"));
    assert!(json["error"]["request_id"].is_string());
}
