use axum::body::Body;
use axum::http::Request;
use axum::response::Response;
use std::convert::Infallible;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;
use tucano_b3::config::AppConfig;

pub struct TestApp {
    router: axum::Router,
    _data_dir: TempDir,
}

impl TestApp {
    pub async fn oneshot(&self, request: Request<Body>) -> Result<Response, Infallible> {
        self.router.clone().oneshot(request).await
    }
}

pub fn empty_app() -> TestApp {
    app_with_config(AppConfig::default())
}

pub fn authenticated_app() -> TestApp {
    let mut config = AppConfig::default();
    config.auth.token = Some("calendar-test-token".to_string());
    app_with_config(config)
}

fn app_with_config(mut config: AppConfig) -> TestApp {
    let data_dir = tempfile::tempdir().expect("create isolated test data directory");
    config.data.dir = data_dir.path().to_path_buf();

    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader = Arc::new(
        tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone())
            .expect("load empty test data directory"),
    );
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    let router = tucano_b3::server::build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        Arc::new(tucano_b3::metrics::AppMetrics::new()),
        std::time::Instant::now(),
    );

    TestApp {
        router,
        _data_dir: data_dir,
    }
}
