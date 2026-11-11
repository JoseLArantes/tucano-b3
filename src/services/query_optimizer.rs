//! Query optimization utilities.
//!
//! Provides helpers for building efficient SQL queries
//! that leverage DataFusion's query planner.

/// Build an optimized query for latest ticker quote.
///
/// Uses LIMIT 2 to fetch only the rows needed for var_pct computation.
/// Avoids full table scans.
pub fn latest_quote_sql(ticker: &str) -> String {
    format!(
        "SELECT ticker, date, pre_abr, pre_max, pre_min, pre_med, pre_ult, \
                pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot \
         FROM cotahist \
         WHERE ticker = '{}' \
         ORDER BY date DESC \
         LIMIT 2",
        ticker.to_uppercase()
    )
}

/// Build an optimized query for quote history with date range.
///
/// Pushes date filter into Parquet reader via predicate pushdown.
/// Only selects columns needed by the API response.
pub fn quote_history_sql(
    ticker: &str,
    since: Option<&str>,
    until: Option<&str>,
    limit: u32,
    offset: u32,
    order: &str,
) -> String {
    let mut sql = format!(
        "SELECT ticker, date, pre_abr, pre_max, pre_min, pre_med, pre_ult, \
                pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot \
         FROM cotahist \
         WHERE ticker = '{}'",
        ticker.to_uppercase()
    );

    if let Some(s) = since {
        sql.push_str(&format!(" AND date >= '{}'", s));
    }
    if let Some(u) = until {
        sql.push_str(&format!(" AND date <= '{}'", u));
    }

    sql.push_str(&format!(" ORDER BY date {}", order));
    sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    sql
}

/// Build an optimized count query.
///
/// Uses COUNT(*) which DataFusion can optimize via parquet metadata
/// without reading actual row data.
pub fn count_sql(table: &str, where_clause: &str) -> String {
    if where_clause.is_empty() {
        format!("SELECT COUNT(*) FROM {}", table)
    } else {
        format!("SELECT COUNT(*) FROM {} WHERE {}", table, where_clause)
    }
}
