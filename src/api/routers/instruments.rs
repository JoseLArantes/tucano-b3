use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentItem {
    pub id: String,
    pub security_id: String,
    pub security_source: Option<String>,
    pub market_identifier_code: Option<String>,
    pub asset: Option<String>,
    pub asset_description: Option<String>,
    pub market_code: Option<String>,
    pub market_name: Option<String>,
    pub segment_code: Option<String>,
    pub segment_name: Option<String>,
    pub instrument_type: Option<String>,
    pub security_category: Option<String>,
    pub cfi_code: Option<String>,
    pub instrument_status: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentifierItem {
    pub id: String,
    pub instrument_id: String,
    pub identifier_type: String,
    pub identifier_value: String,
    pub market_code: Option<String>,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct InstrumentFilterParams {
    pub ticker: Option<String>,
    pub isin: Option<String>,
    pub security_id: Option<String>,
    pub market_code: Option<String>,
    pub segment_code: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments",
    tag = "B3 Instruments File (BVBG.028.02)",
    params(PaginationParams, InstrumentFilterParams),
    responses(
        (status = 200, description = "List canonical instruments", body = PaginatedResponse<InstrumentItem>)
    )
)]
pub async fn list_instruments(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<InstrumentFilterParams>,
) -> Result<Json<PaginatedResponse<InstrumentItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let isin = filters.isin.as_deref().map(|s| s.trim().to_uppercase());
    let sec_id = filters.security_id.as_deref().map(|s| s.trim());
    let market = filters.market_code.as_deref().map(|s| s.trim().to_uppercase());
    let segment = filters.segment_code.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(DISTINCT i.id)
        FROM b3_instruments i
        LEFT JOIN b3_instrument_identifiers ident ON ident.instrument_id = i.id
        WHERE ($1::varchar IS NULL OR ident.identifier_type = 'ticker' AND ident.identifier_value = $1)
          AND ($2::varchar IS NULL OR ident.identifier_type = 'isin' AND ident.identifier_value = $2)
          AND ($3::varchar IS NULL OR i.security_id = $3)
          AND ($4::varchar IS NULL OR i.market_code = $4)
          AND ($5::varchar IS NULL OR i.segment_code = $5)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&ticker)
        .bind(&isin)
        .bind(&sec_id)
        .bind(&market)
        .bind(&segment)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instruments i
        JOIN b3_source_files f ON f.id = i.source_file_id
        LEFT JOIN b3_instrument_identifiers ident ON ident.instrument_id = i.id
        WHERE ($1::varchar IS NULL OR ident.identifier_type = 'ticker' AND ident.identifier_value = $1)
          AND ($2::varchar IS NULL OR ident.identifier_type = 'isin' AND ident.identifier_value = $2)
          AND ($3::varchar IS NULL OR i.security_id = $3)
          AND ($4::varchar IS NULL OR i.market_code = $4)
          AND ($5::varchar IS NULL OR i.segment_code = $5)
        GROUP BY i.id, f.id
        ORDER BY i.security_id ASC
        LIMIT $6 OFFSET $7
    "#;

    let rows = sqlx::query(select_query)
        .bind(&ticker)
        .bind(&isin)
        .bind(&sec_id)
        .bind(&market)
        .bind(&segment)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| {
            let id: String = row.get("id");
            let sec_id: String = row.get("security_id");
            let provenance = Provenance {
                source_file_type: row.get("source_file_type"),
                source_file_id: row.get("source_file_id"),
                source_file_name: row.get("original_file_name"),
                source_member_name: row.get("source_member_name"),
                reference_date: row.get("reference_date"),
                sha256: row.get("sha256"),
                ingested_at: row.get("ingested_at"),
            };

            let internal_metadata = if pagination.include_internal() {
                Some(InternalMetadata {
                    data_changed_at: row.get("data_changed_at"),
                    field_changed_at: row.get("field_changed_at"),
                    resolution_status: Some("resolved".to_string()),
                    last_seen_at: row.get("last_seen_at"),
                    source_line_number: row.get("source_line_number"),
                })
            } else {
                None
            };

            InstrumentItem {
                id,
                security_id: sec_id,
                security_source: row.get("security_source"),
                market_identifier_code: row.get("market_identifier_code"),
                asset: row.get("asset"),
                asset_description: row.get("asset_description"),
                market_code: row.get("market_code"),
                market_name: row.get("market_name"),
                segment_code: row.get("segment_code"),
                segment_name: row.get("segment_name"),
                instrument_type: row.get("instrument_type"),
                security_category: row.get("security_category"),
                cfi_code: row.get("cfi_code"),
                instrument_status: row.get("instrument_status"),
                maturity_date: row.get("maturity_date"),
                ticker_symbol: row.get("current_ticker"),
                isin: row.get("current_isin"),
                provenance,
                internal_metadata,
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

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments/{id}",
    tag = "B3 Instruments File (BVBG.028.02)",
    responses(
        (status = 200, description = "Get instrument by primary ID or security ID", body = InstrumentItem),
        (status = 404, description = "Instrument not found")
    )
)]
pub async fn get_instrument(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instruments i
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE i.id = $1 OR i.security_id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Instrument not found".to_string()))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

pub async fn get_instrument_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let clean_ticker = ticker.trim().to_uppercase();
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            ident.identifier_value as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instrument_identifiers ident
        JOIN b3_instruments i ON i.id = ident.instrument_id
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE ident.identifier_type = 'ticker' AND ident.identifier_value = $1
        ORDER BY ident.valid_to NULLS FIRST, ident.valid_from DESC
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&clean_ticker)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("Instrument not found for ticker {clean_ticker}")))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

pub async fn get_instrument_by_isin(
    State(state): State<Arc<AppState>>,
    Path(isin): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let clean_isin = isin.trim().to_uppercase();
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            ident.identifier_value as current_isin
        FROM b3_instrument_identifiers ident
        JOIN b3_instruments i ON i.id = ident.instrument_id
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE ident.identifier_type = 'isin' AND ident.identifier_value = $1
        ORDER BY ident.valid_to NULLS FIRST, ident.valid_from DESC
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&clean_isin)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("Instrument not found for ISIN {clean_isin}")))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

pub async fn list_instrument_identifiers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<IdentifierItem>>, (StatusCode, String)> {
    let query = r#"
        SELECT id, instrument_id, identifier_type, identifier_value, market_code, valid_from, valid_to, created_at
        FROM b3_instrument_identifiers
        WHERE instrument_id = $1 OR instrument_id = (SELECT id FROM b3_instruments WHERE security_id = $1 LIMIT 1)
        ORDER BY valid_from DESC NULLS LAST, identifier_type ASC
    "#;

    let rows = sqlx::query(query)
        .bind(&id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| IdentifierItem {
            id: r.get("id"),
            instrument_id: r.get("instrument_id"),
            identifier_type: r.get("identifier_type"),
            identifier_value: r.get("identifier_value"),
            market_code: r.get("market_code"),
            valid_from: r.get("valid_from"),
            valid_to: r.get("valid_to"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(items))
}
