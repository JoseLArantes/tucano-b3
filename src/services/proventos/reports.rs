//! Reports over the data-com of registered cash distributions.

use super::{format_micros, normalize_ticker_filter, parse_iso_date, parse_tickers};
use super::{ProventosError, ProventosService};
use crate::models::proventos::{
    ProventosPeriodQuery, ProventosPeriodTotal, ProventosSeriesPoint, ProventosTickerSeries,
    ProventosYearlyQuery, ProventosYearlyResponse,
};
use chrono::{Datelike, NaiveDate, NaiveTime};
use std::collections::HashMap;

impl ProventosService {
    /// Return one ticker's total for a calendar year of data-com.
    pub fn yearly(
        &self,
        query: ProventosYearlyQuery,
    ) -> Result<ProventosYearlyResponse, ProventosError> {
        let ticker = normalize_ticker_filter(&query.ticker)?;
        let ano = query.ano;
        let (Some(desde), Some(ate)) = (
            NaiveDate::from_ymd_opt(ano, 1, 1).filter(|_| (1..=9999).contains(&ano)),
            NaiveDate::from_ymd_opt(ano, 12, 31),
        ) else {
            return Err(ProventosError::Invalid(
                "ano deve estar entre 1 e 9999".into(),
            ));
        };
        let total = self
            .ledger
            .proventos_period_totals(std::slice::from_ref(&ticker), desde, ate)?
            .first()
            .map_or(0, |(_, total)| *total);
        Ok(ProventosYearlyResponse {
            ticker,
            ano,
            proventos_on_year: format_micros(total),
        })
    }

    /// Return a total for every requested ticker, in first occurrence order.
    pub fn period(
        &self,
        query: ProventosPeriodQuery,
    ) -> Result<Vec<ProventosPeriodTotal>, ProventosError> {
        let (tickers, desde, ate) = report_selection(&query)?;
        let totals: HashMap<_, _> = self
            .ledger
            .proventos_period_totals(&tickers, desde, ate)?
            .into_iter()
            .collect();
        Ok(tickers
            .into_iter()
            .map(|ticker| ProventosPeriodTotal {
                proventos_on_period: format_micros(
                    totals.get(&ticker).copied().unwrap_or_default(),
                ),
                ticker,
            })
            .collect())
    }

    /// Return compact daily totals per ticker, omitting dates without records.
    pub fn series(
        &self,
        query: ProventosPeriodQuery,
    ) -> Result<Vec<ProventosTickerSeries>, ProventosError> {
        let (tickers, desde, ate) = report_selection(&query)?;
        let mut series = HashMap::<_, Vec<_>>::new();
        for (ticker, date, total) in self.ledger.proventos_daily_totals(&tickers, desde, ate)? {
            series
                .entry(ticker)
                .or_default()
                .push(ProventosSeriesPoint {
                    date_ms: date.and_time(NaiveTime::MIN).and_utc().timestamp_millis(),
                    valor: format_micros(total),
                });
        }
        Ok(tickers
            .into_iter()
            .map(|ticker| ProventosTickerSeries {
                series: series.remove(&ticker).unwrap_or_default(),
                ticker,
            })
            .collect())
    }
}

fn report_selection(
    query: &ProventosPeriodQuery,
) -> Result<(Vec<String>, NaiveDate, NaiveDate), ProventosError> {
    let tickers = parse_tickers(query.ticker.as_deref(), query.tickers.as_deref())?;
    if tickers.is_empty() {
        return Err(ProventosError::Invalid("informe ticker ou tickers".into()));
    }
    let desde = parse_iso_date(&query.desde, "desde")?;
    let ate = parse_iso_date(&query.ate, "ate")?;
    if !(1..=9999).contains(&desde.year()) || !(1..=9999).contains(&ate.year()) {
        return Err(ProventosError::Invalid(
            "as datas devem usar um ano entre 1 e 9999".into(),
        ));
    }
    if desde > ate {
        return Err(ProventosError::Invalid(
            "desde deve ser igual ou anterior a ate".into(),
        ));
    }
    Ok((tickers, desde, ate))
}
