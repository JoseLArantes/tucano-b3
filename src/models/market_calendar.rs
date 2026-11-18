//! B3 market calendar and negotiation-hours contracts.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Tipo de funcionamento informado para uma data do calendário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarketDayStatus {
    /// Não há sessão de negociação.
    Closed,
    /// A sessão ocorre em horário diferente do padrão.
    SpecialHours,
    /// A negociação funciona normalmente, apesar da data comemorativa.
    Open,
}

/// Evento de funcionamento de mercado em uma data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketCalendarEvent {
    /// Data local da B3 no formato ISO 8601 (`AAAA-MM-DD`).
    #[schema(example = "2026-02-18", format = "date")]
    pub data: String,
    /// Nome do feriado ou evento.
    #[schema(example = "Quarta-feira de Cinzas")]
    #[schema(max_length = 200, example = "Quarta-feira de Cinzas")]
    pub evento: String,
    /// Situação da sessão de negociação na data.
    pub situacao: MarketDayStatus,
    /// Observação publicada pela B3 para o evento, quando aplicável.
    #[schema(max_length = 1000)]
    pub descricao: Option<String>,
    /// Horário de pré-abertura (`HH:MM`), quando publicado.
    #[schema(example = "12:45")]
    pub pre_abertura: Option<String>,
    /// Horário de início da negociação (`HH:MM`), quando publicado.
    #[schema(example = "13:00")]
    pub inicio_negociacao: Option<String>,
    /// Horário de encerramento da negociação (`HH:MM`), quando publicado.
    #[schema(example = "18:00")]
    pub fim_negociacao: Option<String>,
}

/// Corpo usado para criar um calendário anual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "ano": 2027,
    "fontes": ["https://www.b3.com.br/pt_br/"],
    "eventos": [{
        "data": "2027-01-01",
        "evento": "Confraternização Universal",
        "situacao": "closed",
        "descricao": "Não haverá sessão de negociação.",
        "pre_abertura": null,
        "inicio_negociacao": null,
        "fim_negociacao": null
    }]
}))]
pub struct MarketCalendarCreateRequest {
    /// Ano civil coberto pelo calendário.
    #[schema(example = 2026, minimum = 1900, maximum = 2200)]
    pub ano: i32,
    /// URLs oficiais ou documentais que fundamentam o calendário.
    #[schema(min_items = 1, max_items = 10)]
    pub fontes: Vec<String>,
    /// Feriados e exceções de funcionamento em ordem cronológica.
    #[schema(max_items = 500)]
    pub eventos: Vec<MarketCalendarEvent>,
}

/// Corpo usado para substituir os dados de um calendário existente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "fontes": ["https://www.b3.com.br/pt_br/"],
    "eventos": [{
        "data": "2027-01-01",
        "evento": "Confraternização Universal",
        "situacao": "closed",
        "descricao": "Não haverá sessão de negociação.",
        "pre_abertura": null,
        "inicio_negociacao": null,
        "fim_negociacao": null
    }]
}))]
pub struct MarketCalendarUpdateRequest {
    /// URLs oficiais ou documentais que fundamentam o calendário.
    #[schema(min_items = 1, max_items = 10)]
    pub fontes: Vec<String>,
    /// Feriados e exceções de funcionamento em ordem cronológica.
    #[schema(max_items = 500)]
    pub eventos: Vec<MarketCalendarEvent>,
}

/// Calendário anual de funcionamento da B3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketCalendarResponse {
    /// Ano civil coberto pelo calendário.
    pub ano: i32,
    /// URLs oficiais ou documentais que fundamentam o calendário.
    pub fontes: Vec<String>,
    /// Feriados e exceções de funcionamento em ordem cronológica.
    pub eventos: Vec<MarketCalendarEvent>,
    /// Instante da última criação ou alteração, em UTC.
    pub atualizado_em: String,
}

/// Anos que possuem um calendário cadastrado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketCalendarYearsResponse {
    /// Anos disponíveis em ordem crescente.
    pub anos: Vec<i32>,
}

/// Confirmação de remoção de um calendário anual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketCalendarDeleteResponse {
    /// Ano removido.
    pub ano: i32,
    /// Indica que o registro foi excluído.
    pub removido: bool,
}

/// Horário publicado para uma modalidade da plataforma Trader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NegotiationSessionHours {
    /// Modalidade descrita pela B3 (`Tela`, `RFQ` ou `Voice`).
    pub modalidade: String,
    /// Horário de abertura local no formato `HH:MM`.
    #[schema(example = "09:00")]
    pub abertura: String,
    /// Horário de fechamento local no formato `HH:MM`.
    #[schema(example = "18:00")]
    pub fechamento: String,
}

/// Horários de negociação publicados pela B3 para o Trader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NegotiationHoursResponse {
    /// Fuso usado nos horários publicados pela B3.
    pub fuso_horario: String,
    /// Página da B3 usada como fonte.
    pub fonte: String,
    /// Horários por modalidade.
    pub modalidades: Vec<NegotiationSessionHours>,
}
