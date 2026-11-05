use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

use tucano_b3::parsers::canonical::*;

#[test]
fn test_canonicalize_string() {
    assert_eq!(canonicalize_string(None), None);
    assert_eq!(canonicalize_string(Some("")), None);
    assert_eq!(canonicalize_string(Some("   ")), None);
    assert_eq!(canonicalize_string(Some(" PETR4  ")), Some("PETR4".to_string()));
}

#[test]
fn test_canonicalize_ticker_and_isin() {
    assert_eq!(canonicalize_ticker(Some("petr4")), Some("PETR4".to_string()));
    assert_eq!(canonicalize_isin(Some("brpetracnpr6")), Some("BRPETRACNPR6".to_string()));
}

#[test]
fn test_canonicalize_date() {
    assert_eq!(canonicalize_date(None).unwrap(), None);
    assert_eq!(canonicalize_date(Some("")).unwrap(), None);
    assert_eq!(canonicalize_date(Some("00000000")).unwrap(), None);
    assert_eq!(canonicalize_date(Some("99991231")).unwrap(), None);
    assert_eq!(
        canonicalize_date(Some("20241230")).unwrap(),
        Some(NaiveDate::from_ymd_opt(2024, 12, 30).unwrap())
    );
    assert_eq!(
        canonicalize_date(Some("2024-12-30")).unwrap(),
        Some(NaiveDate::from_ymd_opt(2024, 12, 30).unwrap())
    );
}

#[test]
fn test_canonicalize_decimal() {
    assert_eq!(canonicalize_decimal(None).unwrap(), None);
    assert_eq!(canonicalize_decimal(Some("36.25")).unwrap(), Some(Decimal::new(3625, 2)));
    assert_eq!(canonicalize_decimal(Some("36,25")).unwrap(), Some(Decimal::new(3625, 2)));
    assert_eq!(canonicalize_decimal(Some("36.25000")).unwrap(), Some(Decimal::new(3625, 2)));
}

#[test]
fn test_parse_implied_decimal() {
    assert_eq!(parse_implied_decimal(Some("3625"), 2).unwrap(), Some(Decimal::new(3625, 2)));
    assert_eq!(parse_implied_decimal(Some("10000000"), 0).unwrap(), Some(Decimal::new(10000000, 0)));
}

#[test]
fn test_stable_row_hash_deterministic() {
    let mut fields1 = BTreeMap::new();
    fields1.insert("ticker".to_string(), serde_json::Value::String("PETR4".to_string()));
    fields1.insert("price".to_string(), serde_json::Value::String("36.25".to_string()));

    let mut fields2 = BTreeMap::new();
    fields2.insert("price".to_string(), serde_json::Value::String("36.25".to_string()));
    fields2.insert("ticker".to_string(), serde_json::Value::String("PETR4".to_string()));

    assert_eq!(stable_row_hash(&fields1), stable_row_hash(&fields2));
}

