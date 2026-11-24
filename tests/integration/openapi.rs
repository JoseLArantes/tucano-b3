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
        "tickers",
        "tipo",
        "data_pagamento_desde",
        "data_pagamento_ate",
        "pagina",
        "por_pagina",
    ] {
        assert!(parameter_names.contains(&expected));
    }
    let tickers_parameter = list_operation["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|parameter| parameter["name"] == "tickers")
        .unwrap();
    assert_eq!(tickers_parameter["example"], "PETR4,VALE3");
    assert!(tickers_parameter["description"]
        .as_str()
        .unwrap()
        .contains("100"));
    let create = &paths["/proventos"]["post"];
    assert!(create["requestBody"]["content"]["application/json"]["schema"].is_object());
    assert!(create["responses"]["201"].is_object());
    assert!(create["responses"]["400"].is_object());
    assert!(paths["/proventos/{id}"]["put"]["responses"]["404"].is_object());
    assert!(paths["/proventos/{id}"]["delete"]["responses"]["200"].is_object());
}

#[tokio::test]
async fn test_openapi_documents_proventos_reporting() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
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

    for (path, expected_params) in [
        ("/proventos/yearly", &["ticker", "ano"][..]),
        (
            "/proventos/period",
            &["ticker", "tickers", "desde", "ate"][..],
        ),
        (
            "/proventos/series",
            &["ticker", "tickers", "desde", "ate"][..],
        ),
    ] {
        let operation = &json["paths"][path]["get"];
        let params = operation["parameters"].as_array().unwrap();
        for name in expected_params {
            let param = params.iter().find(|param| param["name"] == *name).unwrap();
            assert_eq!(param["in"], "query");
            assert!(param["description"].is_string());
            assert_eq!(
                param["required"],
                path.ends_with("/yearly") || matches!(*name, "desde" | "ate")
            );
        }
        assert_eq!(
            operation["security"],
            serde_json::json!([{"BearerAuth": []}, {"ApiKeyAuth": []}])
        );
        for status in ["200", "400", "401", "429", "500"] {
            assert!(
                operation["responses"][status].is_object(),
                "{path} {status}"
            );
        }
    }
    let schemas = &json["components"]["schemas"];
    for (schema, value_field) in [
        ("ProventosYearlyResponse", "proventos_on_year"),
        ("ProventosPeriodTotal", "proventos_on_period"),
        ("ProventosSeriesPoint", "valor"),
    ] {
        assert_eq!(schemas[schema]["additionalProperties"], false);
        assert_eq!(schemas[schema]["properties"][value_field]["type"], "string");
    }
}

#[tokio::test]
async fn test_openapi_exposes_proventos_csv_as_a_swagger_file_input() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
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
    let operation = &json["paths"]["/proventos/upload"]["post"];
    let form = &operation["requestBody"]["content"]["multipart/form-data"]["schema"];
    assert_eq!(
        form["$ref"],
        "#/components/schemas/ProventosCsvUploadRequest"
    );
    let schema = &json["components"]["schemas"]["ProventosCsvUploadRequest"];
    assert_eq!(schema["required"], serde_json::json!(["file"]));
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["file"]["type"], "string");
    assert_eq!(schema["properties"]["file"]["format"], "binary");
    assert_eq!(
        operation["security"],
        serde_json::json!([{"BearerAuth": []}, {"ApiKeyAuth": []}])
    );
    for status in ["200", "400", "401", "413", "429", "500", "503"] {
        assert!(operation["responses"][status].is_object(), "{status}");
    }
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
