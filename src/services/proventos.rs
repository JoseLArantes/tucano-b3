//! Persistence and validation for source-attributed cash distributions.

mod reports;

use crate::ledger::{Ledger, LedgerError, ProventoDraft, ProventoEntry, ProventoFilters};
use crate::models::proventos::{
    ProventoDeleteResponse, ProventoRequest, ProventoResponse, ProventoTipo, ProventosListResponse,
    ProventosQuery,
};
use chrono::{NaiveDate, Utc};
use std::sync::Arc;
use thiserror::Error;

const INITIAL_PROVENTOS: &str = include_str!("proventos-2026-11.json");
const INITIAL_PROVENTOS_SEED_ID: &str = "proventos-2026-11-v1";
const MAX_VALOR_MICROS: i64 = 99_999_999_999_999;

/// Provento validation or persistence failure.
#[derive(Debug, Error)]
pub enum ProventosError {
    #[error("Falha no ledger de proventos: {0}")]
    Ledger(#[from] LedgerError),
    #[error("Provento não encontrado: {id}")]
    NotFound { id: i64 },
    #[error("Provento inválido: {0}")]
    Invalid(String),
    #[error("Não foi possível ler a carga inicial de proventos: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Service for provento CRUD backed by the embedded SQLite ledger.
pub struct ProventosService {
    ledger: Arc<Ledger>,
}

impl ProventosService {
    /// Apply the initial November 2026 proventos records on first use of a ledger.
    pub fn new(ledger: Arc<Ledger>) -> Result<Self, ProventosError> {
        let service = Self { ledger };
        let seed: Vec<ProventoRequest> = serde_json::from_str(INITIAL_PROVENTOS)?;
        let records = seed
            .into_iter()
            .map(validate_record)
            .collect::<Result<Vec<_>, _>>()?;
        service.ledger.initialize_proventos_seed(
            INITIAL_PROVENTOS_SEED_ID,
            &records,
            &Utc::now().to_rfc3339(),
        )?;
        Ok(service)
    }

    /// List proventos in payment-date order, with optional filters and pagination.
    pub fn list(&self, query: ProventosQuery) -> Result<ProventosListResponse, ProventosError> {
        let filters = validate_filters(&query)?;
        let (pagina, por_pagina) = query.pagination();
        let (entries, total) = self
            .ledger
            .list_proventos(&filters, por_pagina, query.offset())?;
        Ok(ProventosListResponse {
            proventos: entries
                .into_iter()
                .map(to_response)
                .collect::<Result<Vec<_>, _>>()?,
            paginacao: crate::models::pagination::PaginationMeta::new(total, pagina, por_pagina),
        })
    }

    /// Return one provento by stable identifier.
    pub fn get(&self, id: i64) -> Result<ProventoResponse, ProventosError> {
        self.ledger
            .get_provento(id)?
            .ok_or(ProventosError::NotFound { id })
            .and_then(to_response)
    }

    /// Create a provento.
    pub fn create(&self, request: ProventoRequest) -> Result<ProventoResponse, ProventosError> {
        let record = validate_record(request)?;
        let entry = self
            .ledger
            .create_provento(&record, &Utc::now().to_rfc3339())?;
        to_response(entry)
    }

    /// Replace all business fields for an existing provento.
    pub fn update(
        &self,
        id: i64,
        request: ProventoRequest,
    ) -> Result<ProventoResponse, ProventosError> {
        let record = validate_record(request)?;
        self.ledger
            .update_provento(id, &record, &Utc::now().to_rfc3339())?
            .ok_or(ProventosError::NotFound { id })
            .and_then(to_response)
    }

    /// Delete a provento.
    pub fn delete(&self, id: i64) -> Result<ProventoDeleteResponse, ProventosError> {
        if self.ledger.delete_provento(id)? {
            Ok(ProventoDeleteResponse { id, removido: true })
        } else {
            Err(ProventosError::NotFound { id })
        }
    }
}

fn validate_record(mut request: ProventoRequest) -> Result<ProventoDraft, ProventosError> {
    let ticker = request.ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || ticker.len() > 10
        || !ticker.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(ProventosError::Invalid(
            "o ticker deve conter de 1 a 10 caracteres alfanuméricos".to_string(),
        ));
    }

    let valor_micros = parse_valor(&request.valor)?;
    let data_com = validate_date(request.data_com.as_deref(), "data_com")?;
    let data_pagamento = validate_date(request.data_pagamento.as_deref(), "data_pagamento")?;
    if let (Some(data_com), Some(data_pagamento)) = (&data_com, &data_pagamento) {
        if data_com > data_pagamento {
            return Err(ProventosError::Invalid(
                "a data_com não pode ser posterior à data_pagamento".to_string(),
            ));
        }
    }

    request.obs = request.obs.map(|obs| obs.trim().to_string());
    if request
        .obs
        .as_ref()
        .is_some_and(|obs| obs.chars().count() > 255)
    {
        return Err(ProventosError::Invalid(
            "obs não pode exceder 255 caracteres".to_string(),
        ));
    }

    Ok(ProventoDraft {
        ticker,
        tipo: request.tipo.as_str().to_string(),
        valor_micros,
        data_com,
        data_pagamento,
        obs: request.obs.filter(|obs| !obs.is_empty()),
    })
}

fn validate_date(value: Option<&str>, field: &str) -> Result<Option<String>, ProventosError> {
    value
        .map(|value| parse_iso_date(value, field).map(|_| value.to_string()))
        .transpose()
}

fn parse_iso_date(value: &str, field: &str) -> Result<NaiveDate, ProventosError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        ProventosError::Invalid(format!(
            "{field} deve usar uma data válida no formato AAAA-MM-DD"
        ))
    })?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(ProventosError::Invalid(format!(
            "{field} deve usar o formato ISO AAAA-MM-DD"
        )));
    }
    Ok(date)
}

fn parse_tickers(
    ticker: Option<&str>,
    tickers: Option<&str>,
) -> Result<Vec<String>, ProventosError> {
    if ticker.is_some() && tickers.is_some() {
        return Err(ProventosError::Invalid(
            "informe ticker ou tickers, sem combinar os dois parâmetros".to_string(),
        ));
    }
    if let Some(ticker) = ticker {
        Ok(vec![normalize_ticker_filter(ticker)?])
    } else if let Some(tickers) = tickers {
        let requested = tickers.split(',').collect::<Vec<_>>();
        if requested.len() > 100 {
            return Err(ProventosError::Invalid(
                "o filtro tickers aceita no máximo 100 códigos".to_string(),
            ));
        }
        let mut normalized = Vec::with_capacity(requested.len());
        for ticker in requested {
            let ticker = normalize_ticker_filter(ticker)?;
            if !normalized.contains(&ticker) {
                normalized.push(ticker);
            }
        }
        Ok(normalized)
    } else {
        Ok(Vec::new())
    }
}

fn validate_filters(query: &ProventosQuery) -> Result<ProventoFilters, ProventosError> {
    let tickers = parse_tickers(query.ticker.as_deref(), query.tickers.as_deref())?;
    let data_pagamento_desde = validate_date(
        query.data_pagamento_desde.as_deref(),
        "data_pagamento_desde",
    )?;
    let data_pagamento_ate =
        validate_date(query.data_pagamento_ate.as_deref(), "data_pagamento_ate")?;
    if let (Some(desde), Some(ate)) = (&data_pagamento_desde, &data_pagamento_ate) {
        if desde > ate {
            return Err(ProventosError::Invalid(
                "data_pagamento_desde não pode ser posterior a data_pagamento_ate".to_string(),
            ));
        }
    }
    Ok(ProventoFilters {
        tickers,
        tipo: query.tipo.map(|tipo| tipo.as_str().to_string()),
        data_pagamento_desde,
        data_pagamento_ate,
    })
}

fn normalize_ticker_filter(value: &str) -> Result<String, ProventosError> {
    let ticker = value.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || ticker.len() > 10
        || !ticker.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(ProventosError::Invalid(
            "cada ticker deve conter de 1 a 10 caracteres alfanuméricos".to_string(),
        ));
    }
    Ok(ticker)
}

/// Parse a positive base-10 BRL value into integer micro-reais without floating point.
fn parse_valor(value: &str) -> Result<i64, ProventosError> {
    let mut parts = value.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
    {
        return Err(ProventosError::Invalid(
            "valor deve ser um decimal em reais com até seis casas e sem separador de milhar"
                .to_string(),
        ));
    }
    let significant_integer = integer.trim_start_matches('0');
    let significant_integer = if significant_integer.is_empty() {
        "0"
    } else {
        significant_integer
    };
    if significant_integer.len() > 8 {
        return Err(ProventosError::Invalid(
            "valor excede o limite de R$ 99.999.999,999999".to_string(),
        ));
    }
    let reais = significant_integer
        .parse::<i64>()
        .map_err(|_| ProventosError::Invalid("valor fora do intervalo aceito".to_string()))?;
    let fractional_text = format!("{fraction:0<6}");
    let micros = fractional_text.parse::<i64>().map_err(|_| {
        ProventosError::Invalid("valor deve usar até seis casas decimais".to_string())
    })?;
    let total = reais * 1_000_000 + micros;
    if total <= 0 || total > MAX_VALOR_MICROS {
        return Err(ProventosError::Invalid(
            "valor deve ser positivo e respeitar DECIMAL(14,6)".to_string(),
        ));
    }
    Ok(total)
}

fn to_response(entry: ProventoEntry) -> Result<ProventoResponse, ProventosError> {
    let tipo = match entry.tipo.as_str() {
        "Dividendo" => ProventoTipo::Dividendo,
        "JCP" => ProventoTipo::Jcp,
        "Rendimento" => ProventoTipo::Rendimento,
        other => {
            return Err(ProventosError::Invalid(format!(
                "tipo de provento inválido no ledger: {other}"
            )))
        }
    };
    Ok(ProventoResponse {
        id: entry.id,
        ticker: entry.ticker,
        tipo,
        valor: format_micros(i128::from(entry.valor_micros)),
        data_com: entry.data_com,
        data_pagamento: entry.data_pagamento,
        obs: entry.obs,
        criado_em: entry.criado_em,
        atualizado_em: entry.atualizado_em,
    })
}

fn format_micros(micros: i128) -> String {
    format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn decimal_conversion_keeps_exact_six_digit_scale() {
        assert_eq!(parse_valor("0.0288").unwrap(), 28_800);
        assert_eq!(parse_valor("0001.2").unwrap(), 1_200_000);
        assert_eq!(parse_valor("99999999.999999").unwrap(), MAX_VALOR_MICROS);
        assert!(parse_valor("1.0000001").is_err());
        assert!(parse_valor("100000000").is_err());
        assert!(parse_valor("0").is_err());
    }

    #[test]
    fn seed_is_applied_once_and_manual_deletion_survives_service_restart() {
        let temp = TempDir::new().unwrap();
        let ledger = Arc::new(Ledger::open(&temp.path().join("ledger/tucano.db")).unwrap());
        let service = ProventosService::new(ledger.clone()).unwrap();
        let query = ProventosQuery {
            ticker: None,
            tickers: None,
            tipo: None,
            data_pagamento_desde: None,
            data_pagamento_ate: None,
            pagina: 1,
            por_pagina: 100,
        };
        let initial = service.list(query.clone()).unwrap();
        assert_eq!(initial.paginacao.total, 34);
        service.delete(initial.proventos[0].id).unwrap();

        let restarted = ProventosService::new(ledger).unwrap();
        let after_restart = restarted.list(query).unwrap();
        assert_eq!(after_restart.paginacao.total, 33);
        assert!(restarted.get(initial.proventos[0].id).is_err());
    }
}
