use tucano_b3::config::AppConfig;
use tucano_b3::server::build_router;

#[test]
fn test_build_router_succeeds() {
    let config = AppConfig::default();
    let router = build_router(&config);
    // Router should be buildable without panic
    drop(router);
}
