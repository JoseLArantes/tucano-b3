use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use crate::db::enums::ResolutionStatus;
use crate::error::Result;
use crate::parsers::canonical::{canonicalize_isin, canonicalize_string, canonicalize_ticker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionResult {
    pub instrument_id: Option<String>,
    pub status: ResolutionStatus,
}

pub struct RelationshipResolver<'a> {
    pool: &'a PgPool,
}

impl<'a> RelationshipResolver<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn resolve_instrument(
        &self,
        security_id: Option<&str>,
        ticker_symbol: Option<&str>,
        isin: Option<&str>,
        market_code: Option<&str>,
        as_of: Option<NaiveDate>,
    ) -> Result<ResolutionResult> {
        let sec_id = canonicalize_string(security_id);
        let ticker = canonicalize_ticker(ticker_symbol);
        let isin_code = canonicalize_isin(isin);
        let mkt = canonicalize_string(market_code);

        // Priority 1: security_id
        if let Some(ref sid) = sec_id {
            let row = sqlx::query("SELECT id FROM b3_instruments WHERE security_id = $1")
                .bind(sid)
                .fetch_optional(self.pool)
                .await?;

            if let Some(r) = row {
                let id: String = r.get("id");
                return Ok(ResolutionResult {
                    instrument_id: Some(id),
                    status: ResolutionStatus::Resolved,
                });
            }
        }

        // Priority 2: ticker and/or isin against b3_instrument_identifiers with validity window
        if ticker.is_none() && isin_code.is_none() {
            return Ok(ResolutionResult {
                instrument_id: None,
                status: ResolutionStatus::Unresolved,
            });
        }

        let mut matches = Vec::new();

        if let Some(ref t) = ticker {
            let rows = sqlx::query(
                r#"
                SELECT DISTINCT instrument_id
                FROM b3_instrument_identifiers
                WHERE identifier_type = 'ticker'
                  AND identifier_value = $1
                  AND ($2::varchar IS NULL OR market_code = $2 OR market_code IS NULL)
                  AND ($3::date IS NULL OR (
                        (valid_from IS NULL OR valid_from <= $3) AND
                        (valid_to IS NULL OR valid_to >= $3)
                  ))
                "#,
            )
            .bind(t)
            .bind(&mkt)
            .bind(as_of)
            .fetch_all(self.pool)
            .await?;

            for r in rows {
                let id: String = r.get("instrument_id");
                matches.push(id);
            }
        }

        if let Some(ref i) = isin_code {
            let rows = sqlx::query(
                r#"
                SELECT DISTINCT instrument_id
                FROM b3_instrument_identifiers
                WHERE identifier_type = 'isin'
                  AND identifier_value = $1
                  AND ($2::varchar IS NULL OR market_code = $2 OR market_code IS NULL)
                  AND ($3::date IS NULL OR (
                        (valid_from IS NULL OR valid_from <= $3) AND
                        (valid_to IS NULL OR valid_to >= $3)
                  ))
                "#,
            )
            .bind(i)
            .bind(&mkt)
            .bind(as_of)
            .fetch_all(self.pool)
            .await?;

            for r in rows {
                let id: String = r.get("instrument_id");
                matches.push(id);
            }
        }

        matches.sort();
        matches.dedup();

        if matches.is_empty() {
            Ok(ResolutionResult {
                instrument_id: None,
                status: ResolutionStatus::Unresolved,
            })
        } else if matches.len() == 1 {
            Ok(ResolutionResult {
                instrument_id: Some(matches.remove(0)),
                status: ResolutionStatus::Resolved,
            })
        } else {
            Ok(ResolutionResult {
                instrument_id: None,
                status: ResolutionStatus::Ambiguous,
            })
        }
    }

    pub async fn resolve_or_create_index(
        &self,
        source_file_id: &str,
        index_code: &str,
        index_name: Option<&str>,
        source_row_hash: &str,
    ) -> Result<String> {
        let code = index_code.trim();
        let existing = sqlx::query("SELECT id FROM b3_indices WHERE index_code = $1")
            .bind(code)
            .fetch_optional(self.pool)
            .await?;

        if let Some(row) = existing {
            let id: String = row.get("id");
            return Ok(id);
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO b3_indices (
                id, index_code, index_name, source_file_id,
                source_row_hash, created_at, updated_at,
                last_seen_at, data_changed_at, field_changed_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), NOW(), NOW(), '{"index_code": "created"}'::jsonb)
            ON CONFLICT (index_code) DO NOTHING
            "#,
        )
        .bind(&new_id)
        .bind(code)
        .bind(index_name)
        .bind(source_file_id)
        .bind(source_row_hash)
        .execute(self.pool)
        .await?;

        let final_row = sqlx::query("SELECT id FROM b3_indices WHERE index_code = $1")
            .bind(code)
            .fetch_one(self.pool)
            .await?;

        let final_id: String = final_row.get("id");
        Ok(final_id)
    }

    pub async fn rebuild_unresolved_identities(&self) -> Result<usize> {
        let mut resolved_count = 0;

        // 1. Tradable securities staging
        let staging_rows = sqlx::query(
            r#"
            SELECT id, security_id, ticker_symbol, isin, market_code, reference_date
            FROM b3_tradable_securities_staging
            WHERE resolution_status IN ('unresolved', 'ambiguous')
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        for row in staging_rows {
            let id: String = row.get("id");
            let sec_id: Option<String> = row.get("security_id");
            let ticker: Option<String> = row.get("ticker_symbol");
            let isin: Option<String> = row.get("isin");
            let mkt: Option<String> = row.get("market_code");
            let ref_dt: NaiveDate = row.get("reference_date");

            let res = self
                .resolve_instrument(
                    sec_id.as_deref(),
                    ticker.as_deref(),
                    isin.as_deref(),
                    mkt.as_deref(),
                    Some(ref_dt),
                )
                .await?;

            if res.status == ResolutionStatus::Resolved {
                if let Some(ref _inst_id) = res.instrument_id {
                    sqlx::query(
                        r#"
                        UPDATE b3_tradable_securities_staging
                        SET resolution_status = 'resolved'
                        WHERE id = $1
                        "#,
                    )
                    .bind(&id)
                    .execute(self.pool)
                    .await?;
                    resolved_count += 1;
                }
            }
        }

        // 2. COTAHIST quotes
        let cotahist_rows = sqlx::query(
            r#"
            SELECT id, ticker_symbol, isin, market_type, trade_date
            FROM b3_cotahist_quotes
            WHERE resolution_status IN ('unresolved', 'ambiguous')
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        for row in cotahist_rows {
            let id: String = row.get("id");
            let ticker: Option<String> = row.get("ticker_symbol");
            let isin: Option<String> = row.get("isin");
            let _mkt_type: Option<String> = row.get("market_type");
            let trade_dt: NaiveDate = row.get("trade_date");

            let res = self
                .resolve_instrument(
                    None,
                    ticker.as_deref(),
                    isin.as_deref(),
                    None,
                    Some(trade_dt),
                )
                .await?;

            if res.status == ResolutionStatus::Resolved {
                sqlx::query(
                    r#"
                    UPDATE b3_cotahist_quotes
                    SET instrument_id = $1, resolution_status = 'resolved'
                    WHERE id = $2
                    "#,
                )
                .bind(res.instrument_id)
                .bind(&id)
                .execute(self.pool)
                .await?;
                resolved_count += 1;
            }
        }

        Ok(resolved_count)
    }
}
