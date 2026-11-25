use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};

fn authenticated_app() -> crate::support::TestApp {
    let mut config = tucano_b3::config::AppConfig::default();
    config.auth.token = Some("proventos-test-token".to_string());
    crate::support::app_with_config(config)
}

fn request(method: Method, uri: &str, body: Value, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if authenticated {
        builder = builder.header("authorization", "Bearer proventos-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn sample_provento() -> Value {
    json!({
        "ticker": "VALE3",
        "tipo": "Rendimento",
        "valor": "1.234567",
        "data_com": "2026-10-01",
        "data_pagamento": "2026-11-30",
        "obs": "Fonte de teste"
    })
}

fn csv_upload(csv: &str, authorized: bool) -> Request<Body> {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/proventos/upload")
        .header(
            "content-type",
            "multipart/form-data; boundary=proventos-test",
        );
    if authorized {
        request = request.header("authorization", "Bearer proventos-test-token");
    }
    request
        .body(Body::from(format!(
            "--proventos-test\r\nContent-Disposition: form-data; name=\"file\"; filename=\"proventos.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--proventos-test--\r\n"
        )))
        .unwrap()
}

#[tokio::test]
async fn csv_upload_imports_exact_values_and_skips_existing_and_in_file_duplicates() {
    let app = authenticated_app();
    let csv = concat!(
        "\u{feff}tipo,ticker,valor,data_com,data_pagamento,obs\r\n",
        "Rendimento, tstu11 ,0.123456,2026-06-10,2026-07-01,\"Fundo, comunicado público\"\r\n",
        "Rendimento,TSTU11,0.123456,2026-06-10,2026-07-01,\"Fundo, comunicado público\"\r\n",
        "JCP,TSTU11,0.000001,2026-06-10,,\r\n",
        "Dividendo,TSTU11,1,2026-06-10,,\r\n",
    );
    for expected in [
        json!({"recebidos": 4, "inseridos": 3, "atualizados": 0, "ignorados": 1}),
        json!({"recebidos": 4, "inseridos": 0, "atualizados": 0, "ignorados": 4}),
    ] {
        let response = app.oneshot(csv_upload(csv, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["data"], expected);
    }
    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos/yearly?ticker=TSTU11&ano=2026",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_body(response).await["data"]["proventos_on_year"],
        "1.123457"
    );

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?ticker=TSTU11&tipo=Rendimento",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(
        json_body(response).await["data"]["proventos"][0]["obs"],
        "Fundo, comunicado público"
    );
}

#[tokio::test]
async fn create_and_import_share_identity_and_update_only_observation() {
    let app = authenticated_app();
    let uri = "/api/v1/proventos";
    let draft = json!({"ticker":"TEST3","tipo":"JCP","valor":"1.00","obs":"original"});
    let response = app
        .oneshot(request(Method::POST, uri, draft.clone(), true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let original = json_body(response).await["data"].clone();
    let response = app
        .oneshot(request(Method::POST, uri, draft, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await["data"], original);

    let csv = "ticker,tipo,valor,obs\nTEST3,JCP,1,intermediaria\ntest3,JCP,1.000000,final\n";
    for updates in [1, 0] {
        let response = app.oneshot(csv_upload(csv, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json_body(response).await["data"],
            json!({
                "recebidos": 2, "inseridos": 0, "atualizados": updates, "ignorados": 2 - updates
            })
        );
    }
    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?ticker=TEST3",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    let data = json_body(response).await["data"].clone();
    assert_eq!(data["paginacao"]["total"], 1);
    assert_eq!(data["proventos"][0]["id"], original["id"]);
    assert_eq!(data["proventos"][0]["criado_em"], original["criado_em"]);
    assert_eq!(data["proventos"][0]["obs"], "final");
}

#[tokio::test]
async fn invalid_csv_upload_is_atomic_and_uploads_require_auth() {
    let app = authenticated_app();
    let invalid = concat!(
        "ticker,tipo,valor\n",
        "ABCD3,Dividendo,1.00\n",
        "ABCD4,JCP,0.1234567\n",
    );
    let response = app.oneshot(csv_upload(invalid, true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = json_body(response).await;
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("linha 3"));

    let missing_headers = "ticker,tipo\nABCD3,Dividendo\n";
    let response = app
        .oneshot(csv_upload(missing_headers, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let valid = "ticker,tipo,valor\nABCD3,Dividendo,1\n";
    let response = app.oneshot(csv_upload(valid, false)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = crate::support::empty_app()
        .oneshot(csv_upload(valid, false))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let response = app.oneshot(csv_upload(valid, true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await["data"]["inseridos"], 1);

    let oversized = "x".repeat(1024 * 1024 + 1);
    let response = app.oneshot(csv_upload(&oversized, true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "PAYLOAD_TOO_LARGE"
    );
}

#[tokio::test]
async fn reports_use_data_com_and_sum_all_types_with_exact_precision() {
    let app = authenticated_app();
    for (data_com, valor) in [
        (Some("2026-01-01"), "0.000001"),
        (Some("2026-08-14"), "0.000002"),
        (Some("2026-12-31"), "0.000003"),
        (Some("2025-12-31"), "2.000000"),
        (None, "3.000000"),
    ] {
        let response = app
            .oneshot(request(
                Method::POST,
                "/api/v1/proventos",
                json!({
                    "ticker": "TAEE3",
                    "tipo": "Rendimento",
                    "valor": valor,
                    "data_com": data_com,
                    "data_pagamento": if data_com.is_some() { "2027-01-01" } else { "2026-08-14" }
                }),
                true,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    for (uri, expected) in [
        (
            "/api/v1/proventos/yearly?ticker=taee3&ano=2026",
            json!({"ticker": "TAEE3", "ano": 2026, "proventos_on_year": "0.200106"}),
        ),
        (
            "/api/v1/proventos/yearly?ticker=TAEE3&ano=2024",
            json!({"ticker": "TAEE3", "ano": 2024, "proventos_on_year": "0.000000"}),
        ),
        (
            "/api/v1/proventos/period?tickers=%20taee3%20,PETR4,TAEE3,ZZZZ99&desde=2026-08-14&ate=2026-08-21",
            json!([
                {"ticker": "TAEE3", "proventos_on_period": "0.200102"},
                {"ticker": "PETR4", "proventos_on_period": "0.674100"},
                {"ticker": "ZZZZ99", "proventos_on_period": "0.000000"}
            ]),
        ),
        (
            "/api/v1/proventos/period?ticker=TAEE3&desde=2026-08-14&ate=2026-08-14",
            json!([{"ticker": "TAEE3", "proventos_on_period": "0.200102"}]),
        ),
        (
            "/api/v1/proventos/series?tickers=ZZZZ99,TAEE3,PETR4&desde=2026-08-14&ate=2026-12-31",
            json!([
                {"ticker": "ZZZZ99", "series": []},
                {"ticker": "TAEE3", "series": [
                    {"date_ms": report_date_ms(8, 14), "valor": "0.200102"},
                    {"date_ms": report_date_ms(12, 31), "valor": "0.000003"}
                ]},
                {"ticker": "PETR4", "series": [
                    {"date_ms": report_date_ms(8, 21), "valor": "0.674100"}
                ]}
            ]),
        ),
    ] {
        let response = app
            .oneshot(request(Method::GET, uri, json!({}), true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert_eq!(json_body(response).await["data"], expected, "{uri}");
    }
}

fn report_date_ms(month: u32, day: u32) -> i64 {
    chrono::NaiveDate::from_ymd_opt(2026, month, day)
        .unwrap()
        .and_time(chrono::NaiveTime::MIN)
        .and_utc()
        .timestamp_millis()
}

#[tokio::test]
async fn reports_validate_query_parameters_and_require_credentials_when_configured() {
    let app = authenticated_app();
    for uri in [
        "/api/v1/proventos/yearly?ticker=TAEE3",
        "/api/v1/proventos/yearly?ticker=TAEE3&ano=10000",
        "/api/v1/proventos/period?desde=2026-08-01&ate=2026-08-31",
        "/api/v1/proventos/period?ticker=TAEE3&tickers=PETR4&desde=2026-08-01&ate=2026-08-31",
        "/api/v1/proventos/series?ticker=TAEE3&desde=2026-08-31&ate=2026-08-01",
        "/api/v1/proventos/series?ticker=TAEE3&desde=2026-02-30&ate=2026-08-01",
        "/api/v1/proventos/series?ticker=TAEE3&desde=2026-08-01",
        "/api/v1/proventos/series?tickers=TAEE3,,PETR4&desde=2026-08-01&ate=2026-08-31",
    ] {
        let response = app
            .oneshot(request(Method::GET, uri, json!({}), true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(json_body(response).await["error"]["code"], "INVALID_PARAM");
    }
    for route in ["yearly", "period", "series"] {
        let response = app
            .oneshot(request(
                Method::GET,
                &format!("/api/v1/proventos/{route}"),
                json!({}),
                false,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn seeded_proventos_are_queryable_with_exact_values_and_filters() {
    let app = authenticated_app();
    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?pagina=1&por_pagina=2",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["paginacao"]["total"], 34);
    assert_eq!(body["data"]["paginacao"]["pagina"], 1);
    assert_eq!(body["data"]["paginacao"]["por_pagina"], 2);
    assert_eq!(body["data"]["proventos"].as_array().unwrap().len(), 2);
    assert_eq!(body["data"]["proventos"][0]["ticker"], "BBDC3");
    assert_eq!(body["data"]["proventos"][0]["valor"], "0.017200");
    assert!(body["data"]["proventos"][0]["obs"]
        .as_str()
        .unwrap()
        .contains("Carga inicial"));

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?ticker=taee3&tipo=JCP&data_pagamento_desde=2026-11-26&data_pagamento_ate=2026-11-26",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["data"]["paginacao"]["total"], 1);
    assert_eq!(body["data"]["proventos"][0]["ticker"], "TAEE3");
    assert_eq!(body["data"]["proventos"][0]["tipo"], "JCP");
    assert_eq!(body["data"]["proventos"][0]["valor"], "0.119800");

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?tickers=TAEE3%2CPETR4&data_pagamento_desde=2026-11-23&data_pagamento_ate=2026-11-30",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["data"]["paginacao"]["total"], 3);
    let matched_tickers = body["data"]["proventos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|provento| provento["ticker"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(matched_tickers, ["PETR4", "TAEE3"].into_iter().collect());

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?ticker=RENT4",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert!(body["data"]["proventos"][0]["obs"]
        .as_str()
        .unwrap()
        .contains("ticker conforme relação recebida"));
}

#[tokio::test]
async fn proventos_support_create_read_replace_and_delete() {
    let app = authenticated_app();
    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/proventos",
            json!({
                "ticker": " vale3 ",
                "tipo": "Rendimento",
                "valor": "1.23",
                "data_com": null,
                "data_pagamento": null,
                "obs": null
            }),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;
    let id = body["data"]["id"].as_i64().unwrap();
    assert_eq!(body["data"]["ticker"], "VALE3");
    assert_eq!(body["data"]["valor"], "1.230000");
    assert_eq!(body["data"]["tipo"], "Rendimento");
    assert!(body["data"]["data_pagamento"].is_null());

    let response = app
        .oneshot(request(
            Method::GET,
            &format!("/api/v1/proventos/{id}"),
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await["data"]["id"], id);

    let response = app
        .oneshot(request(
            Method::PUT,
            &format!("/api/v1/proventos/{id}"),
            sample_provento(),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["valor"], "1.234567");
    assert_eq!(body["data"]["obs"], "Fonte de teste");

    let response = app
        .oneshot(request(
            Method::DELETE,
            &format!("/api/v1/proventos/{id}"),
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_body(response).await["data"],
        json!({"id": id, "removido": true})
    );

    let response = app
        .oneshot(request(
            Method::GET,
            &format!("/api/v1/proventos/{id}"),
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn proventos_reject_invalid_fields_and_protect_mutations() {
    let app = authenticated_app();
    for invalid in [
        json!({"ticker":"PETR4","tipo":"Outro","valor":"0.10","data_com":null,"data_pagamento":null,"obs":null}),
        json!({"ticker":"PETR4","tipo":"JCP","valor":"0.000000","data_com":null,"data_pagamento":null,"obs":null}),
        json!({"ticker":"PETR4","tipo":"JCP","valor":"1.1234567","data_com":null,"data_pagamento":null,"obs":null}),
        json!({"ticker":"PETR4","tipo":"Dividendo","valor":"1.00","data_com":"2026-11-02","data_pagamento":"2026-11-01","obs":null}),
    ] {
        let response = app
            .oneshot(request(Method::POST, "/api/v1/proventos", invalid, true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/proventos?tipo=Outro",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "INVALID_PARAM");

    for uri in [
        "/api/v1/proventos?tickers=PETR4%2C%2CVALE3",
        "/api/v1/proventos?ticker=PETR4&tickers=VALE3",
    ] {
        let response = app
            .oneshot(request(Method::GET, uri, json!({}), true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let too_many_tickers = std::iter::repeat_n("PETR4", 101)
        .collect::<Vec<_>>()
        .join(",");
    let response = app
        .oneshot(request(
            Method::GET,
            &format!("/api/v1/proventos?tickers={too_many_tickers}"),
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(request(
            Method::DELETE,
            "/api/v1/proventos/1",
            json!({}),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let app = crate::support::empty_app();
    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/proventos",
            sample_provento(),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
