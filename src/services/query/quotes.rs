use super::{QueryError, QueryPurpose, QueryService};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub const NOMINAL_WARNING: &str = "Preços históricos nominais brutos, na moeda e forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos.";

/// Filtros do histórico nominal COTAHIST; datas inclusivas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct QuoteFilters {
    /// Código de negociação; não identifica um instrumento permanente.
    #[param(example = "PETR4")]
    pub ticker_symbol: Option<String>,
    /// Identificador canônico do instrumento, quando sustentado temporalmente.
    pub instrument_id: Option<String>,
    /// Código ISIN publicado na ocorrência.
    pub isin: Option<String>,
    /// Primeiro pregão incluído, em YYYY-MM-DD.
    pub trade_date_from: Option<NaiveDate>,
    /// Último pregão incluído, em YYYY-MM-DD.
    pub trade_date_to: Option<NaiveDate>,
    /// Código de mercado B3, preservado com zeros à esquerda.
    #[param(example = "010")]
    pub market_type: Option<String>,
    /// Código BDI oficial.
    pub bdi_code: Option<String>,
    /// Distribuição do papel; componente da granularidade.
    pub distribution_number: Option<String>,
    /// Prazo do mercado a termo conforme o arquivo.
    pub term_days: Option<String>,
    /// Revisão retida a fixar entre páginas; ausente seleciona a revisão ativa.
    pub data_revision: Option<Uuid>,
    /// Número máximo de itens; padrão 100, máximo 1000.
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    /// Deslocamento não negativo; padrão 0.
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
    /// Acrescenta metadados internos seguros; não concede autorização.
    #[param(default = false)]
    pub include_internal: Option<bool>,
}
impl QuoteFilters {
    pub fn validate(&self) -> Result<(i64, i64), QueryError> {
        let (limit, offset) = (self.limit.unwrap_or(100), self.offset.unwrap_or(0));
        if !(1..=1000).contains(&limit)
            || offset < 0
            || matches!((self.trade_date_from,self.trade_date_to),(Some(a),Some(b)) if a>b)
        {
            return Err(QueryError::Invalid(
                "Limites ou intervalo inconsistentes.".into(),
            ));
        }
        Ok((limit, offset))
    }
}

/// Origem oficial selecionada para este fato e revisão.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteProvenance {
    /// UUID do arquivo fonte preservado.
    pub source_file_id: String,
    /// Família oficial; anual, mensal e diário permanecem distinguíveis.
    pub source_file_type: String,
    /// SHA-256 dos bytes do artefato bruto.
    pub sha256: String,
    /// UUID do member no inventário determinístico.
    pub source_member_id: String,
    /// Caminho de origem do member.
    pub source_member_name: String,
    /// Linha/registro original, iniciado em 1.
    pub record_index: i64,
}
/// Relação temporal com o registro de instrumentos, independente do preço.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteIdentity {
    /// Instrumento associado; nulo quando não há evidência inequívoca.
    pub instrument_id: Option<String>,
    /// Resultado fechado da resolução temporal.
    pub status: IdentityStatus,
}
/// Resultado da resolução temporal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatus {
    /// Evidência temporal inequívoca relaciona a ocorrência a um instrumento.
    Resolved,
    /// Nenhuma relação sustentada foi encontrada; o fato de mercado permanece disponível.
    Unresolved,
    /// Mais de uma relação permanece possível para a data e os qualificadores.
    Ambiguous,
}
/// Metadados de interpretação acrescentados mediante include_internal=true.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteInternal {
    /// Versão imutável de negócio selecionada.
    pub version_id: Uuid,
    /// Hash canônico sem metadados operacionais.
    pub business_hash: String,
}
/// Registro COTAHIST com precisão decimal e unidades originais.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalQuote {
    /// Identidade estável do fato, independente de suas versões.
    pub id: Uuid,
    /// Data do pregão.
    pub trade_date: NaiveDate,
    /// Ticker no contexto publicado.
    pub ticker_symbol: Option<String>,
    /// Código de mercado B3.
    pub market_type: Option<String>,
    /// Código BDI.
    pub bdi_code: Option<String>,
    /// Distribuição do papel (DISMES).
    pub distribution_number: Option<String>,
    /// Prazo a termo original.
    pub term_days: Option<String>,
    /// Nome abreviado do emissor publicado.
    pub issuer_short_name: Option<String>,
    /// Especificação do papel.
    pub paper_specification: Option<String>,
    /// Moeda original, sem conversão histórica para BRL.
    pub currency: Option<String>,
    /// Preço nominal de abertura, decimal textual.
    pub open_price: Option<String>,
    /// Máximo nominal, decimal textual.
    pub high_price: Option<String>,
    /// Mínimo nominal, decimal textual.
    pub low_price: Option<String>,
    /// Preço médio nominal, decimal textual.
    pub average_price: Option<String>,
    /// Último negócio (PREULT); não redefine fechamento oficial.
    pub last_price: Option<String>,
    /// Melhor oferta de compra publicada.
    pub best_bid_price: Option<String>,
    /// Melhor oferta de venda publicada.
    pub best_ask_price: Option<String>,
    /// Número de negócios.
    pub trades_count: Option<i32>,
    /// Quantidade negociada, decimal textual.
    pub traded_quantity: Option<String>,
    /// Volume na moeda e unidade originais, decimal textual.
    pub traded_volume: Option<String>,
    /// Preço de exercício de opção, decimal textual.
    pub option_exercise_price: Option<String>,
    /// Indicador de correção publicado.
    pub price_correction_indicator: Option<String>,
    /// Vencimento; nulo para sentinela/ausência.
    pub maturity_date: Option<NaiveDate>,
    /// Fator de cotação FATCOT, sem equivalência com ajuste corporativo.
    pub quotation_factor: Option<i32>,
    /// Exercício em pontos, decimal textual.
    pub exercise_price_points: Option<String>,
    /// ISIN da fonte.
    pub isin: Option<String>,
    /// Estado da associação temporal com instrumento.
    pub identity: QuoteIdentity,
    /// Origem navegável até arquivo, member e registro.
    pub provenance: QuoteProvenance,
    /// Metadados seguros opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<QuoteInternal>,
}
/// Página consistente do histórico nominal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotePage {
    /// Itens em ordem por data e identificador único.
    pub items: Vec<HistoricalQuote>,
    /// Total na revisão e nos filtros selecionados.
    pub total: i64,
    /// Limite efetivo, padrão 100.
    pub limit: i64,
    /// Deslocamento efetivo.
    pub offset: i64,
    /// Revisão fixada para repetição e paginação.
    pub data_revision: Uuid,
    /// Limitação obrigatória de preços sem ajustes.
    pub disclaimer: String,
    /// Qualidade e limitações de cobertura conhecidas.
    pub coverage: Coverage,
}
/// Cobertura demonstrada, sem inferência de continuidade por min/max.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Coverage {
    /// Início inclusivo solicitado; nulo para escopo integral da revisão.
    pub requested_from: Option<NaiveDate>,
    /// Fim inclusivo solicitado; nulo para escopo integral da revisão.
    pub requested_to: Option<NaiveDate>,
    /// Estado de cobertura: parcial quando há limites ou incerteza.
    pub status: CoverageStatus,
    /// Primeira data efetivamente selecionada.
    pub first_observed: Option<NaiveDate>,
    /// Última data efetivamente selecionada; não é prova de continuidade.
    pub last_observed: Option<NaiveDate>,
    /// Quantidade de ocorrências rejeitadas na revisão.
    pub rejected: i64,
    /// Quantidade de fatos sem vínculo temporal inequívoco.
    pub unresolved: i64,
    /// Calendário não demonstrado pela consulta não gera percentual de completude.
    pub calendar_status: String,
}
/// Vocabulário fechado de cobertura.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial,
    Unknown,
    NotApplicable,
}

/// Diagnóstico limitado de uma revisão publicada e elegível.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DatasetDiagnostics {
    /// Revisão selecionada para repetição da consulta.
    pub data_revision: Uuid,
    /// Qualidade e limites de inferência da cobertura.
    pub coverage: Coverage,
}
/// Revisão opcional para diagnóstico consistente.
#[derive(Debug, Default, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticsFilters {
    /// Revisão publicada a fixar; ausente seleciona a ativa.
    pub data_revision: Option<Uuid>,
}

const FILTER_SQL: &str = " FROM b3_history_revision_quotes q JOIN b3_cotahist_facts f ON f.id=q.fact_id JOIN b3_cotahist_versions v ON v.id=q.version_id JOIN b3_source_file_members m ON m.id=q.source_member_id JOIN b3_source_files sf ON sf.id=m.source_file_id WHERE q.revision_id=$1 AND ($2::text IS NULL OR f.ticker_symbol=$2) AND ($3::date IS NULL OR f.trade_date>=$3) AND ($4::date IS NULL OR f.trade_date<=$4) AND ($5::text IS NULL OR f.market_type=$5) AND ($6::text IS NULL OR f.bdi_code=$6) AND ($7::text IS NULL OR f.distribution_number=$7) AND ($8::text IS NULL OR f.term_days=$8) AND ($9::text IS NULL OR q.instrument_id=$9) AND ($10::text IS NULL OR v.isin=$10)";

impl QueryService {
    pub async fn diagnostics(
        &self,
        filters: DiagnosticsFilters,
        purpose: QueryPurpose,
    ) -> Result<DatasetDiagnostics, QueryError> {
        self.bounded(async {
            let revision = self
                .resolve_revision(filters.data_revision, purpose)
                .await?;
            Ok(DatasetDiagnostics {
                data_revision: revision,
                coverage: self.coverage(revision, purpose).await?,
            })
        })
        .await
    }
    pub async fn quotes(
        &self,
        filters: QuoteFilters,
        purpose: QueryPurpose,
        series: bool,
    ) -> Result<QuotePage, QueryError> {
        self.bounded(self.quotes_inner(filters, purpose, series))
            .await
    }
    async fn quotes_inner(
        &self,
        filters: QuoteFilters,
        purpose: QueryPurpose,
        series: bool,
    ) -> Result<QuotePage, QueryError> {
        let (limit, offset) = filters.validate()?;
        if series
            && filters.ticker_symbol.is_none()
            && filters.instrument_id.is_none()
            && filters.isin.is_none()
        {
            return Err(QueryError::Invalid(
                "Informe ticker, ISIN ou instrumento para a série.".into(),
            ));
        }
        let revision = self
            .resolve_revision(filters.data_revision, purpose)
            .await?;
        macro_rules! bind_filters {
            ($q:expr) => {
                $q.bind(revision)
                    .bind(
                        filters
                            .ticker_symbol
                            .as_ref()
                            .map(|s| s.trim().to_uppercase()),
                    )
                    .bind(filters.trade_date_from)
                    .bind(filters.trade_date_to)
                    .bind(&filters.market_type)
                    .bind(&filters.bdi_code)
                    .bind(&filters.distribution_number)
                    .bind(&filters.term_days)
                    .bind(&filters.instrument_id)
                    .bind(filters.isin.as_ref().map(|s| s.trim().to_uppercase()))
            };
        }
        if series {
            let ambiguous: bool = bind_filters!(sqlx::query_scalar(&format!(
                "SELECT EXISTS(SELECT 1{FILTER_SQL} GROUP BY f.trade_date HAVING count(*)>1)"
            )))
            .fetch_one(&self.pool)
            .await?;
            if ambiguous {
                return Err(QueryError::Ambiguous);
            }
        }
        let summary = bind_filters!(sqlx::query(&format!("SELECT count(*) AS total,min(f.trade_date) AS first,max(f.trade_date) AS last,count(*) FILTER(WHERE q.resolution_status<>'resolved') AS unresolved{FILTER_SQL}"))).fetch_one(&self.pool).await?;
        let total: i64 = summary.get("total");
        let fields="SELECT f.id,f.trade_date,f.ticker_symbol,f.market_type,f.bdi_code,f.distribution_number,f.term_days,v.id AS version_id,v.business_hash,v.issuer_short_name,v.paper_specification,v.currency,v.open_price::text,v.high_price::text,v.low_price::text,v.average_price::text,v.last_price::text,v.best_bid_price::text,v.best_ask_price::text,v.trades_count,v.traded_quantity::text,v.traded_volume::text,v.option_exercise_price::text,v.price_correction_indicator,v.maturity_date,v.quotation_factor,v.exercise_price_points::text,v.isin,q.instrument_id,q.resolution_status,sf.id AS source_file_id,sf.source_file_type,sf.sha256,m.id AS source_member_id,m.member_name,q.record_index";
        let rows = bind_filters!(sqlx::query(&format!(
            "{fields}{FILTER_SQL} ORDER BY f.trade_date,f.id LIMIT $11 OFFSET $12"
        )))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let items = rows
            .iter()
            .map(|row| historical_quote_from_row(row, filters.include_internal.unwrap_or(false)))
            .collect();
        let mut coverage = self.coverage(revision, purpose).await?;
        coverage.requested_from = filters.trade_date_from;
        coverage.requested_to = filters.trade_date_to;
        coverage.first_observed = summary.get("first");
        coverage.last_observed = summary.get("last");
        coverage.unresolved = summary.get("unresolved");
        Ok(QuotePage {
            items,
            total,
            limit,
            offset,
            data_revision: revision,
            disclaimer: NOMINAL_WARNING.into(),
            coverage,
        })
    }

    /// Tests whether the pinned revision contains this published ticker before opening a stream.
    pub async fn ticker_has_quotes(
        &self,
        revision: Uuid,
        ticker_symbol: &str,
    ) -> Result<bool, QueryError> {
        self.bounded(async {
            Ok(sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM b3_cotahist_facts f JOIN b3_history_revision_quotes q ON q.fact_id=f.id WHERE q.revision_id=$1 AND f.ticker_symbol=$2)",
            )
            .bind(revision)
            .bind(ticker_symbol)
            .fetch_one(&self.pool)
            .await?)
        })
        .await
    }

    /// Fetches one bounded keyset page for the immutable revision-pinned ticker export.
    pub async fn ticker_quote_batch(
        &self,
        revision: Uuid,
        ticker_symbol: &str,
        after: Option<(NaiveDate, Uuid)>,
        include_internal: bool,
        batch_size: i64,
    ) -> Result<Vec<HistoricalQuote>, QueryError> {
        self.bounded(async {
            let fields="SELECT f.id,f.trade_date,f.ticker_symbol,f.market_type,f.bdi_code,f.distribution_number,f.term_days,v.id AS version_id,v.business_hash,v.issuer_short_name,v.paper_specification,v.currency,v.open_price::text,v.high_price::text,v.low_price::text,v.average_price::text,v.last_price::text,v.best_bid_price::text,v.best_ask_price::text,v.trades_count,v.traded_quantity::text,v.traded_volume::text,v.option_exercise_price::text,v.price_correction_indicator,v.maturity_date,v.quotation_factor,v.exercise_price_points::text,v.isin,q.instrument_id,q.resolution_status,sf.id AS source_file_id,sf.source_file_type,sf.sha256,m.id AS source_member_id,m.member_name,q.record_index FROM b3_cotahist_facts f JOIN b3_history_revision_quotes q ON q.fact_id=f.id JOIN b3_cotahist_versions v ON v.id=q.version_id JOIN b3_source_file_members m ON m.id=q.source_member_id JOIN b3_source_files sf ON sf.id=m.source_file_id WHERE q.revision_id=$1 AND f.ticker_symbol=$2 AND ($3::date IS NULL OR (f.trade_date,f.id)>($3,$4)) ORDER BY f.trade_date,f.id LIMIT $5";
            let rows = sqlx::query(fields)
                .bind(revision)
                .bind(ticker_symbol)
                .bind(after.map(|cursor| cursor.0))
                .bind(after.map(|cursor| cursor.1))
                .bind(batch_size.clamp(1, 1000))
                .fetch_all(&self.pool)
                .await?;
            Ok(rows
                .iter()
                .map(|row| historical_quote_from_row(row, include_internal))
                .collect())
        })
        .await
    }

    pub async fn coverage(
        &self,
        revision: Uuid,
        purpose: QueryPurpose,
    ) -> Result<Coverage, QueryError> {
        self.resolve_revision(Some(revision), purpose).await?;
        let row=sqlx::query("SELECT min(f.trade_date) AS first,max(f.trade_date) AS last,count(*) FILTER(WHERE q.resolution_status<>'resolved') AS unresolved FROM b3_history_revision_quotes q JOIN b3_cotahist_facts f ON f.id=q.fact_id WHERE q.revision_id=$1").bind(revision).fetch_one(&self.pool).await?;
        let rejected: i64 =
            sqlx::query_scalar("SELECT rejected FROM b3_history_revisions WHERE id=$1")
                .bind(revision)
                .fetch_one(&self.pool)
                .await?;
        Ok(Coverage {
            requested_from: None,
            requested_to: None,
            status: CoverageStatus::Partial,
            first_observed: row.get("first"),
            last_observed: row.get("last"),
            rejected,
            unresolved: row.get("unresolved"),
            calendar_status: "unknown".into(),
        })
    }
}

fn historical_quote_from_row(
    row: &sqlx::postgres::PgRow,
    include_internal: bool,
) -> HistoricalQuote {
    let identity = match row.get::<String, _>("resolution_status").as_str() {
        "resolved" => IdentityStatus::Resolved,
        "ambiguous" => IdentityStatus::Ambiguous,
        _ => IdentityStatus::Unresolved,
    };
    HistoricalQuote {
        id: row.get("id"),
        trade_date: row.get("trade_date"),
        ticker_symbol: row.get("ticker_symbol"),
        market_type: row.get("market_type"),
        bdi_code: row.get("bdi_code"),
        distribution_number: row.get("distribution_number"),
        term_days: row.get("term_days"),
        issuer_short_name: row.get("issuer_short_name"),
        paper_specification: row.get("paper_specification"),
        currency: row.get("currency"),
        open_price: row.get("open_price"),
        high_price: row.get("high_price"),
        low_price: row.get("low_price"),
        average_price: row.get("average_price"),
        last_price: row.get("last_price"),
        best_bid_price: row.get("best_bid_price"),
        best_ask_price: row.get("best_ask_price"),
        trades_count: row.get("trades_count"),
        traded_quantity: row.get("traded_quantity"),
        traded_volume: row.get("traded_volume"),
        option_exercise_price: row.get("option_exercise_price"),
        price_correction_indicator: row.get("price_correction_indicator"),
        maturity_date: row.get("maturity_date"),
        quotation_factor: row.get("quotation_factor"),
        exercise_price_points: row.get("exercise_price_points"),
        isin: row.get("isin"),
        identity: QuoteIdentity {
            instrument_id: row.get("instrument_id"),
            status: identity,
        },
        provenance: QuoteProvenance {
            source_file_id: row.get("source_file_id"),
            source_file_type: row.get("source_file_type"),
            sha256: row.get("sha256"),
            source_member_id: row.get("source_member_id"),
            source_member_name: row.get("member_name"),
            record_index: row.get("record_index"),
        },
        internal: include_internal.then(|| QuoteInternal {
            version_id: row.get("version_id"),
            business_hash: row.get("business_hash"),
        }),
    }
}
