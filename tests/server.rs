use std::sync::Arc;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::server::build_router;

#[test]
fn test_build_router_succeeds() {
    let config = AppConfig::default();
    let data_loader = Arc::new(DataLoader::new(&config.data).unwrap());
    let router = build_router(&config, data_loader);
    // Router should be buildable without panic
    drop(router);
}
