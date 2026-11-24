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
