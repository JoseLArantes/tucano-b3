use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

pub async fn handle_healthcheck(pool: &PgPool) -> Value {
    let db_ok = sqlx::query("SELECT 1").execute(pool).await.is_ok();
    json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "service": "tucano-b3-mcp",
        "database_connected": db_ok,
    })
}

pub async fn handle_search_instruments(pool: &PgPool, args: &Value) -> Value {
    let query_str = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100);

    let pattern = format!("%{}%", query_str.to_uppercase());

    let sql = r#"
        SELECT 
            i.id, i.security_id, i.asset, i.asset_description, i.market_name, i.segment_name,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as isin
        FROM b3_instruments i
        WHERE UPPER(i.security_id) LIKE $1 
           OR UPPER(i.asset) LIKE $1 
           OR UPPER(i.asset_description) LIKE $1
           OR EXISTS (SELECT 1 FROM b3_instrument_identifiers WHERE instrument_id = i.id AND UPPER(identifier_value) LIKE $1)
        ORDER BY i.security_id ASC
        LIMIT $2
    "#;

    let rows = match sqlx::query(sql)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.get::<String, _>("id"),
                "security_id": r.get::<String, _>("security_id"),
                "ticker": r.get::<Option<String>, _>("ticker"),
                "isin": r.get::<Option<String>, _>("isin"),
                "asset": r.get::<Option<String>, _>("asset"),
                "asset_description": r.get::<Option<String>, _>("asset_description"),
                "market_name": r.get::<Option<String>, _>("market_name"),
                "segment_name": r.get::<Option<String>, _>("segment_name"),
            })
        })
        .collect();

    json!({ "count": items.len(), "instruments": items })
}

pub async fn handle_get_instrument_coverage(pool: &PgPool, args: &Value) -> Value {
    let ticker = args
        .get("ticker")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_uppercase();

    let sql = r#"
        SELECT 
            min(trade_date) as min_date,
            max(trade_date) as max_date,
            count(*) as quote_count
        FROM b3_cotahist_quotes
        WHERE ticker_symbol = $1
    "#;

    let row = match sqlx::query(sql).bind(&ticker).fetch_one(pool).await {
        Ok(r) => r,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let min_d: Option<NaiveDate> = row.get("min_date");
    let max_d: Option<NaiveDate> = row.get("max_date");
    let count: i64 = row.get("quote_count");

    json!({
        "ticker": ticker,
        "first_trade_date": min_d,
        "last_trade_date": max_d,
        "total_quotes": count,
    })
}

pub async fn handle_get_ohlcv_series(pool: &PgPool, args: &Value) -> Value {
    let ticker = args
        .get("ticker")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_uppercase();
    let start_date = args
        .get("start_date")
        .and_then(|v| v.as_str())
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let end_date = args
        .get("end_date")
        .and_then(|v| v.as_str())
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(250)
        .clamp(1, 1000);

    let sql = r#"
        SELECT 
            trade_date, open_price, high_price, low_price, last_price,
            traded_quantity, traded_volume, trades_count
        FROM b3_cotahist_quotes
        WHERE ticker_symbol = $1
          AND ($2::date IS NULL OR trade_date >= $2)
          AND ($3::date IS NULL OR trade_date <= $3)
        ORDER BY trade_date ASC
        LIMIT $4
    "#;

    let rows = match sqlx::query(sql)
        .bind(&ticker)
        .bind(start_date)
        .bind(end_date)
        .bind(limit)
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let candles: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "trade_date": r.get::<NaiveDate, _>("trade_date"),
                "open": r.get::<Option<Decimal>, _>("open_price"),
                "high": r.get::<Option<Decimal>, _>("high_price"),
                "low": r.get::<Option<Decimal>, _>("low_price"),
                "close": r.get::<Option<Decimal>, _>("last_price"),
                "quantity": r.get::<Option<Decimal>, _>("traded_quantity"),
                "volume": r.get::<Option<Decimal>, _>("traded_volume"),
                "trades": r.get::<Option<i32>, _>("trades_count"),
            })
        })
        .collect();

    json!({
        "ticker": ticker,
        "count": candles.len(),
        "disclaimer": "Preços nominais brutos históricos da B3 (não ajustados retroativamente por proventos ou desdobramentos).",
        "series": candles
    })
}

pub async fn handle_get_index_portfolio(pool: &PgPool, args: &Value) -> Value {
    let index_code = args
        .get("index_code")
        .and_then(|v| v.as_str())
        .unwrap_or("IBOV")
        .trim()
        .to_uppercase();

    let sql = r#"
        SELECT 
            composition_date, ticker_symbol, isin, asset_name, theoretical_quantity, portfolio_weight
        FROM b3_index_portfolio_compositions
        WHERE index_code = $1
          AND composition_date = (SELECT max(composition_date) FROM b3_index_portfolio_compositions WHERE index_code = $1)
        ORDER BY portfolio_weight DESC NULLS LAST
    "#;

    let rows = match sqlx::query(sql).bind(&index_code).fetch_all(pool).await {
        Ok(r) => r,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let components: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "ticker": r.get::<Option<String>, _>("ticker_symbol"),
                "isin": r.get::<Option<String>, _>("isin"),
                "asset_name": r.get::<Option<String>, _>("asset_name"),
                "theoretical_quantity": r.get::<Option<Decimal>, _>("theoretical_quantity"),
                "weight": r.get::<Option<Decimal>, _>("portfolio_weight"),
            })
        })
        .collect();

    json!({
        "index_code": index_code,
        "component_count": components.len(),
        "components": components
    })
}

pub async fn handle_get_dataset_diagnostics(pool: &PgPool) -> Value {
    let queries = vec![
        (
            "b3_instruments",
            "SELECT min(created_at::date), max(created_at::date), count(*) FROM b3_instruments",
        ),
        (
            "b3_tradable_securities",
            "SELECT min(reference_date), max(reference_date), count(*) FROM b3_tradable_securities",
        ),
        (
            "b3_price_report_quotes",
            "SELECT min(trade_date), max(trade_date), count(*) FROM b3_price_report_quotes",
        ),
        (
            "b3_cotahist_quotes",
            "SELECT min(trade_date), max(trade_date), count(*) FROM b3_cotahist_quotes",
        ),
        (
            "b3_quarantine_records",
            "SELECT NULL::date, NULL::date, count(*) FROM b3_quarantine_records",
        ),
        (
            "b3_field_changes",
            "SELECT NULL::date, NULL::date, count(*) FROM b3_field_changes",
        ),
    ];

    let mut map = serde_json::Map::new();

    for (name, q) in queries {
        if let Ok(row) = sqlx::query(q).fetch_one(pool).await {
            let min_d: Option<NaiveDate> = row.get(0);
            let max_d: Option<NaiveDate> = row.get(1);
            let count: i64 = row.get(2);
            map.insert(
                name.to_string(),
                json!({
                    "min_date": min_d,
                    "max_date": max_d,
                    "total_rows": count,
                }),
            );
        }
    }

    json!({ "diagnostics": map })
}
