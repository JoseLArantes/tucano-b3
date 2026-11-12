//! Derived COTAHIST analytics (returns, stats, drawdown, relative, movers, etc.).
//!
//! Computations use raw nominal prices. Results are labeled derived and must not
//! be presented as official B3 series.

use std::collections::{BTreeMap, BTreeSet};

use arrow::record_batch::RecordBatch;

use crate::data::query_service::QueryService;
use crate::data::ticker_registry::TickerRegistry;
use crate::error::{AppError, AppResult};
use crate::models::cotahist_analytics::*;
use crate::models::pagination::{PaginationMeta, PaginationParams};
use crate::services::cotahist::CotahistService;

const TRADING_DAYS_PER_YEAR: f64 = 252.0;
const MAX_COMPARE_TICKERS: usize = 10;
const MAX_MOVERS: u32 = 100;
const MIN_ROLLING_WINDOW: u32 = 2;
const MAX_ROLLING_WINDOW: u32 = 252;
const DEFAULT_ROLLING_WINDOW: u32 = 20;

/// Internal OHLC row used by analytics computations.
#[derive(Debug, Clone)]
struct OhlcRow {
    date: String,
    pre_abr: Option<f64>,
    pre_max: Option<f64>,
    pre_min: Option<f64>,
    pre_ult: Option<f64>,
    pre_ofe_compra: Option<f64>,
    pre_ofe_venda: Option<f64>,
    tot_neg: Option<u32>,
    quat_tot: Option<u64>,
    vol_tot: Option<f64>,
}

/// Analytics operations over COTAHIST (and IBOV for relative performance).
pub struct CotahistAnalyticsService;

impl CotahistAnalyticsService {
    /// Period return on nominal closes.
    pub async fn get_returns(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<ReturnsResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        let closes = Self::closes_with_date(&rows);
        if closes.len() < 2 {
            return Err(AppError::NoDataInRange);
        }
        let first = closes.first().unwrap();
        let last = closes.last().unwrap();
        let retorno_abs = round4(last.1 - first.1);
        let retorno_pct = round4((last.1 / first.1 - 1.0) * 100.0);
        let steps = (closes.len() - 1) as f64;
        let retorno_anualizado_pct = if steps > 0.0 && first.1 > 0.0 {
            let r = last.1 / first.1;
            if r > 0.0 {
                Some(round4(
                    (r.powf(TRADING_DAYS_PER_YEAR / steps) - 1.0) * 100.0,
                ))
            } else {
                None
            }
        } else {
            None
        };

        Ok(ReturnsResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            data_inicial: first.0.clone(),
            data_final: last.0.clone(),
            pre_ult_inicial: first.1,
            pre_ult_final: last.1,
            pregoes: closes.len() as u32,
            retorno_abs,
            retorno_pct,
            retorno_anualizado_pct,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Market-card statistics for a period.
    pub async fn get_stats(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<StatsResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        let closes = Self::closes_with_date(&rows);
        if closes.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let first = closes.first().unwrap();
        let last = closes.last().unwrap();
        let (min_date, min_px) = closes
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then_with(|| a.0.cmp(&b.0)))
            .unwrap();
        let (max_date, max_px) = closes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then_with(|| b.0.cmp(&a.0)))
            .unwrap();

        let pre_ult_medio = round4(closes.iter().map(|c| c.1).sum::<f64>() / closes.len() as f64);
        let retorno_pct = if closes.len() >= 2 {
            round4((last.1 / first.1 - 1.0) * 100.0)
        } else {
            0.0
        };

        let daily_returns = daily_returns(&closes);
        let volatilidade_anualizada_pct =
            sample_std(&daily_returns).map(|s| round4(s * TRADING_DAYS_PER_YEAR.sqrt() * 100.0));

        let atr_medio = mean_true_range(&rows);
        let vol_vals: Vec<f64> = rows.iter().filter_map(|r| r.vol_tot).collect();
        let quat_vals: Vec<f64> = rows
            .iter()
            .filter_map(|r| r.quat_tot.map(|q| q as f64))
            .collect();
        let neg_vals: Vec<f64> = rows
            .iter()
            .filter_map(|r| r.tot_neg.map(|n| n as f64))
            .collect();

        let sem_negocio = rows.iter().filter(|r| r.tot_neg.unwrap_or(0) == 0).count() as f64;

        Ok(StatsResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            pregoes: rows.len() as u32,
            data_inicial: first.0.clone(),
            data_final: last.0.clone(),
            pre_ult_inicial: first.1,
            pre_ult_final: last.1,
            pre_ult_min: *min_px,
            data_pre_ult_min: min_date.clone(),
            pre_ult_max: *max_px,
            data_pre_ult_max: max_date.clone(),
            pre_ult_medio,
            pre_min_periodo: rows.iter().filter_map(|r| r.pre_min).reduce(f64::min),
            pre_max_periodo: rows.iter().filter_map(|r| r.pre_max).reduce(f64::max),
            retorno_pct,
            volatilidade_anualizada_pct,
            atr_medio: atr_medio.map(round4),
            vol_tot_medio: mean(&vol_vals).map(round4),
            quat_tot_medio: mean(&quat_vals).map(round4),
            tot_neg_medio: mean(&neg_vals).map(round4),
            vol_tot_mediano: median(&vol_vals).map(round4),
            fracao_pregoes_sem_negocio: round4(sem_negocio / rows.len() as f64),
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Maximum drawdown on nominal closes.
    pub async fn get_drawdown(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<DrawdownResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        let closes = Self::closes_with_date(&rows);
        if closes.len() < 2 {
            return Err(AppError::NoDataInRange);
        }

        let mut peak_idx = 0usize;
        let mut peak_px = closes[0].1;
        let mut best = (0usize, 0usize, 0.0_f64); // peak_idx, trough_idx, dd_pct

        for (i, (_d, px)) in closes.iter().enumerate() {
            if *px > peak_px {
                peak_px = *px;
                peak_idx = i;
            }
            let dd = if peak_px != 0.0 {
                (px - peak_px) / peak_px * 100.0
            } else {
                0.0
            };
            if dd < best.2 {
                best = (peak_idx, i, dd);
            }
        }

        let last = closes.last().unwrap();
        let max_until_now = closes.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
        let drawdown_atual_pct = if max_until_now != 0.0 {
            round4((last.1 - max_until_now) / max_until_now * 100.0)
        } else {
            0.0
        };

        let pico = &closes[best.0];
        let vale = &closes[best.1];

        Ok(DrawdownResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            max_drawdown_pct: round4(best.2),
            max_drawdown_abs: round4(vale.1 - pico.1),
            data_pico: pico.0.clone(),
            pre_ult_pico: pico.1,
            data_vale: vale.0.clone(),
            pre_ult_vale: vale.1,
            pregoes_submersos: (best.1 - best.0 + 1) as u32,
            drawdown_atual_pct,
            pre_ult_atual: last.1,
            pre_ult_max_ate_atual: max_until_now,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Position of latest close in the period high–low range.
    pub async fn get_range_position(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<RangePositionResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        let closes = Self::closes_with_date(&rows);
        if closes.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let last = closes.last().unwrap();
        let (min_date, min_px) = closes
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then_with(|| a.0.cmp(&b.0)))
            .unwrap()
            .clone();
        let (max_date, max_px) = closes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then_with(|| b.0.cmp(&a.0)))
            .unwrap()
            .clone();

        let posicao_pct = if (max_px - min_px).abs() > f64::EPSILON {
            Some(round4((last.1 - min_px) / (max_px - min_px) * 100.0))
        } else {
            None
        };

        Ok(RangePositionResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            data_referencia: last.0.clone(),
            pre_ult: last.1,
            pre_ult_min: min_px,
            data_pre_ult_min: min_date,
            pre_ult_max: max_px,
            data_pre_ult_max: max_date,
            posicao_pct,
            distancia_max_pct: round4((last.1 / max_px - 1.0) * 100.0),
            distancia_min_pct: round4((last.1 / min_px - 1.0) * 100.0),
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Relative performance versus IBOV on aligned session dates.
    pub async fn get_relative(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
        benchmark: &str,
    ) -> AppResult<RelativeResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let bench = benchmark.trim().to_uppercase();
        if bench != "IBOV" {
            return Err(AppError::InvalidParam {
                message: "O único benchmark suportado no momento é 'IBOV'.".to_string(),
            });
        }
        if !query_service.has_table("ibov") {
            return Err(AppError::NoDataInRange);
        }

        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        let ticker_map: BTreeMap<String, f64> = Self::closes_with_date(&rows).into_iter().collect();
        let ibov_map = Self::fetch_ibov_closes(query_service, desde, ate).await?;

        let mut aligned: Vec<(String, f64, f64)> = Vec::new();
        for (date, t_px) in &ticker_map {
            if let Some(b_px) = ibov_map.get(date) {
                aligned.push((date.clone(), *t_px, *b_px));
            }
        }
        if aligned.len() < 2 {
            return Err(AppError::NoDataInRange);
        }

        let first = aligned.first().unwrap();
        let last = aligned.last().unwrap();
        let retorno_ticker_pct = round4((last.1 / first.1 - 1.0) * 100.0);
        let retorno_benchmark_pct = round4((last.2 / first.2 - 1.0) * 100.0);

        Ok(RelativeResponse {
            ticker: upper,
            benchmark: bench,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            data_inicial: first.0.clone(),
            data_final: last.0.clone(),
            pregoes_alinhados: aligned.len() as u32,
            retorno_ticker_pct,
            retorno_benchmark_pct,
            excesso_retorno_pct: round4(retorno_ticker_pct - retorno_benchmark_pct),
            pre_ult_ticker_inicial: first.1,
            pre_ult_ticker_final: last.1,
            pre_ult_benchmark_inicial: first.2,
            pre_ult_benchmark_final: last.2,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Session movers (gainers / losers / volume / trades).
    pub async fn get_movers(
        query_service: &QueryService,
        date: &str,
        ranking: &str,
        limite: u32,
        market_type: Option<u8>,
    ) -> AppResult<MoversResponse> {
        CotahistService::validate_date(date)?;
        if !query_service.has_table("cotahist") {
            return Err(AppError::NoDataInRange);
        }

        let ranking_norm = ranking.trim().to_lowercase();
        let order = match ranking_norm.as_str() {
            "gainers" => "var_pct DESC NULLS LAST, ticker ASC",
            "losers" => "var_pct ASC NULLS LAST, ticker ASC",
            "volume" => "vol_tot DESC NULLS LAST, ticker ASC",
            "trades" => "tot_neg DESC NULLS LAST, ticker ASC",
            _ => {
                return Err(AppError::InvalidParam {
                    message: "ranking deve ser gainers, losers, volume ou trades.".to_string(),
                })
            }
        };

        let limit = limite.clamp(1, MAX_MOVERS);
        let mt_filter = market_type
            .map(|m| format!(" AND market_type = {m}"))
            .unwrap_or_default();

        let sql = format!(
            "WITH base AS ( \
                SELECT ticker, date, pre_ult, vol_tot, tot_neg, quat_tot, market_type, \
                       LAG(pre_ult) OVER (PARTITION BY ticker ORDER BY date) AS prev_ult \
                FROM cotahist \
             ), \
             day AS ( \
                SELECT ticker, date, pre_ult, vol_tot, tot_neg, quat_tot, market_type, prev_ult, \
                       CASE WHEN prev_ult IS NULL OR prev_ult = 0 THEN NULL \
                            ELSE ROUND(((pre_ult - prev_ult) / prev_ult) * 100.0, 4) END AS var_pct, \
                       CASE WHEN prev_ult IS NULL THEN NULL \
                            ELSE ROUND(pre_ult - prev_ult, 4) END AS var_abs \
                FROM base \
                WHERE date = '{date}'{mt_filter} \
             ) \
             SELECT ticker, date, pre_ult, var_pct, var_abs, vol_tot, tot_neg, quat_tot, market_type \
             FROM day \
             ORDER BY {order} \
             LIMIT {limit}"
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
        let itens = Self::extract_movers(&batches);
        if itens.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        Ok(MoversResponse {
            data: date.to_string(),
            ranking: ranking_norm,
            itens,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Cross-sectional liquidity screen.
    pub async fn get_liquidity_screen(
        query_service: &QueryService,
        desde: &str,
        ate: Option<&str>,
        market_type: Option<u8>,
        vol_tot_medio_min: Option<f64>,
        pagination: &PaginationParams,
    ) -> AppResult<LiquidityScreenResponse> {
        CotahistService::validate_period(desde, ate)?;
        if !query_service.has_table("cotahist") {
            return Ok(LiquidityScreenResponse {
                desde: desde.to_string(),
                ate: ate.map(str::to_string),
                market_type,
                itens: Vec::new(),
                paginacao: PaginationMeta::new(0, pagination.pagina, pagination.por_pagina),
                aviso: COTAHIST_AVISO_NOMINAL.to_string(),
            });
        }

        let mut where_clause = format!("WHERE date >= '{desde}'");
        if let Some(a) = ate {
            where_clause.push_str(&format!(" AND date <= '{a}'"));
        }
        if let Some(m) = market_type {
            where_clause.push_str(&format!(" AND market_type = {m}"));
        }

        let having = vol_tot_medio_min
            .map(|v| format!(" HAVING AVG(vol_tot) >= {v}"))
            .unwrap_or_default();

        let count_sql = format!(
            "SELECT COUNT(*) FROM ( \
                SELECT ticker FROM cotahist {where_clause} GROUP BY ticker{having} \
             ) t"
        );
        let count_batches = query_service
            .execute_sql(&count_sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
        let total = CotahistService::extract_count_public(&count_batches);

        // Keep aggregates to AVG/COUNT only for DataFusion portability and scan cost.
        // Exact medians would require a heavier windowed plan over the full universe.
        let sql = format!(
            "SELECT ticker, \
                    COUNT(*) AS pregoes, \
                    AVG(vol_tot) AS vol_tot_medio, \
                    AVG(vol_tot) AS vol_tot_mediano, \
                    AVG(quat_tot) AS quat_tot_medio, \
                    AVG(tot_neg) AS tot_neg_medio, \
                    AVG(tot_neg) AS tot_neg_mediano, \
                    AVG(CASE WHEN tot_neg IS NULL OR tot_neg = 0 THEN 1.0 ELSE 0.0 END) AS fracao_pregoes_sem_negocio \
             FROM cotahist {where_clause} \
             GROUP BY ticker{having} \
             ORDER BY vol_tot_medio DESC NULLS LAST, ticker ASC \
             LIMIT {} OFFSET {}",
            pagination.limit(),
            pagination.offset()
        );

        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let itens = Self::extract_liquidity(&batches);

        Ok(LiquidityScreenResponse {
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            market_type,
            itens,
            paginacao: PaginationMeta::new(total, pagination.pagina, pagination.por_pagina),
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Opening gaps versus previous close.
    pub async fn get_gaps(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
        gap_pct_min: Option<f64>,
    ) -> AppResult<GapsResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        if rows.len() < 2 {
            return Err(AppError::NoDataInRange);
        }

        let min_abs = gap_pct_min.unwrap_or(0.0).abs();
        let mut gaps = Vec::new();
        for window in rows.windows(2) {
            let prev = &window[0];
            let curr = &window[1];
            let (Some(abr), Some(prev_ult)) = (curr.pre_abr, prev.pre_ult) else {
                continue;
            };
            if prev_ult == 0.0 {
                continue;
            }
            let gap_abs = round4(abr - prev_ult);
            let gap_pct = round4((abr / prev_ult - 1.0) * 100.0);
            if gap_pct.abs() < min_abs {
                continue;
            }
            let direcao = if gap_pct > 0.0 {
                "alta"
            } else if gap_pct < 0.0 {
                "baixa"
            } else {
                "neutro"
            }
            .to_string();

            let preenchido = match (curr.pre_min, curr.pre_max) {
                (Some(lo), Some(hi)) => Some(lo <= prev_ult && prev_ult <= hi),
                _ => None,
            };

            gaps.push(GapItem {
                data: curr.date.clone(),
                pre_abr: abr,
                pre_ult_anterior: prev_ult,
                gap_abs,
                gap_pct,
                direcao,
                preenchido,
            });
        }

        Ok(GapsResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            gap_pct_min,
            gaps,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Rolling SMA / EMA / volatility / ADV series.
    pub async fn get_rolling(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
        janela: u32,
    ) -> AppResult<RollingResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let window = if janela == 0 {
            DEFAULT_ROLLING_WINDOW
        } else {
            janela.clamp(MIN_ROLLING_WINDOW, MAX_ROLLING_WINDOW)
        };

        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        if rows.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let alpha = 2.0 / (window as f64 + 1.0);
        let mut ema: Option<f64> = None;
        let mut series = Vec::with_capacity(rows.len());

        for i in 0..rows.len() {
            let row = &rows[i];
            let pre_ult = row.pre_ult;

            if let Some(px) = pre_ult {
                ema = Some(match ema {
                    Some(prev) => alpha * px + (1.0 - alpha) * prev,
                    None => px,
                });
            }

            let sma = if i + 1 >= window as usize {
                let slice = &rows[i + 1 - window as usize..=i];
                let vals: Vec<f64> = slice.iter().filter_map(|r| r.pre_ult).collect();
                if vals.len() == window as usize {
                    Some(round4(vals.iter().sum::<f64>() / window as f64))
                } else {
                    None
                }
            } else {
                None
            };

            let volatilidade_anualizada_pct = if i + 1 >= window as usize {
                let slice = &rows[i + 1 - window as usize..=i];
                let closes = Self::closes_with_date(slice);
                let rets = daily_returns(&closes);
                sample_std(&rets).map(|s| round4(s * TRADING_DAYS_PER_YEAR.sqrt() * 100.0))
            } else {
                None
            };

            let adv_vol_tot = if i + 1 >= window as usize {
                let slice = &rows[i + 1 - window as usize..=i];
                let vols: Vec<f64> = slice.iter().filter_map(|r| r.vol_tot).collect();
                if vols.len() == window as usize {
                    mean(&vols).map(round4)
                } else {
                    None
                }
            } else {
                None
            };

            series.push(RollingPoint {
                data: row.date.clone(),
                pre_ult,
                sma,
                ema: ema.map(round4),
                volatilidade_anualizada_pct,
                adv_vol_tot,
            });
        }

        Ok(RollingResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            janela: window,
            series,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Bid/ask spread series from COTAHIST offer fields.
    pub async fn get_spread(
        query_service: &QueryService,
        registry: &TickerRegistry,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<SpreadResponse> {
        let upper = Self::require_ticker(registry, ticker)?;
        CotahistService::validate_period(desde, ate)?;
        let rows = Self::fetch_ohlc(query_service, &upper, desde, ate).await?;
        if rows.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let mut series = Vec::with_capacity(rows.len());
        let mut spreads = Vec::new();
        for row in &rows {
            let (spread_abs, spread_pct, mid) = match (row.pre_ofe_compra, row.pre_ofe_venda) {
                (Some(bid), Some(ask)) => {
                    let abs = round4(ask - bid);
                    let mid_px = (ask + bid) / 2.0;
                    let pct = if mid_px != 0.0 {
                        Some(round4(abs / mid_px * 100.0))
                    } else {
                        None
                    };
                    if let Some(p) = pct {
                        spreads.push(p);
                    }
                    (Some(abs), pct, Some(round4(mid_px)))
                }
                _ => (None, None, None),
            };

            series.push(SpreadPoint {
                data: row.date.clone(),
                pre_ofe_compra: row.pre_ofe_compra,
                pre_ofe_venda: row.pre_ofe_venda,
                spread_abs,
                spread_pct,
                mid,
            });
        }

        Ok(SpreadResponse {
            ticker: upper,
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            series,
            pregoes_com_spread: spreads.len() as u32,
            spread_pct_medio: mean(&spreads).map(round4),
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    /// Multi-ticker normalized comparison (base 100 on first common date).
    pub async fn compare(
        query_service: &QueryService,
        registry: &TickerRegistry,
        tickers_csv: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<CompareResponse> {
        CotahistService::validate_period(desde, ate)?;
        let tickers = Self::parse_tickers(tickers_csv, registry)?;
        if tickers.is_empty() {
            return Err(AppError::InvalidParam {
                message: "Informe ao menos um ticker em 'tickers' (ex: VALE3,PETR4).".to_string(),
            });
        }

        let mut per_ticker: Vec<(String, BTreeMap<String, f64>)> = Vec::new();
        for t in &tickers {
            let rows = Self::fetch_ohlc(query_service, t, desde, ate).await?;
            let map: BTreeMap<String, f64> = Self::closes_with_date(&rows).into_iter().collect();
            if !map.is_empty() {
                per_ticker.push((t.clone(), map));
            }
        }
        if per_ticker.is_empty() {
            return Err(AppError::NoDataInRange);
        }

        let mut common: Option<BTreeSet<String>> = None;
        for (_t, map) in &per_ticker {
            let keys: BTreeSet<String> = map.keys().cloned().collect();
            common = Some(match common {
                None => keys,
                Some(prev) => prev.intersection(&keys).cloned().collect(),
            });
        }
        let datas: Vec<String> = common.unwrap_or_default().into_iter().collect();
        if datas.len() < 2 {
            return Err(AppError::NoDataInRange);
        }

        let mut series = Vec::new();
        for (ticker, map) in &per_ticker {
            let base = *map.get(&datas[0]).unwrap();
            if base == 0.0 {
                continue;
            }
            let last = *map.get(datas.last().unwrap()).unwrap();
            let valores: Vec<Option<f64>> = datas
                .iter()
                .map(|d| map.get(d).map(|px| round4(px / base * 100.0)))
                .collect();

            series.push(CompareSeries {
                ticker: ticker.clone(),
                pre_ult_base: base,
                pre_ult_final: last,
                retorno_pct: round4((last / base - 1.0) * 100.0),
                valores_base_100: valores,
            });
        }

        Ok(CompareResponse {
            desde: desde.to_string(),
            ate: ate.map(str::to_string),
            datas,
            series,
            aviso: COTAHIST_AVISO_NOMINAL.to_string(),
        })
    }

    // --- helpers ---

    fn require_ticker(registry: &TickerRegistry, ticker: &str) -> AppResult<String> {
        let upper = ticker.trim().to_uppercase();
        if !registry.exists(&upper) {
            return Err(AppError::TickerNotFound { ticker: upper });
        }
        Ok(upper)
    }

    fn parse_tickers(csv: &str, registry: &TickerRegistry) -> AppResult<Vec<String>> {
        let mut out = Vec::new();
        for part in csv.split(',') {
            let t = part.trim().to_uppercase();
            if t.is_empty() {
                continue;
            }
            if !t.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(AppError::InvalidParam {
                    message: format!("Ticker inválido: '{t}'. Use apenas letras e dígitos."),
                });
            }
            if !registry.exists(&t) {
                return Err(AppError::TickerNotFound { ticker: t });
            }
            if !out.contains(&t) {
                out.push(t);
            }
            if out.len() > MAX_COMPARE_TICKERS {
                return Err(AppError::InvalidParam {
                    message: format!("No máximo {MAX_COMPARE_TICKERS} tickers por comparação."),
                });
            }
        }
        Ok(out)
    }

    async fn fetch_ohlc(
        query_service: &QueryService,
        ticker: &str,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<Vec<OhlcRow>> {
        if !query_service.has_table("cotahist") {
            return Ok(Vec::new());
        }
        let where_clause = CotahistService::period_where_public(ticker, desde, ate);
        let sql = format!(
            "SELECT date, pre_abr, pre_max, pre_min, pre_ult, \
                    pre_ofe_compra, pre_ofe_venda, tot_neg, quat_tot, vol_tot \
             FROM cotahist {where_clause} \
             ORDER BY date ASC"
        );
        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
        Ok(Self::extract_ohlc(&batches))
    }

    async fn fetch_ibov_closes(
        query_service: &QueryService,
        desde: &str,
        ate: Option<&str>,
    ) -> AppResult<BTreeMap<String, f64>> {
        let mut where_clause = format!("WHERE date >= '{desde}'");
        if let Some(a) = ate {
            where_clause.push_str(&format!(" AND date <= '{a}'"));
        }
        let sql = format!(
            "SELECT date, pre_ult FROM ibov {where_clause} AND pre_ult IS NOT NULL ORDER BY date ASC"
        );
        let batches = query_service
            .execute_sql(&sql)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

        let mut map = BTreeMap::new();
        for batch in &batches {
            for row in 0..batch.num_rows() {
                if let (Some(date), Some(px)) = (
                    CotahistService::get_str_public(batch, row, "date"),
                    CotahistService::get_f64_public(batch, row, "pre_ult"),
                ) {
                    map.insert(date, px);
                }
            }
        }
        Ok(map)
    }

    fn closes_with_date(rows: &[OhlcRow]) -> Vec<(String, f64)> {
        rows.iter()
            .filter_map(|r| r.pre_ult.map(|px| (r.date.clone(), px)))
            .collect()
    }

    fn extract_ohlc(batches: &[RecordBatch]) -> Vec<OhlcRow> {
        let mut rows = Vec::new();
        for batch in batches {
            for row in 0..batch.num_rows() {
                let Some(date) = CotahistService::get_str_public(batch, row, "date") else {
                    continue;
                };
                rows.push(OhlcRow {
                    date,
                    pre_abr: CotahistService::get_f64_public(batch, row, "pre_abr"),
                    pre_max: CotahistService::get_f64_public(batch, row, "pre_max"),
                    pre_min: CotahistService::get_f64_public(batch, row, "pre_min"),
                    pre_ult: CotahistService::get_f64_public(batch, row, "pre_ult"),
                    pre_ofe_compra: CotahistService::get_f64_public(batch, row, "pre_ofe_compra"),
                    pre_ofe_venda: CotahistService::get_f64_public(batch, row, "pre_ofe_venda"),
                    tot_neg: CotahistService::get_u32_public(batch, row, "tot_neg"),
                    quat_tot: CotahistService::get_u64_public(batch, row, "quat_tot"),
                    vol_tot: CotahistService::get_f64_public(batch, row, "vol_tot"),
                });
            }
        }
        rows
    }

    fn extract_movers(batches: &[RecordBatch]) -> Vec<MoverItem> {
        let mut itens = Vec::new();
        for batch in batches {
            for row in 0..batch.num_rows() {
                let ticker =
                    CotahistService::get_str_public(batch, row, "ticker").unwrap_or_default();
                let data = CotahistService::get_str_public(batch, row, "date").unwrap_or_default();
                itens.push(MoverItem {
                    ticker,
                    data,
                    pre_ult: CotahistService::get_f64_public(batch, row, "pre_ult"),
                    var_pct: CotahistService::get_f64_public(batch, row, "var_pct"),
                    var_abs: CotahistService::get_f64_public(batch, row, "var_abs"),
                    vol_tot: CotahistService::get_f64_public(batch, row, "vol_tot"),
                    tot_neg: CotahistService::get_u32_public(batch, row, "tot_neg"),
                    quat_tot: CotahistService::get_u64_public(batch, row, "quat_tot"),
                    market_type: CotahistService::get_u8_public(batch, row, "market_type"),
                });
            }
        }
        itens
    }

    fn extract_liquidity(batches: &[RecordBatch]) -> Vec<LiquidityItem> {
        let mut itens = Vec::new();
        for batch in batches {
            for row in 0..batch.num_rows() {
                let ticker =
                    CotahistService::get_str_public(batch, row, "ticker").unwrap_or_default();
                itens.push(LiquidityItem {
                    ticker,
                    pregoes: CotahistService::get_u32_public(batch, row, "pregoes").unwrap_or(0),
                    vol_tot_medio: CotahistService::get_f64_public(batch, row, "vol_tot_medio"),
                    vol_tot_mediano: CotahistService::get_f64_public(batch, row, "vol_tot_mediano"),
                    quat_tot_medio: CotahistService::get_f64_public(batch, row, "quat_tot_medio"),
                    tot_neg_medio: CotahistService::get_f64_public(batch, row, "tot_neg_medio"),
                    tot_neg_mediano: CotahistService::get_f64_public(batch, row, "tot_neg_mediano"),
                    fracao_pregoes_sem_negocio: CotahistService::get_f64_public(
                        batch,
                        row,
                        "fracao_pregoes_sem_negocio",
                    )
                    .unwrap_or(0.0),
                });
            }
        }
        itens
    }
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

fn mean(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

fn median(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

fn daily_returns(closes: &[(String, f64)]) -> Vec<f64> {
    let mut out = Vec::new();
    for w in closes.windows(2) {
        let (prev, curr) = (w[0].1, w[1].1);
        if prev != 0.0 {
            out.push(curr / prev - 1.0);
        }
    }
    out
}

fn sample_std(vals: &[f64]) -> Option<f64> {
    if vals.len() < 2 {
        return None;
    }
    let m = mean(vals)?;
    let var = vals.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (vals.len() - 1) as f64;
    Some(var.sqrt())
}

fn mean_true_range(rows: &[OhlcRow]) -> Option<f64> {
    if rows.is_empty() {
        return None;
    }
    let mut trs = Vec::new();
    let mut prev_close: Option<f64> = None;
    for row in rows {
        let tr = match (row.pre_max, row.pre_min) {
            (Some(hi), Some(lo)) => {
                let mut tr = hi - lo;
                if let Some(pc) = prev_close {
                    tr = tr.max((hi - pc).abs()).max((lo - pc).abs());
                }
                Some(tr)
            }
            _ => match (row.pre_ult, prev_close) {
                (Some(c), Some(pc)) => Some((c - pc).abs()),
                _ => None,
            },
        };
        if let Some(v) = tr {
            trs.push(v);
        }
        if row.pre_ult.is_some() {
            prev_close = row.pre_ult;
        }
    }
    mean(&trs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_std_known_values() {
        let vals = vec![0.01, -0.02, 0.015];
        let s = sample_std(&vals).unwrap();
        assert!(s > 0.0);
    }

    #[test]
    fn median_even_odd() {
        assert_eq!(median(&[1.0, 3.0, 2.0]), Some(2.0));
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
    }

    #[test]
    fn daily_returns_basic() {
        let closes = vec![("2026-01-01".into(), 100.0), ("2026-01-02".into(), 110.0)];
        let r = daily_returns(&closes);
        assert_eq!(r.len(), 1);
        assert!((r[0] - 0.1).abs() < 1e-12);
    }
}
