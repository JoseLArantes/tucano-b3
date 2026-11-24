//! Integration tests for OpenAPI and Swagger UI.

use crate::support::empty_app as test_app;
use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn test_openapi_json_returns_200() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_openapi_json_is_valid() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify structure
    assert_eq!(json["openapi"], "3.0.3");
    assert_eq!(json["info"]["title"], "API dados de mercado da B3");
    assert!(json["paths"].is_object());
    assert!(json["components"]["schemas"].is_object());
}

#[tokio::test]
async fn test_openapi_groups_ingestion_operations_under_one_tag() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let tags = json["tags"].as_array().unwrap();
    assert!(tags
        .iter()
        .any(|tag| tag["name"].as_str() == Some("ingestão")));
    assert!(!tags
        .iter()
        .any(|tag| tag["name"].as_str() == Some("ingest")));

    let paths = json["paths"].as_object().unwrap();
    for (path, method) in [
        ("/ingest/upload", "post"),
        ("/ingest/folder", "post"),
        ("/ingest/bucket", "post"),
        ("/ingest/indices", "post"),
        ("/ingest/indices/{id}", "get"),
        ("/ingest/indices/status", "get"),
    ] {
        assert_eq!(
            paths[path][method]["tags"],
            serde_json::json!(["ingestão"]),
            "unexpected OpenAPI tag for {method} {path}"
        );
    }
}

#[tokio::test]
async fn test_openapi_documents_provento_crud_and_exact_decimal_schema() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let paths = &json["paths"];
    for (path, method) in [
        ("/proventos", "get"),
        ("/proventos", "post"),
        ("/proventos/{id}", "get"),
        ("/proventos/{id}", "put"),
        ("/proventos/{id}", "delete"),
    ] {
        assert_eq!(
            paths[path][method]["tags"],
            serde_json::json!(["proventos"])
        );
        assert!(paths[path][method]["security"].is_array());
    }
    assert_eq!(
        json["components"]["schemas"]["ProventoRequest"]["properties"]["valor"]["type"],
        "string"
    );
    assert_eq!(
        json["components"]["schemas"]["ProventoRequest"]["additionalProperties"],
        false
    );
    assert!(json["components"]["schemas"]["ProventoRequest"]["example"].is_object());
    assert!(json["components"]["schemas"]["ProventoResponse"]["example"].is_object());
    assert!(json["components"]["schemas"]["ProventosListResponse"]["example"].is_object());
    assert!(json["components"]["schemas"]["ProventosListApiResponse"].is_object());

    let list_operation = &paths["/proventos"]["get"];
    assert!(list_operation["description"].as_str().unwrap().len() > 200);
    for parameter in list_operation["parameters"].as_array().unwrap() {
        assert!(parameter["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
    }
    let parameter_names = list_operation["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|parameter| parameter["name"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "ticker",
        "tipo",
        "data_pagamento_desde",
        "data_pagamento_ate",
        "pagina",
        "por_pagina",
    ] {
        assert!(parameter_names.contains(&expected));
    }
    let create = &paths["/proventos"]["post"];
    assert!(create["requestBody"]["content"]["application/json"]["schema"].is_object());
    assert!(create["responses"]["201"].is_object());
    assert!(create["responses"]["400"].is_object());
    assert!(paths["/proventos/{id}"]["put"]["responses"]["404"].is_object());
    assert!(paths["/proventos/{id}"]["delete"]["responses"]["200"].is_object());
}

#[tokio::test]
async fn test_openapi_has_all_endpoints() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let paths = json["paths"].as_object().unwrap();

    // COTAHIST endpoints
    assert!(paths.contains_key("/cotahist/tickers"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/quotes"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/latest"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/series"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/pre-ult/min"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/pre-ult/max"));
    assert!(paths.contains_key("/cotahist/dates/{date}/quotes"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/returns"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/stats"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/drawdown"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/range-position"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/relative"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/gaps"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/rolling"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/spread"));
    assert!(paths.contains_key("/cotahist/dates/{date}/movers"));
    assert!(paths.contains_key("/cotahist/liquidity"));
    assert!(paths.contains_key("/cotahist/compare"));

    assert!(paths.contains_key("/metrics/ibov/history"));
    assert!(paths.contains_key("/metrics/ibov/latest"));
    assert!(paths.contains_key("/metrics/idiv/history"));
    assert!(paths.contains_key("/metrics/ifix/latest"));
    assert!(paths.contains_key("/ingest/indices"));
    assert!(!paths.contains_key("/ibov/history"));

    // System endpoints
    assert!(paths.contains_key("/health"));
    assert!(paths.contains_key("/info"));
}

#[tokio::test]
async fn test_openapi_has_security_scheme() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let security = &json["components"]["securitySchemes"]["ApiKeyAuth"];
    assert_eq!(security["type"], "apiKey");
    assert_eq!(security["name"], "X-API-Key");
    assert_eq!(security["in"], "header");
}

#[tokio::test]
async fn test_openapi_has_error_schema() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["components"]["schemas"]["ErrorResponse"].is_object());
    assert!(json["components"]["schemas"]["ErrorDetail"].is_object());
}

#[tokio::test]
async fn test_swagger_ui_returns_200() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_openapi_descriptions_in_portuguese() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Check that info description is in Portuguese
    let description = json["info"]["description"].as_str().unwrap();
    assert!(description.contains("API REST"));
    assert!(description.contains("B3"));
}
