use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{PaginatedResponse, PaginationParams};
use crate::api::AppState;

/// Dados cadastrais consolidados de companhias listadas na B3 (derivados da base de instrumentos).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyDataItem {
    /// Código base do ativo / emissor na B3 (ex: `PETR`, `VALE`).
    #[schema(example = "PETR")]
    pub asset: String,

    /// Descrição ou razão social consolidada do emissor na B3.
    #[schema(example = "PETROLEO BRASILEIRO S.A. PETROBRAS")]
    pub asset_description: Option<String>,

    /// Mercado de negociação predominante (ex: `VISTA`).
    #[schema(example = "VISTA")]
    pub market_name: Option<String>,

    /// Segmento de listagem da B3 (ex: `MERCADO A VISTA`).
    #[schema(example = "MERCADO A VISTA")]
    pub segment_name: Option<String>,

    /// Lista de todos os símbolos de negociação (tickers) emitidos pela companhia (ex: `["PETR3", "PETR4"]`).
    #[schema(example = json!(["PETR3", "PETR4"]))]
    pub tickers: Vec<String>,

    /// Lista de todos os códigos ISIN emitidos pela companhia.
    #[schema(example = json!(["BRPETRACNOR9", "BRPETRACNPR6"]))]
    pub isins: Vec<String>,
}

/// Parâmetros de filtro para busca de dados consolidados de companhias.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CompanyDataFilterParams {
    /// Termo de busca textual por código base do ativo (`asset`) ou descrição (`asset_description`).
    #[param(example = "PETROBRAS")]
    pub search: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/company-data",
    tag = "B3 Company Data",
    summary = "Listagem Consolidada de Companhias e Emissores",
    description = "Retorna visão consolidada de empresas e emissores listados na B3, agregando todos os tickers e ISINs associados ao código base do ativo (`asset`). \n\n### Escopo Estrito B3-Only:\n- Esta visão é construída exclusivamente a partir do cadastro oficial de instrumentos (`BVBG.028.02`).\n- Não utiliza fontes externas ou identificadores fora do ecossistema da bolsa (ex: CVM, Yahoo Finance).",
    params(PaginationParams, CompanyDataFilterParams),
    responses(
        (status = 200, description = "Lista consolidada de companhias retornada com sucesso.", body = PaginatedResponse<CompanyDataItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_company_data(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<CompanyDataFilterParams>,
) -> Result<Json<PaginatedResponse<CompanyDataItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let search = filters
        .search
        .as_deref()
        .map(|s| format!("%{}%", s.trim().to_uppercase()));

    let count_query = r#"
        SELECT count(DISTINCT asset)
        FROM b3_instruments
        WHERE asset IS NOT NULL
          AND ($1::varchar IS NULL OR UPPER(asset) LIKE $1 OR UPPER(asset_description) LIKE $1)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&search)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            asset,
            MAX(asset_description) as asset_description,
            MAX(market_name) as market_name,
            MAX(segment_name) as segment_name,
            ARRAY_AGG(DISTINCT (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = b3_instruments.id AND identifier_type = 'ticker')) FILTER (WHERE (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = b3_instruments.id AND identifier_type = 'ticker') IS NOT NULL) as tickers,
            ARRAY_AGG(DISTINCT (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = b3_instruments.id AND identifier_type = 'isin')) FILTER (WHERE (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = b3_instruments.id AND identifier_type = 'isin') IS NOT NULL) as isins
        FROM b3_instruments
        WHERE asset IS NOT NULL
          AND ($1::varchar IS NULL OR UPPER(asset) LIKE $1 OR UPPER(asset_description) LIKE $1)
        GROUP BY asset
        ORDER BY asset ASC
        LIMIT $2 OFFSET $3
    "#;

    let rows = sqlx::query(select_query)
        .bind(&search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| {
            let tickers: Option<Vec<String>> = r.get("tickers");
            let isins: Option<Vec<String>> = r.get("isins");

            CompanyDataItem {
                asset: r.get("asset"),
                asset_description: r.get("asset_description"),
                market_name: r.get("market_name"),
                segment_name: r.get("segment_name"),
                tickers: tickers.unwrap_or_default(),
                isins: isins.unwrap_or_default(),
            }
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}
