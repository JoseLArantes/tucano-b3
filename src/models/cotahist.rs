//! COTAHIST data models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::pagination::PaginationMeta;

pub use crate::models::cotahist_analytics::*;

/// A single COTAHIST quote record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub ticker: String,
    pub data: String,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: Option<u32>,
    pub quat_tot: Option<u64>,
    pub vol_tot: Option<f64>,
    pub var_pct: Option<f64>,
    pub var_abs: Option<f64>,
}

/// Ticker metadata from the ledger.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerResponse {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
}

/// Extended ticker info with latest quote data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerDetailResponse {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
    pub total_trading_days: Option<u32>,
    pub latest_price: Option<f64>,
    pub latest_date: Option<String>,
}

/// Verified equity listing backed by a spot-market COTAHIST record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EquityResponse {
    /// Ticker normalizado usado para localizar a listagem.
    pub ticker: String,
    /// Código de negociação preservado do registro oficial COTAHIST.
    pub raw_ticker: String,
    /// ISIN oficial de 12 caracteres validado pelo dígito verificador.
    pub isin: String,
    /// Código BDI original, sem conversão.
    pub bdi_code: String,
    /// Código TPMERC original, incluindo zeros à esquerda (por exemplo, `010`).
    pub market_code: String,
    /// Nome de emissor reportado no COTAHIST, quando presente.
    pub company_name: Option<String>,
    /// Especificação oficial ESPECI, como `ON`, `PN` ou `UNT`.
    pub security_specification: String,
    /// Primeiro pregão encontrado para este par ticker/ISIN no conjunto verificado.
    pub first_seen: String,
    /// Pregão mais recente encontrado para este par ticker/ISIN.
    pub last_seen: String,
    /// Indica que o ledger local associa o ticker ao mesmo ISIN como listagem atual.
    pub is_active: bool,
}

/// Nominal, unadjusted B3 closing price for one share or unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EquityQuoteResponse {
    /// Código de negociação preservado do registro oficial COTAHIST.
    pub ticker: String,
    /// ISIN usado para selecionar a série histórica da listagem.
    pub isin: String,
    /// Código BDI original do registro COTAHIST.
    pub bdi_code: String,
    /// Código TPMERC original do registro COTAHIST.
    pub market_code: String,
    /// Data do pregão no formato ISO 8601 `AAAA-MM-DD`.
    pub data: String,
    /// PREULT dividido pelo FATCOT, como decimal exato por ação ou unit.
    pub close_per_share: String,
    /// FATCOT oficial aplicado ao PREULT.
    pub quotation_factor: u32,
    /// Fonte do fechamento nominal.
    pub source: String,
    /// Horário UTC em que a consulta foi realizada.
    pub retrieved_at: String,
    /// Estado da observação; atualmente `available` para pontos retornados.
    pub availability: String,
    /// Sempre `false`: COTAHIST não aplica ajustes retroativos por eventos corporativos.
    pub adjusted_for_corporate_actions: bool,
}

/// Calendar coverage of rows in the verified equity dataset.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EquityCoverage {
    /// Primeiro pregão coberto pelas listagens elegíveis.
    pub from: String,
    /// Último pregão coberto pelas listagens elegíveis.
    pub to: String,
}

/// Segmento COTAHIST disponível na primeira versão do volume financeiro.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TurnoverSegment {
    /// Mercado à vista de lote padrão (TPMERC 010 e CODBDI 02).
    StandardLot,
}

/// Observação oficial de volume financeiro por listagem e pregão.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EquityTurnoverObservation {
    /// Data do pregão em São Paulo (AAAA-MM-DD).
    #[schema(example = "2026-01-05")]
    pub data: String,
    /// ISIN da listagem no registro COTAHIST.
    #[schema(example = "BRPETRACNOR9")]
    pub isin: String,
    /// Ticker oficial no pregão, sem normalização de sufixo.
    #[schema(example = "PETR3")]
    pub ticker: String,
    /// Moeda do volume financeiro. O piloto cobre BRL.
    #[schema(example = "BRL")]
    pub currency: String,
    /// Segmento da observação.
    pub segment: TurnoverSegment,
    /// VOLTOT exato em reais, com ponto decimal e duas casas. FATCOT não se aplica.
    #[schema(example = "10000.01")]
    pub financial_turnover_brl: String,
    /// Campo posicional VOLTOT, com 18 dígitos e duas casas implícitas.
    #[schema(example = "000000000001000001")]
    pub voltot_raw: String,
    /// Nome do membro TXT original, inclusive para arquivos ZIP.
    #[schema(example = "COTAHIST_A2026.TXT")]
    pub source_member: String,
    /// Número físico da linha no membro TXT, a partir de 1.
    #[schema(example = 2)]
    pub source_line: u64,
}

/// Observações publicadas e revisão lida na mesma consulta.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EquityTurnoverResponse {
    /// Identificador opaco da revisão publicada, ou null antes da primeira carga.
    pub dataset_revision: Option<String>,
    /// Quantidade de observações no intervalo solicitado.
    pub observation_count: usize,
    /// Somente observações disponíveis, em ordem crescente de data.
    pub observations: Vec<EquityTurnoverObservation>,
}

/// Paged verified equity listings.
#[derive(Debug, Serialize, ToSchema)]
pub struct EquityListResponse {
    /// Listagens compatíveis com ações ordinárias, preferenciais e units.
    pub equities: Vec<EquityResponse>,
    /// Metadados de paginação após os filtros de elegibilidade e status.
    pub paginacao: PaginationMeta,
    /// Cobertura observada do conjunto verificado, quando há dados disponíveis.
    pub coverage: Option<EquityCoverage>,
}

/// Response for the ticker list endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct TickerListResponse {
    pub tickers: Vec<TickerResponse>,
    pub paginacao: PaginationMeta,
}

/// Response for the quotes endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct QuoteListResponse {
    pub ticker: String,
    pub quotes: Vec<QuoteResponse>,
    pub paginacao: PaginationMeta,
}

/// Response for the latest quote endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct LatestQuoteResponse {
    pub ticker: String,
    pub data: String,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: Option<u32>,
    pub quat_tot: Option<u64>,
    pub vol_tot: Option<f64>,
    pub var_pct: Option<f64>,
    pub var_abs: Option<f64>,
}

/// Response for the date quotes endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct DateQuotesResponse {
    pub date: String,
    pub quotes: Vec<QuoteResponse>,
    pub paginacao: PaginationMeta,
}

/// Compact daily point for historical charts.
///
/// `date_ms` is the trading date at UTC midnight as Unix epoch milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SeriesPoint {
    /// Data do pregão em milissegundos (Unix epoch, UTC 00:00)
    pub date_ms: i64,
    /// Preço de fechamento (PREULT)
    pub pre_ult: Option<f64>,
    /// Volume total negociado em R$ (VOLTOT)
    pub vol_tot: Option<f64>,
    /// Quantidade total de negócios (TOTNEG)
    pub tot_neg: Option<u32>,
}

/// Response for the compact historical series endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct SeriesResponse {
    pub ticker: String,
    /// Data inicial solicitada (AAAA-MM-DD)
    pub desde: String,
    /// Data final solicitada, se informada (AAAA-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    pub series: Vec<SeriesPoint>,
}

/// Extreme (min or max) closing price for a period.
#[derive(Debug, Serialize, ToSchema)]
pub struct PreUltExtremeResponse {
    pub ticker: String,
    /// Data inicial do período (AAAA-MM-DD)
    pub desde: String,
    /// Data final do período, se informada (AAAA-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Data do pregão em que o extremo ocorreu (AAAA-MM-DD)
    pub data: String,
    /// Preço de fechamento extremo (PREULT)
    pub pre_ult: f64,
}
