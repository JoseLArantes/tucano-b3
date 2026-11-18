use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};

fn authenticated_app() -> crate::support::TestApp {
    let mut config = tucano_b3::config::AppConfig::default();
    config.auth.token = Some("calendar-test-token".to_string());
    crate::support::app_with_config(config)
}

fn request(method: Method, uri: &str, body: Value, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if authenticated {
        builder = builder.header("authorization", "Bearer calendar-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn calendar_body(year: i32, event_date: &str) -> Value {
    json!({
        "ano": year,
        "fontes": ["https://www.b3.com.br/pt_br/"],
        "eventos": [{
            "data": event_date,
            "evento": "Confraternização Universal",
            "situacao": "closed",
            "descricao": "Não haverá sessão de negociação.",
            "pre_abertura": null,
            "inicio_negociacao": null,
            "fim_negociacao": null
        }]
    })
}

#[tokio::test]
async fn seeded_calendar_crud_and_trader_hours_are_available() {
    let app = authenticated_app();

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/calendar/2026",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["ano"], 2026);
    assert_eq!(body["data"]["eventos"].as_array().unwrap().len(), 16);
    assert_eq!(body["data"]["eventos"][3]["data"], "2026-02-18");
    assert_eq!(body["data"]["eventos"][3]["situacao"], "special_hours");
    assert_eq!(body["data"]["eventos"][3]["pre_abertura"], "12:45");
    assert_eq!(body["data"]["eventos"][3]["inicio_negociacao"], "13:00");
    assert_eq!(body["data"]["eventos"][8]["situacao"], "open");

    let response = app
        .oneshot(request(Method::GET, "/api/v1/calendar", json!({}), true))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["data"]["anos"], json!([2026]));

    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/negotiation-hours",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["data"]["fuso_horario"], "America/Sao_Paulo");
    assert_eq!(body["data"]["modalidades"][0]["modalidade"], "Tela");
    assert_eq!(body["data"]["modalidades"][0]["abertura"], "09:00");
    assert_eq!(body["data"]["modalidades"][1]["modalidade"], "RFQ");
    assert_eq!(body["data"]["modalidades"][2]["modalidade"], "Voice");
    assert_eq!(body["data"]["modalidades"][2]["fechamento"], "19:15");

    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/calendar",
            calendar_body(2027, "2027-01-01"),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(request(Method::GET, "/api/v1/calendar", json!({}), true))
        .await
        .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["data"]["anos"], json!([2026, 2027]));

    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/calendar",
            calendar_body(2027, "2027-01-01"),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = app
        .oneshot(request(
            Method::PUT,
            "/api/v1/calendar/2027",
            json!({
                "fontes": ["https://www.b3.com.br/pt_br/"],
                "eventos": [{
                    "data": "2027-01-01",
                    "evento": "Ano Novo",
                    "situacao": "closed",
                    "descricao": "Calendário atualizado.",
                    "pre_abertura": null,
                    "inicio_negociacao": null,
                    "fim_negociacao": null
                }]
            }),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["eventos"][0]["evento"], "Ano Novo");

    let response = app
        .oneshot(request(
            Method::DELETE,
            "/api/v1/calendar/2027",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .oneshot(request(
            Method::GET,
            "/api/v1/calendar/2027",
            json!({}),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn calendar_rejects_invalid_year_dates_and_unconfigured_mutations() {
    let app = authenticated_app();
    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/calendar",
            calendar_body(2027, "2026-12-31"),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "INVALID_PARAM");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/calendar")
                .header("content-type", "application/json")
                .header("authorization", "Bearer calendar-test-token")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "INVALID_PARAM");

    let app = crate::support::empty_app();
    let response = app
        .oneshot(request(
            Method::POST,
            "/api/v1/calendar",
            calendar_body(2027, "2027-01-01"),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn calendar_crud_requires_valid_auth_when_enabled() {
    let app = authenticated_app();
    let response = app
        .oneshot(request(
            Method::DELETE,
            "/api/v1/calendar/2026",
            json!({}),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
