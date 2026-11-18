//! Market-calendar persistence and validation shared by the HTTP handlers.

use crate::ledger::{Ledger, LedgerError};
use crate::models::market_calendar::{
    MarketCalendarCreateRequest, MarketCalendarEvent, MarketCalendarResponse,
    MarketCalendarUpdateRequest, MarketDayStatus,
};
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;

const INITIAL_CALENDAR: &str = include_str!("b3-2026-calendar.json");
const INITIAL_CALENDAR_SEED_ID: &str = "b3-market-calendar-2026-v1";

/// Calendar lookup or write failed.
#[derive(Debug, Error)]
pub enum MarketCalendarError {
    #[error("Falha no ledger de calendários: {0}")]
    Ledger(#[from] LedgerError),
    #[error("Calendário não encontrado para {year}")]
    NotFound { year: i32 },
    #[error("Já existe um calendário para {year}")]
    AlreadyExists { year: i32 },
    #[error("Calendário inválido: {0}")]
    Invalid(String),
    #[error("Não foi possível ler os dados do calendário persistido: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Service that stores complete yearly calendars in the embedded SQLite ledger.
pub struct MarketCalendarService {
    ledger: Arc<Ledger>,
}

impl MarketCalendarService {
    /// Ensure the official 2026 calendar is available on a new installation.
    pub fn new(ledger: Arc<Ledger>) -> Result<Self, MarketCalendarError> {
        let service = Self { ledger };
        let seed: MarketCalendarCreateRequest = serde_json::from_str(INITIAL_CALENDAR)?;
        let seed = validate_create(seed)?;
        let payload = serde_json::to_string(&seed)?;
        service.ledger.initialize_market_calendar_seed(
            INITIAL_CALENDAR_SEED_ID,
            seed.ano,
            &payload,
            &Utc::now().to_rfc3339(),
        )?;
        Ok(service)
    }

    /// List years with calendars, in ascending order.
    pub fn years(&self) -> Result<Vec<i32>, MarketCalendarError> {
        Ok(self.ledger.list_market_calendar_years()?)
    }

    /// Return the requested calendar, if it exists.
    pub fn get(&self, year: i32) -> Result<MarketCalendarResponse, MarketCalendarError> {
        validate_year(year)?;
        let (payload, updated_at) = self
            .ledger
            .get_market_calendar(year)?
            .ok_or(MarketCalendarError::NotFound { year })?;
        let calendar: MarketCalendarCreateRequest = serde_json::from_str(&payload)?;
        Ok(MarketCalendarResponse {
            ano: calendar.ano,
            fontes: calendar.fontes,
            eventos: calendar.eventos,
            atualizado_em: updated_at,
        })
    }

    /// Create a new calendar. Existing years are preserved and return a conflict.
    pub fn create(
        &self,
        request: MarketCalendarCreateRequest,
    ) -> Result<MarketCalendarResponse, MarketCalendarError> {
        let request = validate_create(request)?;
        let payload = serde_json::to_string(&request)?;
        let updated_at = Utc::now().to_rfc3339();
        if !self
            .ledger
            .create_market_calendar(request.ano, &payload, &updated_at)?
        {
            return Err(MarketCalendarError::AlreadyExists { year: request.ano });
        }
        Ok(to_response(request, updated_at))
    }

    /// Replace all event and source data for an existing year.
    pub fn update(
        &self,
        year: i32,
        request: MarketCalendarUpdateRequest,
    ) -> Result<MarketCalendarResponse, MarketCalendarError> {
        let request = validate_update(year, request)?;
        let payload = serde_json::to_string(&request)?;
        let updated_at = Utc::now().to_rfc3339();
        if !self
            .ledger
            .update_market_calendar(year, &payload, &updated_at)?
        {
            return Err(MarketCalendarError::NotFound { year });
        }
        Ok(MarketCalendarResponse {
            ano: year,
            fontes: request.fontes,
            eventos: request.eventos,
            atualizado_em: updated_at,
        })
    }

    /// Delete the calendar for a year.
    pub fn delete(&self, year: i32) -> Result<(), MarketCalendarError> {
        validate_year(year)?;
        if self.ledger.delete_market_calendar(year)? {
            Ok(())
        } else {
            Err(MarketCalendarError::NotFound { year })
        }
    }
}

fn to_response(request: MarketCalendarCreateRequest, updated_at: String) -> MarketCalendarResponse {
    MarketCalendarResponse {
        ano: request.ano,
        fontes: request.fontes,
        eventos: request.eventos,
        atualizado_em: updated_at,
    }
}

fn validate_create(
    mut request: MarketCalendarCreateRequest,
) -> Result<MarketCalendarCreateRequest, MarketCalendarError> {
    validate_year(request.ano)?;
    validate_sources(&request.fontes)?;
    validate_events(request.ano, &mut request.eventos)?;
    Ok(request)
}

fn validate_update(
    year: i32,
    request: MarketCalendarUpdateRequest,
) -> Result<MarketCalendarCreateRequest, MarketCalendarError> {
    validate_year(year)?;
    validate_sources(&request.fontes)?;
    let mut events = request.eventos;
    validate_events(year, &mut events)?;
    Ok(MarketCalendarCreateRequest {
        ano: year,
        fontes: request.fontes,
        eventos: events,
    })
}

fn validate_year(year: i32) -> Result<(), MarketCalendarError> {
    if !(1900..=2200).contains(&year) {
        return Err(MarketCalendarError::Invalid(
            "o ano deve estar entre 1900 e 2200".to_string(),
        ));
    }
    Ok(())
}

fn validate_sources(sources: &[String]) -> Result<(), MarketCalendarError> {
    if sources.is_empty() || sources.len() > 10 {
        return Err(MarketCalendarError::Invalid(
            "informe de 1 a 10 URLs de fonte".to_string(),
        ));
    }
    for source in sources {
        let parsed = url::Url::parse(source)
            .map_err(|_| MarketCalendarError::Invalid("URL de fonte inválida".to_string()))?;
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if !matches!(parsed.scheme(), "https" | "http")
            || !(host == "b3.com.br" || host.ends_with(".b3.com.br"))
        {
            return Err(MarketCalendarError::Invalid(
                "as fontes devem ser páginas ou documentos oficiais de b3.com.br".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_events(
    year: i32,
    events: &mut [MarketCalendarEvent],
) -> Result<(), MarketCalendarError> {
    if events.len() > 500 {
        return Err(MarketCalendarError::Invalid(
            "o calendário aceita no máximo 500 eventos".to_string(),
        ));
    }
    let mut dates = HashSet::with_capacity(events.len());
    for event in events.iter() {
        let date = NaiveDate::parse_from_str(&event.data, "%Y-%m-%d").map_err(|_| {
            MarketCalendarError::Invalid(format!("data inválida: {}; use AAAA-MM-DD", event.data))
        })?;
        if date.format("%Y-%m-%d").to_string() != event.data {
            return Err(MarketCalendarError::Invalid(format!(
                "data fora do formato ISO AAAA-MM-DD: {}",
                event.data
            )));
        }
        if date.year() != year {
            return Err(MarketCalendarError::Invalid(format!(
                "a data {} não pertence ao ano {year}",
                event.data
            )));
        }
        if !dates.insert(event.data.as_str()) {
            return Err(MarketCalendarError::Invalid(format!(
                "há mais de um evento para a data {}",
                event.data
            )));
        }
        if event.evento.trim().is_empty() || event.evento.len() > 200 {
            return Err(MarketCalendarError::Invalid(
                "o nome do evento deve ter de 1 a 200 caracteres".to_string(),
            ));
        }
        if event
            .descricao
            .as_ref()
            .is_some_and(|value| value.len() > 1000)
        {
            return Err(MarketCalendarError::Invalid(
                "a descrição não pode exceder 1000 caracteres".to_string(),
            ));
        }

        for value in [
            event.pre_abertura.as_deref(),
            event.inicio_negociacao.as_deref(),
            event.fim_negociacao.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let time = NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
                MarketCalendarError::Invalid(format!("horário inválido: {value}; use HH:MM"))
            })?;
            if time.format("%H:%M").to_string() != value {
                return Err(MarketCalendarError::Invalid(format!(
                    "horário fora do formato HH:MM: {value}"
                )));
            }
        }

        let has_time = event.pre_abertura.is_some()
            || event.inicio_negociacao.is_some()
            || event.fim_negociacao.is_some();
        match event.situacao {
            MarketDayStatus::SpecialHours if !has_time => {
                return Err(MarketCalendarError::Invalid(format!(
                    "o evento especial de {} deve informar ao menos um horário",
                    event.data
                )))
            }
            MarketDayStatus::Closed | MarketDayStatus::Open if has_time => {
                return Err(MarketCalendarError::Invalid(format!(
                    "horários intradiários só podem ser informados para special_hours ({})",
                    event.data
                )))
            }
            _ => {}
        }
        if let (Some(start), Some(end)) = (
            event.inicio_negociacao.as_deref(),
            event.fim_negociacao.as_deref(),
        ) {
            let start = NaiveTime::parse_from_str(start, "%H:%M").map_err(|_| {
                MarketCalendarError::Invalid("horário de início inválido".to_string())
            })?;
            let end = NaiveTime::parse_from_str(end, "%H:%M").map_err(|_| {
                MarketCalendarError::Invalid("horário de encerramento inválido".to_string())
            })?;
            if start >= end {
                return Err(MarketCalendarError::Invalid(format!(
                    "o início da negociação deve preceder o encerramento ({})",
                    event.data
                )));
            }
        }
    }
    events.sort_by(|left, right| left.data.cmp(&right.data));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn seed_loads_official_2026_calendar_once() {
        let directory = TempDir::new().unwrap();
        let ledger = Arc::new(Ledger::open(&directory.path().join("ledger/tucano.db")).unwrap());
        let service = MarketCalendarService::new(ledger.clone()).unwrap();
        let calendar = service.get(2026).unwrap();
        assert_eq!(calendar.eventos.len(), 16);
        assert_eq!(calendar.eventos[0].data, "2026-01-01");
        assert_eq!(calendar.eventos[3].pre_abertura.as_deref(), Some("12:45"));
        assert_eq!(
            calendar.eventos[3].inicio_negociacao.as_deref(),
            Some("13:00")
        );

        MarketCalendarService::new(ledger).unwrap();
        assert_eq!(service.years().unwrap(), vec![2026]);
    }

    #[test]
    fn deleting_seed_is_not_reversed_when_service_restarts() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("ledger/tucano.db");
        let ledger = Arc::new(Ledger::open(&path).unwrap());
        let service = MarketCalendarService::new(ledger.clone()).unwrap();
        service.delete(2026).unwrap();
        drop(service);
        drop(ledger);

        let service = MarketCalendarService::new(Arc::new(Ledger::open(&path).unwrap())).unwrap();
        assert!(matches!(
            service.get(2026),
            Err(MarketCalendarError::NotFound { year: 2026 })
        ));
    }

    #[test]
    fn validation_rejects_dates_outside_calendar_year() {
        let event = MarketCalendarEvent {
            data: "2027-01-01".into(),
            evento: "Confraternização Universal".into(),
            situacao: MarketDayStatus::Closed,
            descricao: None,
            pre_abertura: None,
            inicio_negociacao: None,
            fim_negociacao: None,
        };
        assert!(validate_events(2026, &mut [event]).is_err());
    }
}
