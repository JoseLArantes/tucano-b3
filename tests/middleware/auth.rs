use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;
use tucano_b3::config::AuthConfig;
use tucano_b3::middleware::auth::{is_public_path, AuthLayer};

fn test_app(auth_config: AuthConfig) -> Router {
    Router::new()
        .route("/health", get(|| async { "healthy" }))
        .route("/api/v1/health", get(|| async { "healthy" }))
        .route("/redoc", get(|| async { "redoc" }))
        .route("/api/redoc", get(|| async { "redoc" }))
        .route("/api/docs", get(|| async { "swagger" }))
        .route("/api/docs/openapi.json", get(|| async { "{}" }))
        .route("/api/v1/cotahist/tickers", get(|| async { "tickers" }))
        .layer(AuthLayer::new(auth_config))
}

#[test]
fn test_is_public_path() {
    assert!(is_public_path("/health"));
    assert!(is_public_path("/api/v1/health"));
    assert!(is_public_path("/redoc"));
    assert!(is_public_path("/api/redoc"));
    assert!(is_public_path("/api/docs"));
    assert!(is_public_path("/api/docs/openapi.json"));
    assert!(is_public_path("/api/docs/index.html"));

    assert!(!is_public_path("/api/v1/cotahist/tickers"));
    assert!(!is_public_path("/api/v1/cotahist/tickers/VALE3/quotes"));
    assert!(!is_public_path("/api/v1/ibov/history"));
    assert!(!is_public_path("/api/v1/ingest/upload"));
    assert!(!is_public_path("/api/v1/ingest/folder"));
    assert!(!is_public_path("/api/v1/info"));
    assert!(!is_public_path("/metrics"));
}

#[tokio::test]
async fn test_auth_disabled_when_no_token() {
    let app = test_app(AuthConfig::default());

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
}

#[tokio::test]
async fn test_auth_rejects_missing_token() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
    assert_eq!(body["error"]["message"], "Não autorizado");
}

#[tokio::test]
async fn test_auth_rejects_invalid_token() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("Authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_accepts_bearer_token() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("Authorization", "Bearer secret123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_accepts_x_api_key() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("X-API-Key", "secret123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_accepts_x_token() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("X-Token", "secret123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_accepts_allowed_tokens() {
    let auth = AuthConfig {
        token: Some("primary-secret".to_string()),
        allowed_tokens: vec!["secondary-secret".to_string()],
    };
    let app = test_app(auth);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("Authorization", "Bearer secondary-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_public_health_bypasses_auth() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth.clone());

    let res1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    let res2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_public_redoc_bypasses_auth() {
    let auth = AuthConfig {
        token: Some("secret123".to_string()),
        allowed_tokens: vec![],
    };
    let app = test_app(auth);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/redoc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
