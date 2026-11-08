use super::{QueryError, QueryPurpose, QueryService, QuoteProvenance};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Busca de evidências cadastrais publicadas; não aplica cadastro atual ao histórico.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct InstrumentFilters {
    /// UUID canônico do instrumento, quando resolvido.
    pub instrument_id: Option<String>,
    /// Texto de busca por ticker, ISIN, código ou descrição.
    pub query: Option<String>,
    /// Ticker exatamente publicado.
    pub ticker_symbol: Option<String>,
    /// Código B3 qualificado pelo namespace do registro.
    pub security_id: Option<String>,
    /// ISIN da evidência.
    pub isin: Option<String>,
    /// Fotografia publicada a fixar; ausente usa fotografias ativas por família.
    pub snapshot_id: Option<Uuid>,
    /// Família cadastral a selecionar, sem fallback entre famílias.
    pub source_file_type: Option<crate::db::enums::SourceFileType>,
    /// Quantidade por página, padrão 100, máximo 1000.
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    /// Deslocamento não negativo, padrão 0.
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

/// Fotografia oficial com associação canônica quando demonstrada.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentEvidence {
    /// Instrumento B3 estável; nulo se ainda não há código forte admissível.
    pub instrument_id: Option<String>,
    /// Fotografia que sustenta estes atributos.
    pub snapshot_id: Uuid,
    /// Data de referência da fotografia, não início presumido de validade.
    pub reference_date: NaiveDate,
    /// Identificador de negociação publicado, quando informado.
    pub security_id: Option<String>,
    /// Namespace/qualificador do código.
    pub security_source: String,
    /// Local/mercado qualificador.
    pub market_identifier_code: Option<String>,
    /// Ticker observado nesta fotografia.
    pub ticker_symbol: Option<String>,
    /// ISIN observado nesta fotografia.
    pub isin: Option<String>,
    /// Descrição publicada, sem enriquecimento CVM.
    pub asset_description: Option<String>,
    /// Código de mercado da fonte.
    pub market_code: Option<String>,
    /// Tipo do instrumento conforme a família cadastral.
    pub instrument_type: Option<String>,
    /// Categoria oficial, sem heurística pelo sufixo do ticker.
    pub security_category: Option<String>,
    /// Código do ativo publicado no cadastro, sem equivalência automática com emissor.
    pub asset: Option<String>,
    /// Código do segmento publicado.
    pub segment_code: Option<String>,
    /// Classificação CFI informada.
    pub cfi_code: Option<String>,
    /// Estado oficial do instrumento na fotografia.
    pub instrument_status: Option<String>,
    /// Vencimento informado, sem sentinelas.
    pub maturity_date: Option<NaiveDate>,
    /// Indicador negociável do Tradable Security List, quando publicado.
    pub tradable_indicator: Option<bool>,
    /// Início de negociação publicado; não inferido da coleta.
    pub trading_start_date: Option<NaiveDate>,
    /// Fim de negociação publicado; não inferido da ausência na lista.
    pub trading_end_date: Option<NaiveDate>,
    /// Origem e localizador de registro.
    pub provenance: QuoteProvenance,
}
/// Página de evidências; famílias podem sustentar observações distintas do mesmo instrumento.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InstrumentPage {
    /// Registros publicados em ordem determinística por fotografia, member e linha.
    pub items: Vec<InstrumentEvidence>,
    /// Total de evidências nos filtros.
    pub total: i64,
    /// Limite efetivo.
    pub limit: i64,
    /// Deslocamento efetivo.
    pub offset: i64,
    /// Fotografias selecionadas; fixe snapshot_id para paginação de uma fotografia.
    pub snapshots: Vec<Uuid>,
}

impl QueryService {
    /// Return every currently published registry observation for one canonical instrument.
    /// There may be more than one row when source families or snapshots differ.
    pub async fn instrument(
        &self,
        id: Uuid,
        mut filters: InstrumentFilters,
        purpose: QueryPurpose,
    ) -> Result<InstrumentPage, QueryError> {
        if filters
            .instrument_id
            .as_deref()
            .is_some_and(|value| value.parse::<Uuid>().ok() != Some(id))
        {
            return Err(QueryError::Invalid(
                "instrument_id diverge do identificador da rota.".into(),
            ));
        }
        filters.instrument_id = Some(id.to_string());
        let page = self.instruments(filters, purpose).await?;
        if page.total == 0 {
            return Err(QueryError::NotFound);
        }
        Ok(page)
    }

    /// List identifier evidence from all retained published snapshots for an instrument.
    pub async fn instrument_identifiers(
        &self,
        id: Uuid,
        filters: super::ListFilters,
        purpose: QueryPurpose,
    ) -> Result<InstrumentIdentifierPage, QueryError> {
        self.bounded(async {
            let (limit, offset) = filters.validate()?;
            let sources = sqlx::query("SELECT DISTINCT s.source_file_type,s.channel FROM b3_instrument_identifiers i JOIN b3_registry_snapshots s ON s.source_file_id=i.source_file_id AND s.status='published' WHERE i.instrument_id=$1")
                .bind(id.to_string())
                .fetch_all(&self.pool)
                .await?;
            if sources.is_empty() {
                return Err(QueryError::NotFound);
            }
            let eligible: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM b3_instrument_identifiers i JOIN b3_registry_snapshots s ON s.source_file_id=i.source_file_id AND s.status='published' CROSS JOIN unnest(ARRAY[$2::text,'third_party_delivery']) p(purpose) WHERE i.instrument_id=$1 AND NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=s.source_file_type AND e.channel=s.channel AND e.purpose=p.purpose AND e.status='eligible' AND e.zero_cost AND length(trim(e.evidence_reference))>0 AND e.verified_at IS NOT NULL))")
                .bind(id.to_string())
                .bind(purpose.as_str())
                .fetch_one(&self.pool)
                .await?;
            if !eligible {
                return Err(QueryError::Forbidden);
            }
            let predicate = " FROM b3_instrument_identifiers i JOIN b3_registry_snapshots s ON s.source_file_id=i.source_file_id AND s.status='published' JOIN b3_source_files sf ON sf.id=i.source_file_id LEFT JOIN b3_source_file_members m ON m.source_file_id=i.source_file_id AND m.member_name=i.source_member_name WHERE i.instrument_id=$1";
            let total: i64 = sqlx::query_scalar(&format!("SELECT count(*){predicate}"))
                .bind(id.to_string())
                .fetch_one(&self.pool)
                .await?;
            let rows = sqlx::query(&format!("SELECT i.*,s.id AS snapshot_id,s.reference_date,sf.source_file_type,sf.sha256,m.id AS source_member_id{predicate} ORDER BY i.valid_from NULLS FIRST,i.identifier_type,i.identifier_value,i.market_code NULLS FIRST,i.id LIMIT $2 OFFSET $3"))
                .bind(id.to_string())
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;
            let items = rows
                .into_iter()
                .map(|row| InstrumentIdentifierEvidence {
                    instrument_id: id,
                    identifier_type: row.get("identifier_type"),
                    identifier_value: row.get("identifier_value"),
                    market_code: row.get("market_code"),
                    valid_from: row.get("valid_from"),
                    valid_to: row.get("valid_to"),
                    snapshot_id: row.get("snapshot_id"),
                    reference_date: row.get("reference_date"),
                    provenance: InstrumentIdentifierProvenance {
                        source_file_id: row.get("source_file_id"),
                        source_file_type: row.get("source_file_type"),
                        sha256: row.get("sha256"),
                        source_member_id: row.get("source_member_id"),
                        source_member_name: row.get("source_member_name"),
                        record_index: row.get("source_line_number"),
                        source_row_hash: row.get("source_row_hash"),
                    },
                })
                .collect();
            Ok(InstrumentIdentifierPage {
                instrument_id: id,
                items,
                total,
                limit,
                offset,
            })
        })
        .await
    }

    pub async fn instruments(
        &self,
        filters: InstrumentFilters,
        purpose: QueryPurpose,
    ) -> Result<InstrumentPage, QueryError> {
        self.bounded(self.instruments_inner(filters, purpose)).await
    }
    async fn instruments_inner(
        &self,
        filters: InstrumentFilters,
        purpose: QueryPurpose,
    ) -> Result<InstrumentPage, QueryError> {
        let limit = filters.limit.unwrap_or(100);
        if filters
            .instrument_id
            .as_ref()
            .is_some_and(|s| s.parse::<Uuid>().is_err())
        {
            return Err(QueryError::Invalid("UUID de instrumento inválido.".into()));
        }
        if matches!(filters.source_file_type,Some(source) if !matches!(source,crate::db::enums::SourceFileType::Bvbg028InstrumentsFile|crate::db::enums::SourceFileType::TradableSecurityList))
        {
            return Err(QueryError::Invalid("Família não cadastral.".into()));
        }
        let offset = filters.offset.unwrap_or(0);
        if !(1..=1000).contains(&limit) || offset < 0 {
            return Err(QueryError::Invalid("Paginação inválida.".into()));
        }
        let snapshots:Vec<Uuid>=sqlx::query_scalar("SELECT s.id FROM b3_registry_snapshots s WHERE s.status='published' AND (($1::uuid IS NOT NULL AND s.id=$1) OR ($1::uuid IS NULL AND EXISTS(SELECT 1 FROM b3_registry_active a WHERE a.snapshot_id=s.id))) AND ($2::text IS NULL OR s.source_file_type=$2) ORDER BY s.id")
            .bind(filters.snapshot_id).bind(filters.source_file_type.map(|s|s.as_str())).fetch_all(&self.pool).await?;
        if snapshots.is_empty() {
            return Err(if filters.snapshot_id.is_some() {
                QueryError::NotFound
            } else {
                QueryError::Unavailable
            });
        }
        let eligible:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM b3_registry_snapshots s CROSS JOIN unnest(ARRAY[$2::text,'third_party_delivery']) p(purpose) WHERE s.id=ANY($1) AND NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=s.source_file_type AND e.channel=s.channel AND e.purpose=p.purpose AND e.status='eligible' AND e.zero_cost AND length(trim(e.evidence_reference))>0 AND e.verified_at IS NOT NULL))")
            .bind(&snapshots).bind(purpose.as_str()).fetch_one(&self.pool).await?;
        if !eligible {
            return Err(QueryError::Forbidden);
        }
        let filter=" FROM b3_registry_records r JOIN b3_registry_snapshots s ON s.id=r.snapshot_id JOIN b3_source_file_members m ON m.id=r.source_member_id JOIN b3_source_files sf ON sf.id=m.source_file_id WHERE s.id=ANY($1) AND ($2::text IS NULL OR r.ticker_symbol=$2) AND ($3::text IS NULL OR r.security_id=$3) AND ($4::text IS NULL OR r.isin=$4) AND ($5::text IS NULL OR r.ticker_symbol ILIKE '%'||$5||'%' OR r.security_id ILIKE '%'||$5||'%' OR r.isin ILIKE '%'||$5||'%' OR r.asset_description ILIKE '%'||$5||'%') AND ($6::text IS NULL OR r.instrument_id=$6)";
        macro_rules! bind {
            ($q:expr) => {
                $q.bind(&snapshots)
                    .bind(
                        filters
                            .ticker_symbol
                            .as_ref()
                            .map(|s| s.trim().to_uppercase()),
                    )
                    .bind(&filters.security_id)
                    .bind(filters.isin.as_ref().map(|s| s.trim().to_uppercase()))
                    .bind(&filters.query)
                    .bind(&filters.instrument_id)
            };
        }
        let total: i64 = bind!(sqlx::query_scalar(&format!("SELECT count(*){filter}")))
            .fetch_one(&self.pool)
            .await?;
        let rows=bind!(sqlx::query(&format!("SELECT r.*,s.reference_date,sf.id AS source_file_id,sf.source_file_type,sf.sha256,m.member_name{filter} ORDER BY r.snapshot_id,r.source_member_id,r.record_index LIMIT $7 OFFSET $8"))).bind(limit).bind(offset).fetch_all(&self.pool).await?;
        let items = rows
            .into_iter()
            .map(|r| InstrumentEvidence {
                instrument_id: r.get("instrument_id"),
                snapshot_id: r.get("snapshot_id"),
                reference_date: r.get("reference_date"),
                security_id: r.get("security_id"),
                security_source: r.get("security_source"),
                market_identifier_code: r.get("market_identifier_code"),
                ticker_symbol: r.get("ticker_symbol"),
                isin: r.get("isin"),
                asset_description: r.get("asset_description"),
                market_code: r.get("market_code"),
                instrument_type: r.get("instrument_type"),
                security_category: r.get("security_category"),
                asset: r.get("asset"),
                segment_code: r.get("segment_code"),
                cfi_code: r.get("cfi_code"),
                instrument_status: r.get("instrument_status"),
                maturity_date: r.get("maturity_date"),
                tradable_indicator: r.get("tradable_indicator"),
                trading_start_date: r.get("trading_start_date"),
                trading_end_date: r.get("trading_end_date"),
                provenance: QuoteProvenance {
                    source_file_id: r.get("source_file_id"),
                    source_file_type: r.get("source_file_type"),
                    sha256: r.get("sha256"),
                    source_member_id: r.get("source_member_id"),
                    source_member_name: r.get("member_name"),
                    record_index: r.get("record_index"),
                },
            })
            .collect();
        Ok(InstrumentPage {
            items,
            total,
            limit,
            offset,
            snapshots,
        })
    }
}

/// A qualified identifier window supported by a published registry snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentIdentifierEvidence {
    /// Stable canonical instrument identifier.
    pub instrument_id: Uuid,
    /// Closed family such as `ticker`, `isin`, or `security_id`.
    pub identifier_type: String,
    /// Identifier exactly as normalized from the official evidence.
    pub identifier_value: String,
    /// Market qualifier; null means the source did not supply one.
    pub market_code: Option<String>,
    /// Explicit beginning of validity, when evidenced.
    pub valid_from: Option<NaiveDate>,
    /// Explicit end of validity, when evidenced.
    pub valid_to: Option<NaiveDate>,
    /// Published registry snapshot containing this observation.
    pub snapshot_id: Uuid,
    /// Snapshot reference date; it is not inferred validity.
    pub reference_date: NaiveDate,
    /// Lineage to the source artifact and record.
    pub provenance: InstrumentIdentifierProvenance,
}

/// Source lineage for an identifier observation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentIdentifierProvenance {
    /// Source artifact id.
    pub source_file_id: String,
    /// Official source family.
    pub source_file_type: String,
    /// SHA-256 of the immutable raw artifact.
    pub sha256: String,
    /// Member id when present in the retained manifest.
    pub source_member_id: Option<String>,
    /// Original deterministic member locator.
    pub source_member_name: Option<String>,
    /// One-based source record index where available.
    pub record_index: Option<i32>,
    /// Canonical source row hash.
    pub source_row_hash: String,
}

/// Revisioned page of qualified identifier evidence.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentIdentifierPage {
    /// Instrument requested.
    pub instrument_id: Uuid,
    /// Identifier observations ordered by validity, type, value, market and id.
    pub items: Vec<InstrumentIdentifierEvidence>,
    /// Total retained published evidence before pagination.
    pub total: i64,
    /// Effective page limit.
    pub limit: i64,
    /// Effective offset.
    pub offset: i64,
}
