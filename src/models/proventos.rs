//! Contratos para proventos cadastrados no ledger SQLite.

use crate::models::pagination::PaginationMeta;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Categoria de provento aceita pela API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProventoTipo {
    /// Dividendo em dinheiro.
    #[serde(rename = "Dividendo")]
    Dividendo,
    /// Juros sobre capital próprio.
    #[serde(rename = "JCP")]
    Jcp,
    /// Rendimento distribuído por fundo ou outro ativo.
    #[serde(rename = "Rendimento")]
    Rendimento,
}

impl ProventoTipo {
    /// Valor textual usado no SQLite e no contrato HTTP.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dividendo => "Dividendo",
            Self::Jcp => "JCP",
            Self::Rendimento => "Rendimento",
        }
    }
}

/// Corpo completo para criação ou substituição de um provento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "ticker": "PETR4",
    "tipo": "JCP",
    "valor": "0.674100",
    "data_com": "2026-08-21",
    "data_pagamento": "2026-11-23",
    "obs": "Fonte informada no cadastro; previsão de pagamento em novembro de 2026"
}))]
pub struct ProventoRequest {
    /// Código de negociação B3, entre 1 e 10 caracteres alfanuméricos.
    #[schema(min_length = 1, max_length = 10, example = "PETR4")]
    pub ticker: String,
    /// Categoria do provento: `Dividendo`, `JCP` ou `Rendimento`.
    #[schema(example = "JCP")]
    pub tipo: ProventoTipo,
    /// Valor por ação ou cota em reais como decimal textual exato com até seis casas.
    #[schema(value_type = String, pattern = r"^[0-9]{1,8}(\.[0-9]{1,6})?$", example = "0.674100")]
    pub valor: String,
    /// Data-com em ISO 8601 (`AAAA-MM-DD`), quando informada pela fonte.
    #[schema(example = "2026-08-21", format = "date")]
    pub data_com: Option<String>,
    /// Data de pagamento em ISO 8601 (`AAAA-MM-DD`), quando informada pela fonte.
    #[schema(example = "2026-11-23", format = "date")]
    pub data_pagamento: Option<String>,
    /// Fonte e observações (máximo de 255 caracteres).
    #[schema(max_length = 255, example = "Fonte informada no cadastro")]
    pub obs: Option<String>,
}

/// Provento persistido, incluindo seu identificador e auditoria de alteração.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "id": 23,
    "ticker": "PETR4",
    "tipo": "JCP",
    "valor": "0.674100",
    "data_com": "2026-08-21",
    "data_pagamento": "2026-11-23",
    "obs": "Fonte informada no cadastro",
    "criado_em": "2026-11-24T12:30:00Z",
    "atualizado_em": "2026-11-24T12:30:00Z"
}))]
pub struct ProventoResponse {
    /// Identificador estável do registro.
    pub id: i64,
    /// Código de negociação B3.
    pub ticker: String,
    /// Categoria do provento.
    pub tipo: ProventoTipo,
    /// Valor por ação ou cota em reais, formatado com seis casas decimais.
    #[schema(value_type = String, example = "0.674100")]
    pub valor: String,
    /// Data-com em ISO 8601, quando informada.
    #[schema(example = "2026-08-21", format = "date")]
    pub data_com: Option<String>,
    /// Data de pagamento em ISO 8601, quando informada.
    #[schema(example = "2026-11-23", format = "date")]
    pub data_pagamento: Option<String>,
    /// Fonte e observações associadas ao registro.
    pub obs: Option<String>,
    /// Instante de criação em UTC.
    pub criado_em: String,
    /// Instante da última alteração em UTC.
    pub atualizado_em: String,
}

/// Filtros e paginação para consulta de proventos.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct ProventosQuery {
    /// Filtra por ticker; a API normaliza a entrada para maiúsculas.
    #[param(min_length = 1, max_length = 10, example = "PETR4")]
    pub ticker: Option<String>,
    /// Filtra por até 100 tickers separados por vírgula; exclusivo com `ticker`.
    /// Cada código é aparado, convertido para maiúsculas e deduplicado.
    #[param(min_length = 1, max_length = 1099, example = "PETR4,VALE3")]
    pub tickers: Option<String>,
    /// Filtra pela categoria `Dividendo`, `JCP` ou `Rendimento`.
    #[param(example = "JCP")]
    pub tipo: Option<ProventoTipo>,
    /// Filtra pagamentos a partir desta data, inclusiva (`AAAA-MM-DD`).
    #[param(format = "date", example = "2026-11-01")]
    pub data_pagamento_desde: Option<String>,
    /// Filtra pagamentos até esta data, inclusiva (`AAAA-MM-DD`).
    #[param(format = "date", example = "2026-11-30")]
    pub data_pagamento_ate: Option<String>,
    /// Página começando em 1 (padrão 1).
    #[serde(default = "default_page")]
    #[param(default = 1, minimum = 1, example = 1)]
    pub pagina: u32,
    /// Registros por página (padrão 50, máximo 1000).
    #[serde(default = "default_per_page")]
    #[param(default = 50, minimum = 1, maximum = 1000, example = 50)]
    pub por_pagina: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    50
}

impl ProventosQuery {
    /// Paginação segura para cálculos de `LIMIT` e `OFFSET`.
    pub fn pagination(&self) -> (u32, u32) {
        let page = self.pagina.max(1);
        let per_page = self.por_pagina.clamp(1, 1000);
        (page, per_page)
    }

    /// Offset SQL sem overflow para entradas grandes.
    pub fn offset(&self) -> u32 {
        let (page, per_page) = self.pagination();
        page.saturating_sub(1).saturating_mul(per_page)
    }
}

/// Página de registros de proventos.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "proventos": [{
        "id": 23,
        "ticker": "PETR4",
        "tipo": "JCP",
        "valor": "0.674100",
        "data_com": "2026-08-21",
        "data_pagamento": "2026-11-23",
        "obs": "Fonte informada no cadastro",
        "criado_em": "2026-11-24T12:30:00Z",
        "atualizado_em": "2026-11-24T12:30:00Z"
    }],
    "paginacao": {"total": 34, "pagina": 1, "por_pagina": 50, "total_paginas": 1}
}))]
pub struct ProventosListResponse {
    /// Registros em ordem determinística por pagamento, ticker, tipo e identificador.
    pub proventos: Vec<ProventoResponse>,
    /// Metadados da paginação aplicada.
    pub paginacao: PaginationMeta,
}

/// Confirmação de exclusão de um registro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"id": 23, "removido": true}))]
pub struct ProventoDeleteResponse {
    /// Identificador do registro removido.
    pub id: i64,
    /// Indica que o registro foi excluído.
    pub removido: bool,
}

/// Formulário multipart para upload de um arquivo CSV de proventos.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProventosCsvUploadRequest {
    /// Arquivo CSV UTF-8 de até 1 MiB e 10.000 registros, com cabeçalho e separador vírgula.
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

/// Resultado de uma importação CSV confirmada no ledger.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"recebidos": 3, "inseridos": 2, "ignorados": 1}))]
pub struct ProventosCsvUploadResponse {
    /// Total de registros válidos lidos, sem contar o cabeçalho.
    pub recebidos: usize,
    /// Quantidade de registros novos inseridos na mesma transação.
    pub inseridos: usize,
    /// Quantidade de registros idênticos a um já persistido ou inserido antes no mesmo arquivo.
    pub ignorados: usize,
}

/// Seleção de um ticker e um ano civil de data-com.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ProventosYearlyQuery {
    /// Código alfanumérico de 1 a 10 caracteres, normalizado para maiúsculas.
    #[param(min_length = 1, max_length = 10, example = "TAEE3")]
    pub ticker: String,
    /// Ano civil da data-com, de 1 a 9999.
    #[param(minimum = 1, maximum = 9999, example = 2026)]
    pub ano: i32,
}

/// Seleção de tickers e intervalo inclusivo de data-com para relatórios.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ProventosPeriodQuery {
    /// Código alfanumérico de 1 a 10 caracteres. Informe este campo ou `tickers`.
    #[param(min_length = 1, max_length = 10, example = "TAEE3")]
    pub ticker: Option<String>,
    /// De 1 a 100 códigos separados por vírgula. Espaços externos e diferenças de caixa são normalizados. Códigos repetidos são consolidados. Informe este campo ou `ticker`.
    #[param(example = "PETR4,TAEE3")]
    pub tickers: Option<String>,
    /// Primeira data-com incluída no relatório, em `AAAA-MM-DD`.
    #[param(format = "date", example = "2026-08-01")]
    pub desde: String,
    /// Última data-com incluída no relatório, em `AAAA-MM-DD`. Deve ser igual ou posterior a `desde`.
    #[param(format = "date", example = "2026-08-31")]
    pub ate: String,
}

/// Total anual de proventos cadastrados para um ticker.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"ticker": "TAEE3", "ano": 2026, "proventos_on_year": "0.200100"}))]
pub struct ProventosYearlyResponse {
    /// Código de negociação normalizado.
    pub ticker: String,
    /// Ano civil usado para filtrar a data-com.
    pub ano: i32,
    /// Soma em reais por ação ou cota, em texto com seis casas decimais. É `0.000000` quando não há registros.
    #[schema(pattern = r"^[0-9]+\.[0-9]{6}$", example = "0.200100")]
    pub proventos_on_year: String,
}

/// Total de proventos para um ticker no intervalo solicitado.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"ticker": "TAEE3", "proventos_on_period": "0.200100"}))]
pub struct ProventosPeriodTotal {
    /// Código de negociação normalizado.
    pub ticker: String,
    /// Soma em reais por ação ou cota, em texto com seis casas decimais. É `0.000000` quando não há registros.
    #[schema(pattern = r"^[0-9]+\.[0-9]{6}$", example = "0.200100")]
    pub proventos_on_period: String,
}

/// Ponto da série com o total de proventos por data-com.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProventosSeriesPoint {
    /// Data-com à meia-noite UTC, em milissegundos desde o Unix epoch.
    #[schema(example = 1786665600000_i64)]
    pub date_ms: i64,
    /// Soma de todos os tipos nessa data-com, em reais por ação ou cota, em texto com seis casas decimais.
    #[schema(pattern = r"^[0-9]+\.[0-9]{6}$", example = "0.200100")]
    pub valor: String,
}

/// Série compacta de proventos para um ticker no intervalo solicitado.
#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "ticker": "TAEE3",
    "series": [{"date_ms": 1786665600000_i64, "valor": "0.200100"}]
}))]
pub struct ProventosTickerSeries {
    /// Código de negociação normalizado.
    pub ticker: String,
    /// Um ponto por data-com com registros, em ordem crescente. Sem registros, a lista fica vazia.
    pub series: Vec<ProventosSeriesPoint>,
}
