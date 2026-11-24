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
