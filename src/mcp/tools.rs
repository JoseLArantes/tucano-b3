use crate::services::query::{
    InstrumentFilters, QueryError, QueryPurpose, QueryService, QuoteFilters,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

pub fn tool_list() -> Value {
    json!([
        {"name":"healthcheck","description":"Verifica a dependência analítica somente leitura.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}},
        {"name":"search_instruments","description":"Busca evidências cadastrais de fotografias publicadas, sem retroatividade inventada.","inputSchema":{"type":"object","properties":{"query":{"type":"string"},"ticker_symbol":{"type":"string"},"security_id":{"type":"string"},"isin":{"type":"string"},"snapshot_id":{"type":"string","format":"uuid"},"source_file_type":{"type":"string"},"limit":{"type":"integer","default":100,"minimum":1,"maximum":1000},"offset":{"type":"integer","default":0,"minimum":0}},"additionalProperties":false}},
        {"name":"get_ohlcv_series","description":"Consulta COTAHIST nominal sem ajustes. PREULT é último negócio. Série ambígua exige mercado/distribuição/prazo. Retorna revisão, proveniência e cobertura.","inputSchema":quote_schema()},
        {"name":"get_instrument_coverage","description":"Consulta cobertura observada da revisão com o mesmo filtro de fatos; min/max não provam continuidade.","inputSchema":quote_schema()},
        {"name":"get_dataset_diagnostics","description":"Diagnóstico da revisão publicada e autorizada, sem ingestão ou SQL arbitrário.","inputSchema":{"type":"object","properties":{"data_revision":{"type":"string","format":"uuid"}},"additionalProperties":false}}
    ])
}
fn quote_schema() -> Value {
    json!({"type":"object","properties":{"ticker_symbol":{"type":"string"},"instrument_id":{"type":"string"},"isin":{"type":"string"},"trade_date_from":{"type":"string","format":"date"},"trade_date_to":{"type":"string","format":"date"},"market_type":{"type":"string"},"bdi_code":{"type":"string"},"distribution_number":{"type":"string"},"term_days":{"type":"string"},"data_revision":{"type":"string","format":"uuid"},"limit":{"type":"integer","default":100,"minimum":1,"maximum":1000},"offset":{"type":"integer","default":0,"minimum":0},"include_internal":{"type":"boolean","default":false}},"additionalProperties":false})
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionArgs {
    data_revision: Option<uuid::Uuid>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ReadHealth {
    healthy: bool,
}

pub async fn call(pool: &PgPool, name: &str, args: Value) -> Result<Value, QueryError> {
    let service = QueryService::new(pool.clone());
    match name {
        "healthcheck" => {
            let _: EmptyArgs = serde_json::from_value(args)
                .map_err(|_| QueryError::Invalid("Argumentos inválidos.".into()))?;
            sqlx::query("SELECT 1").execute(pool).await?;
            Ok(serde_json::to_value(ReadHealth { healthy: true })
                .expect("typed health serializable"))
        }
        "search_instruments" => {
            let filters: InstrumentFilters = serde_json::from_value(args)
                .map_err(|_| QueryError::Invalid("Filtros cadastrais inválidos.".into()))?;
            Ok(
                serde_json::to_value(service.instruments(filters, QueryPurpose::Mcp).await?)
                    .expect("typed registry serializable"),
            )
        }
        "get_ohlcv_series" | "get_instrument_coverage" => {
            let filters: QuoteFilters = serde_json::from_value(args)
                .map_err(|_| QueryError::Invalid("Filtros COTAHIST inválidos.".into()))?;
            let series = name == "get_ohlcv_series";
            let page = service.quotes(filters, QueryPurpose::Mcp, series).await?;
            Ok(serde_json::to_value(page).expect("typed quote serializable"))
        }
        "get_dataset_diagnostics" => {
            let args: RevisionArgs = serde_json::from_value(args)
                .map_err(|_| QueryError::Invalid("Argumentos inválidos.".into()))?;
            Ok(serde_json::to_value(
                service
                    .diagnostics(
                        crate::services::query::DiagnosticsFilters {
                            data_revision: args.data_revision,
                        },
                        QueryPurpose::Mcp,
                    )
                    .await?,
            )
            .expect("typed diagnostics serializable"))
        }
        _ => Err(QueryError::Invalid(
            "Ferramenta inexistente ou não analítica.".into(),
        )),
    }
}

// Public helpers remain transport adapters over the same query service.
pub async fn handle_healthcheck(pool: &PgPool) -> Value {
    call(pool, "healthcheck", json!({}))
        .await
        .unwrap_or_else(safe_error)
}
pub async fn handle_search_instruments(pool: &PgPool, args: &Value) -> Value {
    call(pool, "search_instruments", args.clone())
        .await
        .unwrap_or_else(safe_error)
}
pub async fn handle_get_instrument_coverage(pool: &PgPool, args: &Value) -> Value {
    call(pool, "get_instrument_coverage", args.clone())
        .await
        .unwrap_or_else(safe_error)
}
pub async fn handle_get_ohlcv_series(pool: &PgPool, args: &Value) -> Value {
    call(pool, "get_ohlcv_series", args.clone())
        .await
        .unwrap_or_else(safe_error)
}
pub async fn handle_get_dataset_diagnostics(pool: &PgPool) -> Value {
    call(pool, "get_dataset_diagnostics", json!({}))
        .await
        .unwrap_or_else(safe_error)
}
fn safe_error(error: QueryError) -> Value {
    json!({"error":error.to_string()})
}
