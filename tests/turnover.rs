//! Synthetic COTAHIST observations through the production ingest and REST paths.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::{Datelike, NaiveDate, Weekday};
use serde_json::Value;
use std::io::{Cursor, Write};
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::cache::CacheStore;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::ingest::IngestService;
use tucano_b3::metrics::AppMetrics;
use tucano_b3::middleware::{ApiKeyRegistry, RateLimitState};

const ON: &str = "BRPETRACNOR9";
const PN: &str = "BRPETRACNPR6";
const UNIT: &str = "BRSANBCDAM13";
const ENDPOINT: &str = "/api/v1/cotahist/equities/by-isin";

fn line(day: NaiveDate, ticker: &str, isin: &str, class: &str, cents: u64) -> String {
    format!(
        "01{}02{ticker:<12}010{:<12}{class:<10}   R$  {}00001000000000000000001{cents:018}{}0001000{}{isin}001",
        day.format("%Y%m%d"),
        "SYNTHETIC",
        "0000000001234".repeat(7),
        " ".repeat(22),
        " ".repeat(13),
    )
}

fn archive(member: &str, rows: &[String]) -> Vec<u8> {
    assert!(rows.iter().all(|row| row.len() == 245));
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(member, zip::write::SimpleFileOptions::default())
        .unwrap();
    writeln!(zip, "00COTAHIST.2026{}", " ".repeat(230)).unwrap();
    for row in rows {
        writeln!(zip, "{row}").unwrap();
    }
    writeln!(zip, "99{}", " ".repeat(243)).unwrap();
    zip.finish().unwrap().into_inner()
}

async fn query(app: &axum::Router, uri: &str, auth: bool) -> (StatusCode, Value) {
    let mut req = Request::builder().uri(uri);
    if auth {
        req = req.header("Authorization", "Bearer turnover-test");
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn turnover_keeps_exact_values_gaps_and_identity_across_imports() {
    let data = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default();
    config.data.dir = data.path().to_path_buf();
    config.auth.token = Some("turnover-test".to_string());
    let cache = Arc::new(CacheStore::new(&config.cache));
    let loader = Arc::new(DataLoader::new(&config.data, cache.clone()).unwrap());
    let rate = Arc::new(RateLimitState::new(
        Arc::new(ApiKeyRegistry::empty()),
        config.rate_limit.clone(),
    ));
    let app = tucano_b3::server::build_router(
        &config,
        loader.clone(),
        cache,
        rate,
        Arc::new(AppMetrics::new()),
        std::time::Instant::now(),
    );

    let sessions: Vec<_> = NaiveDate::from_ymd_opt(2026, 1, 5)
        .unwrap()
        .iter_days()
        .filter(|date| !matches!(date.weekday(), Weekday::Sat | Weekday::Sun))
        .take(60)
        .collect();
    let uri = format!(
        "{ENDPOINT}/{ON}/turnover?desde={}&ate={}",
        sessions[0], sessions[59]
    );
    assert_eq!(query(&app, &uri, false).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        query(&app, &uri, true).await.1["data"]["dataset_revision"],
        Value::Null
    );
    assert_eq!(
        query(&app, &format!("{ENDPOINT}/{ON}/turnover"), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        query(
            &app,
            &format!("{ENDPOINT}/{ON}/turnover?desde=2025-01-01&ate=2026-01-03"),
            true
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let mut rows = vec![
        line(sessions[0], "PETR3", ON, "ON", 9_007_199_254_740_993),
        line(sessions[0], "PETR4", PN, "PN", 1_000_001),
        line(sessions[0], "SANB11", UNIT, "UNT", 20_000),
        line(sessions[1], "PETR3", ON, "ON", 0),
        // This synthetic reuse must stay in the PN series.
        line(sessions[2], "PETR3", PN, "PN", 8800),
    ];
    let mut odd_lot = line(sessions[1], "PETR3F", ON, "ON", 123);
    odd_lot.replace_range(10..12, "96");
    odd_lot.replace_range(24..27, "020");
    rows.push(odd_lot);
    let mut blank = line(sessions[2], "PETR3", ON, "ON", 0);
    blank.replace_range(170..188, &" ".repeat(18));
    rows.push(blank);
    for &date in &sessions[3..] {
        rows.push(line(date, "PETR3", ON, "ON", 10_000));
    }

    let ingest = |member, rows: &[String]| {
        IngestService::ingest_zip(archive(member, rows), data.path(), &loader)
    };
    ingest("COTAHIST_A2026.TXT", &rows[..rows.len() - 1]).unwrap();
    loader.reload().await.unwrap();
    let annual = query(&app, &uri, true).await.1["data"].clone();
    assert_eq!(annual["observation_count"], 58);
    assert_eq!(
        annual["observations"][0]["financial_turnover_brl"],
        "90071992547409.93"
    );
    assert_eq!(
        annual["observations"][0]["voltot_raw"],
        "009007199254740993"
    );
    assert_eq!(annual["observations"][0]["source_line"], 2);
    assert_eq!(annual["observations"][1]["financial_turnover_brl"], "0.00");
    assert_eq!(annual["observations"][2]["data"], sessions[3].to_string());

    let folder = tempfile::tempdir().unwrap();
    std::fs::write(
        folder.path().join("COTAHIST_M012026.ZIP"),
        archive("COTAHIST_M012026.TXT", &rows[..3]),
    )
    .unwrap();
    IngestService::ingest_folder(folder.path(), data.path(), &loader).unwrap();
    loader.reload().await.unwrap();
    assert_eq!(query(&app, &uri, true).await.1["data"], annual);

    ingest("COTAHIST_D20260327.TXT", &rows[rows.len() - 2..]).unwrap();
    loader.reload().await.unwrap();
    let daily = query(&app, &uri, true).await.1["data"].clone();
    assert_ne!(daily["dataset_revision"], annual["dataset_revision"]);
    assert_eq!(daily["observation_count"], 59);
    assert_eq!(daily["observations"][58]["data"], sessions[59].to_string());
    assert_eq!(
        daily["observations"][0]["source_member"],
        "COTAHIST_A2026.TXT"
    );
    for (isin, count, expected) in [(PN, 2, "10000.01"), (UNIT, 1, "200.00")] {
        let (_, body) = query(&app, &uri.replace(ON, isin), true).await;
        assert_eq!(body["data"]["observation_count"], count);
        assert_eq!(
            body["data"]["observations"][0]["financial_turnover_brl"],
            expected
        );
    }
    let (_, pn) = query(&app, &uri.replace(ON, PN), true).await;
    assert_eq!(pn["data"]["observations"][1]["ticker"], "PETR3");

    let conflict = ingest(
        "COTAHIST_D20260105.TXT",
        &[line(sessions[0], "PETR3", ON, "ON", 1)],
    );
    assert!(conflict
        .unwrap_err()
        .to_string()
        .contains("VOLTOT conflict"));
    loader.reload().await.unwrap();
    assert_eq!(query(&app, &uri, true).await.1["data"], daily);

    let restarted = DataLoader::new_standalone(&config.data).unwrap();
    let reloaded = restarted.get_query_service().await;
    let restored = tucano_b3::services::cotahist::CotahistService::equity_turnover(
        &reloaded,
        ON,
        &sessions[0].to_string(),
        &sessions[59].to_string(),
    )
    .await
    .unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), daily);

    let (status, spec) = query(&app, "/api/docs/openapi.json", false).await;
    assert_eq!(status, StatusCode::OK);
    let path = &spec["paths"]["/cotahist/equities/by-isin/{isin}/turnover"]["get"];
    assert_eq!(
        path["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/EquityTurnoverApiResponse"
    );
    assert_eq!(
        spec["components"]["schemas"]["EquityTurnoverObservation"]["properties"]
            ["financial_turnover_brl"]["type"],
        "string"
    );
}
