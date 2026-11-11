use tucano_b3::services::query_optimizer::*;

#[test]
fn test_latest_quote_sql() {
    let sql = latest_quote_sql("vale3");
    assert!(sql.contains("ticker = 'VALE3'"));
    assert!(sql.contains("LIMIT 2"));
    assert!(sql.contains("ORDER BY date DESC"));
}

#[test]
fn test_quote_history_sql_with_dates() {
    let sql = quote_history_sql(
        "PETR4",
        Some("2026-01-01"),
        Some("2026-01-31"),
        50,
        0,
        "DESC",
    );
    assert!(sql.contains("ticker = 'PETR4'"));
    assert!(sql.contains("date >= '2026-01-01'"));
    assert!(sql.contains("date <= '2026-01-31'"));
    assert!(sql.contains("ORDER BY date DESC"));
    assert!(sql.contains("LIMIT 50 OFFSET 0"));
}

#[test]
fn test_quote_history_sql_no_dates() {
    let sql = quote_history_sql("ITUB4", None, None, 20, 10, "ASC");
    assert!(sql.contains("ticker = 'ITUB4'"));
    assert!(!sql.contains("date >="));
    assert!(!sql.contains("date <="));
    assert!(sql.contains("ORDER BY date ASC"));
    assert!(sql.contains("LIMIT 20 OFFSET 10"));
}

#[test]
fn test_count_sql() {
    let sql_all = count_sql("cotahist", "");
    assert_eq!(sql_all, "SELECT COUNT(*) FROM cotahist");

    let sql_where = count_sql("cotahist", "ticker = 'VALE3'");
    assert_eq!(
        sql_where,
        "SELECT COUNT(*) FROM cotahist WHERE ticker = 'VALE3'"
    );
}
