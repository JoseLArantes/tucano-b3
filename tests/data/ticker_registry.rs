use std::path::Path;
use tucano_b3::data::ticker_registry::TickerRegistry;

#[test]
fn test_load_missing_ledger() {
    let registry = TickerRegistry::load(Path::new("/nonexistent/tucano.db"));
    assert!(registry.is_ok());
    let registry = registry.unwrap();
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_exists_unknown_ticker() {
    let registry = TickerRegistry::load(Path::new("/nonexistent/tucano.db")).unwrap();
    assert!(!registry.exists("XYZXYZ"));
}

#[test]
fn test_get_unknown_ticker() {
    let registry = TickerRegistry::load(Path::new("/nonexistent/tucano.db")).unwrap();
    assert!(registry.get("XYZXYZ").is_none());
}

#[test]
fn test_list_empty() {
    let registry = TickerRegistry::load(Path::new("/nonexistent/tucano.db")).unwrap();
    let tickers = registry.list(true);
    assert!(tickers.is_empty());
}
