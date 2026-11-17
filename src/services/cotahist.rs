//! COTAHIST service layer.
//!
//! Builds and executes SQL queries for all COTAHIST endpoints.
//! All methods are async and return typed results.

use crate::data::query_service::QueryService;
use crate::data::ticker_registry::TickerRegistry;
use crate::error::{AppError, AppResult};
use crate::models::cotahist::*;
use crate::models::pagination::PaginationParams;
use arrow::array::*;
use arrow::record_batch::RecordBatch;
use std::collections::HashMap;

/// Service for COTAHIST data operations.
pub struct CotahistService;

impl CotahistService {
    /// List spot-market equities whose latest records identify a supported share class.
    pub async fn list_equities(
        query_service: &QueryService,
        registry: &TickerRegistry,
        prefix: Option<&str>,
        active_only: bool,
        page: u32,
        per_page: u32,
    ) -> AppResult<(Vec<EquityResponse>, usize, Option<EquityCoverage>)> {
        if !query_service.has_table("equities_v1") {
            return Ok((Vec::new(), 0, None));
        }
        let prefix = prefix.unwrap_or("").trim().to_ascii_uppercase();
        if !prefix.is_empty() && !prefix.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(AppError::InvalidParam {
                message: "O prefixo do ticker deve conter somente letras e números.".to_string(),
            });
        }
        let sql = "SELECT ticker, raw_ticker, isin, bdi_code, market_code, company_name, security_specification, MIN(date) AS first_seen, MAX(date) AS last_seen FROM equities_v1 WHERE market_type = 10 AND bdi_code = '02' AND currency_code = 'BRL' AND quotation_factor > 0 AND isin IS NOT NULL AND length(isin) = 12 AND (security_specification LIKE 'ON%' OR security_specification LIKE 'PN%' OR security_specification LIKE 'UNT%') GROUP BY ticker, raw_ticker, isin, bdi_code, market_code, company_name, security_specification ORDER BY ticker, isin, last_seen DESC, raw_ticker DESC, security_specification DESC";
        let batches = query_service
            .execute_sql(sql)
            .await
            .map_err(anyhow::Error::new)?;
        let mut rows = Vec::new();
        let mut row_positions = HashMap::new();
        let mut coverage_from: Option<chrono::NaiveDate> = None;
        let mut coverage_to: Option<chrono::NaiveDate> = None;
        for batch in &batches {
            for idx in 0..batch.num_rows() {
                let ticker = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .value(idx)
                    .to_string();
                let raw_ticker = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .value(idx)
                    .to_string();
                let isin = string_value(batch, 2, idx);
                if !valid_isin(&isin) {
                    continue;
                }
                let bdi_code = string_value(batch, 3, idx);
                let market_code = string_value(batch, 4, idx);
                let company_name = nullable_string_value(batch, 5, idx);
                let security_specification = string_value(batch, 6, idx);
                let first_seen = date32_string(batch, 7, idx);
                let last_seen = date32_string(batch, 8, idx);
                let first_day = chrono::NaiveDate::parse_from_str(&first_seen, "%Y-%m-%d").ok();
                let last_day = chrono::NaiveDate::parse_from_str(&last_seen, "%Y-%m-%d").ok();
                if let Some(day) = first_day {
                    coverage_from = Some(coverage_from.map_or(day, |current| current.min(day)));
                }
                if let Some(day) = last_day {
                    coverage_to = Some(coverage_to.map_or(day, |current| current.max(day)));
                }
                if !ticker.starts_with(&prefix) {
                    continue;
                }
                let is_active = registry.get(&ticker).is_some_and(|v| {
                    v.is_active
                        && v.isin
                            .as_deref()
                            .is_some_and(|active_isin| active_isin.eq_ignore_ascii_case(&isin))
                });
                if active_only && !is_active {
                    continue;
                }
                let candidate = EquityResponse {
                    ticker,
                    raw_ticker,
                    isin,
                    bdi_code,
                    market_code,
                    company_name,
                    security_specification,
                    first_seen,
                    last_seen,
                    is_active,
                };
                let key = (candidate.ticker.clone(), candidate.isin.clone());
                if let Some(index) = row_positions.get(&key).copied() {
                    let current: &mut EquityResponse = &mut rows[index];
                    if candidate.first_seen < current.first_seen {
                        current.first_seen = candidate.first_seen;
                    }
                    if candidate.last_seen > current.last_seen {
                        current.last_seen = candidate.last_seen;
                    }
                } else {
                    row_positions.insert(key, rows.len());
                    rows.push(candidate);
                }
            }
        }
        let total = rows.len();
        let offset = page
            .max(1)
            .saturating_sub(1)
            .saturating_mul(per_page.max(1)) as usize;
        let coverage = match (coverage_from, coverage_to) {
            (Some(from), Some(to)) => Some(EquityCoverage {
                from: from.format("%Y-%m-%d").to_string(),
                to: to.format("%Y-%m-%d").to_string(),
            }),
            _ => None,
        };
        Ok((
            rows.into_iter()
                .skip(offset)
                .take(per_page.clamp(1, 200) as usize)
                .collect(),
            total,
            coverage,
        ))
    }

    /// Load a closing quote by stable ISIN and an inclusive date range.
    pub async fn equity_quotes_by_isin(
        query_service: &QueryService,
        isin: &str,
        from: &str,
        to: &str,
        limit: u32,
    ) -> AppResult<Vec<EquityQuoteResponse>> {
        if !valid_isin(isin) {
            return Err(AppError::TickerNotFound {
                ticker: isin.to_string(),
            });
        }
        Self::validate_date(from)?;
        Self::validate_date(to)?;
        if from > to
            || (chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d").unwrap()
                - chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d").unwrap())
            .num_days()
                > 366
        {
            return Err(AppError::DateRangeInvalid);
        }
        let isin = isin.to_ascii_uppercase();
        let sql = format!(
            "SELECT raw_ticker, isin, bdi_code, market_code, date, pre_ult_raw, quotation_factor FROM equities_v1 WHERE isin = '{}' AND market_type = 10 AND bdi_code = '02' AND currency_code = 'BRL' AND (security_specification LIKE 'ON%' OR security_specification LIKE 'PN%' OR security_specification LIKE 'UNT%') AND quotation_factor > 0 AND date >= CAST('{}' AS DATE) AND date <= CAST('{}' AS DATE) ORDER BY date DESC LIMIT {}",
            isin, from, to, limit.clamp(1, 400)
        );
        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(anyhow::Error::new)?;
        let mut rows = Vec::new();
        let retrieved_at = chrono::Utc::now().to_rfc3339();
        for batch in &batches {
            for idx in 0..batch.num_rows() {
                if let Some(quote) = equity_quote_from_batch(batch, idx, &retrieved_at) {
                    rows.push(quote);
                }
            }
        }
        Ok(rows)
    }

    /// Return the latest verified close for an ISIN without a history-range limit.
    pub async fn latest_equity_quote_by_isin(
        query_service: &QueryService,
        isin: &str,
    ) -> AppResult<Option<EquityQuoteResponse>> {
        if !valid_isin(isin) {
            return Err(AppError::TickerNotFound {
                ticker: isin.to_string(),
            });
        }
        if !query_service.has_table("equities_v1") {
            return Ok(None);
        }
        let isin = isin.to_ascii_uppercase();
        let sql = format!(
            "SELECT raw_ticker, isin, bdi_code, market_code, date, pre_ult_raw, quotation_factor FROM equities_v1 WHERE isin = '{}' AND market_type = 10 AND bdi_code = '02' AND currency_code = 'BRL' AND (security_specification LIKE 'ON%' OR security_specification LIKE 'PN%' OR security_specification LIKE 'UNT%') AND quotation_factor > 0 ORDER BY date DESC LIMIT 1",
            isin
        );
        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(anyhow::Error::new)?;
        let Some(batch) = batches.first() else {
            return Ok(None);
        };
        let retrieved_at = chrono::Utc::now().to_rfc3339();
        Ok((batch.num_rows() > 0)
            .then(|| equity_quote_from_batch(batch, 0, &retrieved_at))
            .flatten())
    }

    /// List all tickers with pagination and optional search filter.
    pub fn list_tickers(
        registry: &TickerRegistry,
        pagina: u32,
        por_pagina: u32,
        busca: Option<&str>,
        ativo: bool,
    ) -> AppResult<(Vec<TickerResponse>, usize)> {
        let mut tickers: Vec<TickerResponse> = registry
            .list(ativo)
            .into_iter()
            .filter(|t| {
                if let Some(search) = busca {
                    t.ticker.to_uppercase().starts_with(&search.to_uppercase())
                } else {
                    true
                }
            })
            .map(|t| TickerResponse {
                ticker: t.ticker,
                isin: t.isin,
                company_name: t.company_name,
                first_seen: t.first_seen,
                last_seen: t.last_seen,
                is_active: t.is_active,
            })
            .collect();

        // Sort by ticker name for deterministic output
        tickers.sort_by(|a, b| a.ticker.cmp(&b.ticker));

        let total = tickers.len();
        let offset = ((pagina.max(1) - 1) * por_pagina) as usize;
        let limit = por_pagina as usize;

        let page: Vec<TickerResponse> = tickers.into_iter().skip(offset).take(limit).collect();

        Ok((page, total))
    }

    /// Get detailed info for a single ticker.
    pub async fn get_ticker_detail(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
    ) -> AppResult<TickerDetailResponse> {
        let upper = ticker.to_uppercase();

        // Validate ticker exists
        let info = registry
            .get(&upper)
            .ok_or_else(|| AppError::TickerNotFound {
                ticker: upper.clone(),
            })?;

        if !query_service.has_table("cotahist") {
            return Ok(TickerDetailResponse {
                ticker: info.ticker,
                isin: info.isin,
                company_name: info.company_name,
                first_seen: info.first_seen,
                last_seen: info.last_seen,
                is_active: info.is_active,
                total_trading_days: None,
                latest_price: None,
                latest_date: None,
            });
        }

        // Query latest price and trading days count
        let sql = format!(
            "SELECT COUNT(DISTINCT date) as trading_days, \
                    MAX(date) as latest_date, \
                    (SELECT pre_ult FROM cotahist WHERE ticker = '{upper}' ORDER BY date DESC LIMIT 1) as latest_price \
             FROM cotahist WHERE ticker = '{upper}'"
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let (trading_days, latest_date, latest_price) = Self::extract_ticker_detail(&batches);

        Ok(TickerDetailResponse {
            ticker: info.ticker,
            isin: info.isin,
            company_name: info.company_name,
            first_seen: info.first_seen,
            last_seen: info.last_seen,
            is_active: info.is_active,
            total_trading_days: trading_days,
            latest_price,
            latest_date,
        })
    }

    /// Get quote history for a ticker with filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn get_quotes(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: Option<&str>,
        ate: Option<&str>,
        dias: Option<u32>,
        ordem: &str,
        pagination: &PaginationParams,
    ) -> AppResult<(Vec<QuoteResponse>, usize)> {
        let upper = ticker.to_uppercase();

        // Validate ticker
        if !registry.exists(&upper) {
            return Err(AppError::TickerNotFound { ticker: upper });
        }

        // Validate dates if provided
        if let Some(d) = desde {
            Self::validate_date(d)?;
        }
        if let Some(d) = ate {
            Self::validate_date(d)?;
        }
        if let (Some(d), Some(a)) = (desde, ate) {
            if d > a {
                return Err(AppError::DateRangeInvalid);
            }
        }

        if !query_service.has_table("cotahist") {
            return Ok((Vec::new(), 0));
        }

        // Build base condition
        let mut where_clause = format!("WHERE ticker = '{upper}'");

        // Date filters
        if let Some(d) = desde {
            where_clause.push_str(&format!(" AND date >= '{d}'"));
        }
        if let Some(a) = ate {
            where_clause.push_str(&format!(" AND date <= '{a}'"));
        }

        // Last N trading days
        if let Some(n) = dias {
            where_clause.push_str(&format!(
                " AND date IN (SELECT DISTINCT date FROM cotahist WHERE ticker = '{upper}' ORDER BY date DESC LIMIT {n})"
            ));
        }

        // Count total (before pagination)
        let count_sql = format!("SELECT COUNT(*) FROM cotahist {where_clause}");
        let count_batches = query_service
            .execute_sql(&count_sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
        let total = Self::extract_count(&count_batches);

        // Ordering and pagination
        let order = if ordem.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };

        let sql = format!(
            "SELECT ticker, date, pre_abr, pre_max, pre_min, pre_med, pre_ult, \
                    pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot, \
                    CASE WHEN LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date) IS NULL THEN NULL \
                         ELSE ROUND(((pre_ult - LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date)) \
                         / LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date)) * 100.0, 4) END as var_pct, \
                    CASE WHEN LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date) IS NULL THEN NULL \
                         ELSE ROUND(pre_ult - LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date), 4) END as var_abs \
             FROM cotahist {where_clause} \
             ORDER BY date {order} \
             LIMIT {} OFFSET {}",
            pagination.limit(),
            pagination.offset()
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let quotes = Self::extract_quotes(&batches);

        Ok((quotes, total))
    }

    /// Get the latest quote for a ticker.
    pub async fn get_latest_quote(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
    ) -> AppResult<LatestQuoteResponse> {
        let upper = ticker.to_uppercase();

        if !registry.exists(&upper) {
            return Err(AppError::TickerNotFound { ticker: upper });
        }

        if !query_service.has_table("cotahist") {
            return Err(AppError::NoDataInRange);
        }

        let sql = format!(
            "SELECT ticker, date, pre_abr, pre_max, pre_min, pre_med, pre_ult, \
                    pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot \
             FROM cotahist WHERE ticker = '{upper}' ORDER BY date DESC LIMIT 2"
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let quotes = Self::extract_quotes(&batches);

        if quotes.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let latest = &quotes[0];
        let var_pct = if quotes.len() > 1 {
            let prev = &quotes[1];
            match (latest.pre_ult, prev.pre_ult) {
                (Some(curr), Some(prev_val)) if prev_val != 0.0 => {
                    Some(((curr - prev_val) / prev_val * 100.0 * 10000.0).round() / 10000.0)
                }
                _ => None,
            }
        } else {
            None
        };

        let var_abs = if quotes.len() > 1 {
            match (latest.pre_ult, quotes[1].pre_ult) {
                (Some(curr), Some(prev_val)) => {
                    Some(((curr - prev_val) * 10000.0).round() / 10000.0)
                }
                _ => None,
            }
        } else {
            None
        };

        Ok(LatestQuoteResponse {
            ticker: latest.ticker.clone(),
            data: latest.data.clone(),
            pre_abr: latest.pre_abr,
            pre_max: latest.pre_max,
            pre_min: latest.pre_min,
            pre_med: latest.pre_med,
            pre_ult: latest.pre_ult,
            pre_ofe_compra: latest.pre_ofe_compra,
            pre_ofe_venda: latest.pre_ofe_venda,
            tot_neg: latest.tot_neg,
            quat_tot: latest.quat_tot,
            vol_tot: latest.vol_tot,
            var_pct,
            var_abs,
        })
    }

    /// Get all quotes for a specific date.
    pub async fn get_quotes_by_date(
        query_service: &QueryService,
        date: &str,
        pagination: &PaginationParams,
    ) -> AppResult<(Vec<QuoteResponse>, usize)> {
        Self::validate_date(date)?;

        if !query_service.has_table("cotahist") {
            return Err(AppError::NoDataInRange);
        }

        let count_sql = format!("SELECT COUNT(*) FROM cotahist WHERE date = '{date}'");
        let count_batches = query_service
            .execute_sql(&count_sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
        let total = Self::extract_count(&count_batches);

        if total == 0 {
            return Err(AppError::NoDataInRange);
        }

        let sql = format!(
            "SELECT ticker, date, pre_abr, pre_max, pre_min, pre_med, pre_ult, \
                    pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot, \
                    NULL as var_pct, NULL as var_abs \
             FROM cotahist WHERE date = '{date}' \
             ORDER BY ticker \
             LIMIT {} OFFSET {}",
            pagination.limit(),
            pagination.offset()
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let quotes = Self::extract_quotes(&batches);

        Ok((quotes, total))
    }

    /// Get a compact historical series for charts (date_ms, pre_ult, vol_tot, tot_neg).
    ///
    /// `desde` is required. Results are ordered ascending by date (chart-friendly).
    /// No pagination: the payload is intentionally slim for graph rendering.
    pub async fn get_series(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<SeriesResponse> {
        let upper = ticker.to_uppercase();

        if !registry.exists(&upper) {
            return Err(AppError::TickerNotFound { ticker: upper });
        }

        Self::validate_period(desde, ate)?;

        if !query_service.has_table("cotahist") {
            return Ok(SeriesResponse {
                ticker: upper,
                desde: desde.to_string(),
                ate: ate.map(str::to_string),
                series: Vec::new(),
            });
        }

        let where_clause = Self::period_where(&upper, desde, ate);
        let sql = format!(
            "SELECT date, pre_ult, vol_tot, tot_neg \
             FROM cotahist {where_clause} \
             ORDER BY date ASC"
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let series = Self::extract_series(&batches);

        Ok(SeriesResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            series,
        })
    }

    /// Get the minimum closing price (`pre_ult`) for a ticker in a period.
    pub async fn get_pre_ult_min(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<PreUltExtremeResponse> {
        Self::get_pre_ult_extreme(query_service, registry, ticker, desde, ate, true).await
    }

    /// Get the maximum closing price (`pre_ult`) for a ticker in a period.
    pub async fn get_pre_ult_max(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<PreUltExtremeResponse> {
        Self::get_pre_ult_extreme(query_service, registry, ticker, desde, ate, false).await
    }

    async fn get_pre_ult_extreme(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
        is_min: bool,
    ) -> AppResult<PreUltExtremeResponse> {
        let upper = ticker.to_uppercase();

        if !registry.exists(&upper) {
            return Err(AppError::TickerNotFound { ticker: upper });
        }

        Self::validate_period(desde, ate)?;

        if !query_service.has_table("cotahist") {
            return Err(AppError::NoDataInRange);
        }

        let where_clause = Self::period_where(&upper, desde, ate);
        let order = if is_min { "ASC" } else { "DESC" };
        // Tie-break by earliest date for deterministic results.
        let sql = format!(
            "SELECT date, pre_ult \
             FROM cotahist {where_clause} AND pre_ult IS NOT NULL \
             ORDER BY pre_ult {order}, date ASC \
             LIMIT 1"
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let extreme = Self::extract_pre_ult_extreme(&batches).ok_or(AppError::NoDataInRange)?;

        Ok(PreUltExtremeResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            data: extreme.0,
            pre_ult: extreme.1,
        })
    }

    /// Validate `desde` (required) and optional `ate`, including inverted ranges.
    pub fn validate_period(desde: &str, ate: Option<&str>) -> AppResult<()> {
        Self::validate_date(desde)?;
        if let Some(a) = ate {
            Self::validate_date(a)?;
            if desde > a {
                return Err(AppError::DateRangeInvalid);
            }
        }
        Ok(())
    }

    /// Build a WHERE clause for ticker + period filters.
    pub fn period_where_public(ticker: &str, desde: &str, ate: Option<&str>) -> String {
        Self::period_where(ticker, desde, ate)
    }

    /// Build a WHERE clause for ticker + period filters.
    fn period_where(ticker: &str, desde: &str, ate: Option<&str>) -> String {
        let mut where_clause = format!("WHERE ticker = '{ticker}' AND date >= '{desde}'");
        if let Some(a) = ate {
            where_clause.push_str(&format!(" AND date <= '{a}'"));
        }
        where_clause
    }

    /// Public count extractor for sibling analytics services.
    pub fn extract_count_public(batches: &[RecordBatch]) -> usize {
        Self::extract_count(batches)
    }

    pub fn get_str_public(batch: &RecordBatch, row: usize, col_name: &str) -> Option<String> {
        Self::get_str(batch, row, col_name)
    }

    pub fn get_f64_public(batch: &RecordBatch, row: usize, col_name: &str) -> Option<f64> {
        Self::get_f64(batch, row, col_name)
    }

    pub fn get_u32_public(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u32> {
        Self::get_u32(batch, row, col_name)
    }

    pub fn get_u64_public(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u64> {
        Self::get_u64(batch, row, col_name)
    }

    pub fn get_u8_public(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u8> {
        Self::get_u8(batch, row, col_name)
    }

    /// Validate a date string in YYYY-MM-DD format.
    pub fn validate_date(date: &str) -> AppResult<()> {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| AppError::InvalidDate {
            value: date.to_string(),
        })?;
        Ok(())
    }

    /// Convert a YYYY-MM-DD date to Unix epoch milliseconds at UTC midnight.
    fn date_to_ms(date: &str) -> Option<i64> {
        let naive = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        let datetime = naive.and_hms_opt(0, 0, 0)?;
        Some(datetime.and_utc().timestamp_millis())
    }

    /// Extract compact series points from DataFusion RecordBatches.
    fn extract_series(batches: &[RecordBatch]) -> Vec<SeriesPoint> {
        let mut series = Vec::new();

        for batch in batches {
            for row in 0..batch.num_rows() {
                let Some(data) = Self::get_str(batch, row, "date") else {
                    continue;
                };
                let Some(date_ms) = Self::date_to_ms(&data) else {
                    continue;
                };

                series.push(SeriesPoint {
                    date_ms,
                    pre_ult: Self::get_f64(batch, row, "pre_ult"),
                    vol_tot: Self::get_f64(batch, row, "vol_tot"),
                    tot_neg: Self::get_u32(batch, row, "tot_neg"),
                });
            }
        }

        series
    }

    /// Extract (date, pre_ult) for a min/max extreme query.
    fn extract_pre_ult_extreme(batches: &[RecordBatch]) -> Option<(String, f64)> {
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return None;
        }
        let batch = &batches[0];
        let data = Self::get_str(batch, 0, "date")?;
        let pre_ult = Self::get_f64(batch, 0, "pre_ult")?;
        Some((data, pre_ult))
    }

    /// Extract QuoteResponse records from DataFusion RecordBatches.
    fn extract_quotes(batches: &[RecordBatch]) -> Vec<QuoteResponse> {
        let mut quotes = Vec::new();

        for batch in batches {
            for row in 0..batch.num_rows() {
                let ticker = Self::get_str(batch, row, "ticker").unwrap_or_default();
                let data = Self::get_str(batch, row, "date").unwrap_or_default();

                quotes.push(QuoteResponse {
                    ticker,
                    data,
                    pre_abr: Self::get_f64(batch, row, "pre_abr"),
                    pre_max: Self::get_f64(batch, row, "pre_max"),
                    pre_min: Self::get_f64(batch, row, "pre_min"),
                    pre_med: Self::get_f64(batch, row, "pre_med"),
                    pre_ult: Self::get_f64(batch, row, "pre_ult"),
                    pre_ofe_compra: Self::get_f64(batch, row, "pre_ofe_compra"),
                    pre_ofe_venda: Self::get_f64(batch, row, "pre_ofe_venda"),
                    tot_neg: Self::get_u32(batch, row, "tot_neg"),
                    quat_tot: Self::get_u64(batch, row, "quat_tot"),
                    vol_tot: Self::get_f64(batch, row, "vol_tot"),
                    var_pct: Self::get_f64(batch, row, "var_pct"),
                    var_abs: Self::get_f64(batch, row, "var_abs"),
                });
            }
        }

        quotes
    }

    /// Extract a count from a COUNT(*) query result.
    fn extract_count(batches: &[RecordBatch]) -> usize {
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return 0;
        }
        let batch = &batches[0];
        let col = batch.column(0);
        if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
            arr.value(0) as usize
        } else if let Some(arr) = col.as_any().downcast_ref::<UInt64Array>() {
            arr.value(0) as usize
        } else {
            0
        }
    }

    /// Extract ticker detail from query result.
    fn extract_ticker_detail(
        batches: &[RecordBatch],
    ) -> (Option<u32>, Option<String>, Option<f64>) {
        if batches.is_empty() || batches[0].num_rows() == 0 {
            return (None, None, None);
        }
        let batch = &batches[0];
        let trading_days = Self::get_u32(batch, 0, "trading_days");
        let latest_date = Self::get_str(batch, 0, "latest_date");
        let latest_price = Self::get_f64(batch, 0, "latest_price");
        (trading_days, latest_date, latest_price)
    }

    // === Helper methods for extracting values from Arrow arrays ===

    fn get_str(batch: &RecordBatch, row: usize, col_name: &str) -> Option<String> {
        let schema = batch.schema();
        let idx = schema.index_of(col_name).ok()?;
        let col = batch.column(idx);
        if col.is_null(row) {
            return None;
        }
        if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
            return Some(arr.value(row).to_string());
        }
        if let Some(arr) = col.as_any().downcast_ref::<Date32Array>() {
            let days = arr.value(row);
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)?;
            let date = epoch.checked_add_signed(chrono::Duration::days(days as i64))?;
            return Some(date.format("%Y-%m-%d").to_string());
        }
        None
    }

    fn get_f64(batch: &RecordBatch, row: usize, col_name: &str) -> Option<f64> {
        let schema = batch.schema();
        let idx = schema.index_of(col_name).ok()?;
        let col = batch.column(idx);
        if col.is_null(row) {
            return None;
        }
        col.as_any()
            .downcast_ref::<Float64Array>()
            .map(|arr| arr.value(row))
    }

    fn get_u32(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u32> {
        let schema = batch.schema();
        let idx = schema.index_of(col_name).ok()?;
        let col = batch.column(idx);
        if col.is_null(row) {
            return None;
        }
        if let Some(arr) = col.as_any().downcast_ref::<UInt32Array>() {
            return Some(arr.value(row));
        }
        if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
            return Some(arr.value(row) as u32);
        }
        None
    }

    fn get_u64(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u64> {
        let schema = batch.schema();
        let idx = schema.index_of(col_name).ok()?;
        let col = batch.column(idx);
        if col.is_null(row) {
            return None;
        }
        if let Some(arr) = col.as_any().downcast_ref::<UInt64Array>() {
            return Some(arr.value(row));
        }
        if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
            return Some(arr.value(row) as u64);
        }
        None
    }

    fn get_u8(batch: &RecordBatch, row: usize, col_name: &str) -> Option<u8> {
        let schema = batch.schema();
        let idx = schema.index_of(col_name).ok()?;
        let col = batch.column(idx);
        if col.is_null(row) {
            return None;
        }
        if let Some(arr) = col.as_any().downcast_ref::<UInt8Array>() {
            return Some(arr.value(row));
        }
        if let Some(arr) = col.as_any().downcast_ref::<UInt32Array>() {
            return Some(arr.value(row) as u8);
        }
        if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
            return Some(arr.value(row) as u8);
        }
        None
    }
}

fn string_value(batch: &RecordBatch, column: usize, row: usize) -> String {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<StringArray>()
        .map(|array| array.value(row).to_string())
        .unwrap_or_default()
}

fn nullable_string_value(batch: &RecordBatch, column: usize, row: usize) -> Option<String> {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<StringArray>()
        .and_then(|array| (!array.is_null(row)).then(|| array.value(row).to_string()))
}

fn date32_string(batch: &RecordBatch, column: usize, row: usize) -> String {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<Date32Array>()
        .and_then(|array| {
            chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                .map(|epoch| epoch + chrono::Duration::days(i64::from(array.value(row))))
        })
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn equity_quote_from_batch(
    batch: &RecordBatch,
    row: usize,
    retrieved_at: &str,
) -> Option<EquityQuoteResponse> {
    let date = batch
        .column(4)
        .as_any()
        .downcast_ref::<Date32Array>()?
        .value(row);
    let day =
        chrono::NaiveDate::from_ymd_opt(1970, 1, 1)? + chrono::Duration::days(i64::from(date));
    let raw = batch
        .column(5)
        .as_any()
        .downcast_ref::<UInt64Array>()?
        .value(row);
    let factor = batch
        .column(6)
        .as_any()
        .downcast_ref::<UInt32Array>()?
        .value(row);
    Some(EquityQuoteResponse {
        ticker: string_value(batch, 0, row),
        isin: string_value(batch, 1, row),
        bdi_code: string_value(batch, 2, row),
        market_code: string_value(batch, 3, row),
        data: day.format("%Y-%m-%d").to_string(),
        close_per_share: exact_quote(raw, factor)?,
        quotation_factor: factor,
        source: "tucano-b3 COTAHIST spot".to_string(),
        retrieved_at: retrieved_at.to_string(),
        availability: "available".to_string(),
        adjusted_for_corporate_actions: false,
    })
}

fn valid_isin(value: &str) -> bool {
    if value.len() != 12
        || !value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return false;
    }
    let expanded = value
        .chars()
        .flat_map(|c| {
            if c.is_ascii_digit() {
                vec![c.to_digit(10).unwrap()]
            } else {
                c.to_digit(36)
                    .unwrap()
                    .to_string()
                    .chars()
                    .map(|n| n.to_digit(10).unwrap())
                    .collect()
            }
        })
        .collect::<Vec<_>>();
    let sum = expanded
        .iter()
        .rev()
        .enumerate()
        .map(|(index, digit)| {
            let product = if index % 2 == 1 { digit * 2 } else { *digit };
            product / 10 + product % 10
        })
        .sum::<u32>();
    sum % 10 == 0
}

fn exact_quote(raw: u64, quotation_factor: u32) -> Option<String> {
    if quotation_factor == 0 {
        return None;
    }
    let denominator = u128::from(quotation_factor) * 100;
    let mut residual = denominator;
    let mut twos = 0;
    let mut fives = 0;
    while residual % 2 == 0 {
        residual /= 2;
        twos += 1;
    }
    while residual % 5 == 0 {
        residual /= 5;
        fives += 1;
    }
    if residual != 1 {
        return None;
    }
    let scale = twos.max(fives);
    let scaled = u128::from(raw) * 10u128.pow(scale) / denominator;
    if scale == 0 {
        return Some(scaled.to_string());
    }
    let base = 10u128.pow(scale);
    let fractional = format!("{:0width$}", scaled % base, width = scale as usize);
    let fractional = fractional.trim_end_matches('0');
    if fractional.is_empty() {
        Some((scaled / base).to_string())
    } else {
        Some(format!("{}.{}", scaled / base, fractional))
    }
}

#[cfg(test)]
mod equity_tests {
    use super::{exact_quote, valid_isin};

    #[test]
    fn quote_factor_converts_b3_lot_price_to_exact_per_share_decimal() {
        assert_eq!(exact_quote(1234, 1).as_deref(), Some("12.34"));
        assert_eq!(exact_quote(1_234, 1_000).as_deref(), Some("0.01234"));
        assert_eq!(exact_quote(5_000_000, 1_000).as_deref(), Some("50"));
    }

    #[test]
    fn isin_check_digit_accepts_real_listing_and_rejects_corruption() {
        assert!(valid_isin("BRPETRACNOR9"));
        assert!(!valid_isin("BRPETRACNOR0"));
    }
}
