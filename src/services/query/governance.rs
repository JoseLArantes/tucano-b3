use super::{QueryError, QueryService};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Paginação de inspeção administrativa.
#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListFilters {
    /// Quantidade por página, padrão 100, máximo 1000.
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    /// Deslocamento não negativo, padrão 0.
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}
impl ListFilters {
    pub fn validate(&self) -> Result<(i64, i64), QueryError> {
        let l = self.limit.unwrap_or(100);
        let o = self.offset.unwrap_or(0);
        if !(1..=1000).contains(&l) || o < 0 {
            return Err(QueryError::Invalid("Paginação inválida.".into()));
        }
        Ok((l, o))
    }
}
/// Página operacional com ordenação e desempate determinísticos.
#[derive(Serialize, ToSchema)]
pub struct OperationPage<T> {
    /// Registros selecionados.
    pub items: Vec<T>,
    /// Limite efetivo.
    pub limit: i64,
    /// Deslocamento efetivo.
    pub offset: i64,
}
/// Referências de auditoria do item operacional.
#[derive(Serialize, ToSchema)]
pub struct OperationProvenance {
    /// Arquivo bruto relacionado, quando aplicável.
    pub source_file_id: Option<String>,
    /// Revisão relacionada, quando aplicável.
    pub data_revision: Option<Uuid>,
}
/// Estados fechados da revisão.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RevisionState {
    Draft,
    Ready,
    Published,
    Invalid,
}
/// Família do conjunto inspecionado pela operação de publicação.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RevisionFamily {
    Cotahist,
    Registry,
}
/// Inspeção do candidato sem interpretação ou publicação em tempo de consulta.
#[derive(Serialize, ToSchema)]
pub struct RevisionView {
    /// Distingue revisão de cotações de fotografia cadastral.
    pub family: RevisionFamily,
    /// UUID da revisão.
    pub id: Uuid,
    /// Revisão-base usada pelo candidato.
    pub base_revision_id: Option<Uuid>,
    /// Início do período declarado.
    pub period_start: NaiveDate,
    /// Fim do período declarado, sem inferência de continuidade.
    pub period_end: NaiveDate,
    /// Estado de construção/publicação.
    pub status: RevisionState,
    /// Members obrigatórios esperados.
    pub expected_members: i32,
    /// Ocorrências rejeitadas.
    pub rejected: i64,
    /// Conflitos sem precedência admissível.
    pub conflicts: i64,
    /// Momento de criação, UTC.
    pub created_at: DateTime<Utc>,
    /// Momento da publicação, quando realizada.
    pub published_at: Option<DateTime<Utc>>,
    /// Artefato e revisão.
    pub provenance: OperationProvenance,
}
/// Divergência preservada, sem escolha arbitrária da versão vencedora.
#[derive(Serialize, ToSchema)]
pub struct ConflictView {
    /// Identidade estável do fato.
    pub fact_id: Uuid,
    /// Versão selecionada anteriormente.
    pub existing_version_id: Uuid,
    /// Versão divergente recebida.
    pub incoming_version_id: Uuid,
    /// Member da ocorrência divergente.
    pub source_member_id: String,
    /// Registro físico original.
    pub record_index: i64,
    /// Origem rastreável.
    pub provenance: OperationProvenance,
}
/// Artefato bruto sem URL tokenizada ou caminho privado.
#[derive(Serialize, ToSchema)]
pub struct SourceFileView {
    /// UUID do artefato.
    pub id: String,
    /// Família B3.
    pub source_file_type: String,
    /// Canal cadastrado de aquisição.
    pub acquisition_channel: String,
    /// SHA-256 dos bytes.
    pub sha256: String,
    /// Tamanho recebido, em bytes.
    pub byte_size: Option<i64>,
    /// Versão do parser utilizado.
    pub parser_version: Option<String>,
    /// Estado operacional persistido.
    pub status: String,
    /// Total de members inventariados.
    pub member_count: Option<i32>,
    /// Registros brutos lidos.
    pub row_count_raw: Option<i32>,
    /// Fatos inseridos em lotes confirmados.
    pub row_count_inserted: Option<i32>,
    /// Fatos cujo conteúdo de negócio mudou.
    pub row_count_changed: Option<i32>,
    /// Ocorrências canonicamente inalteradas.
    pub row_count_unchanged: Option<i32>,
    /// Duplicatas equivalentes no escopo, sem dupla contagem.
    pub row_count_duplicate: i64,
    /// Ocorrências rejeitadas.
    pub row_count_quarantined: Option<i32>,
    /// Identidade da origem.
    pub provenance: OperationProvenance,
}
/// Member inventariado, selecionado ou excluído.
#[derive(Serialize, ToSchema)]
pub struct MemberView {
    /// UUID do member.
    pub id: String,
    /// Caminho lógico aninhado do member.
    pub member_name: String,
    /// Índice determinístico de inventário.
    pub member_index: i32,
    /// Índice do container pai, se aninhado.
    pub parent_index: Option<i32>,
    /// SHA-256 do conteúdo descomprimido.
    pub sha256: String,
    /// Tamanho descomprimido, em bytes.
    pub byte_size: Option<i64>,
    /// Seleção para interpretação.
    pub selected: bool,
    /// Motivo de exclusão.
    pub exclusion_reason: Option<String>,
    /// Estado persistido do member.
    pub status: String,
    /// Registros de detalhe efetivamente lidos.
    pub row_count_raw: Option<i32>,
    /// Inserções em lotes confirmados.
    pub row_count_inserted: Option<i32>,
    /// Alterações de negócio confirmadas.
    pub row_count_changed: Option<i32>,
    /// Ocorrências equivalentes já conhecidas.
    pub row_count_unchanged: Option<i32>,
    /// Duplicatas equivalentes no mesmo escopo.
    pub row_count_duplicate: i64,
    /// Rejeições localizadas.
    pub row_count_quarantined: Option<i32>,
    /// Arquivo original.
    pub provenance: OperationProvenance,
}
/// Rejeição localizada; payload bruto omitido da inspeção comum.
#[derive(Serialize, ToSchema)]
pub struct RejectionView {
    /// Member original.
    pub source_member_id: String,
    /// Índice físico da ocorrência.
    pub record_index: i64,
    /// Offset em bytes quando conhecido.
    pub byte_offset: Option<i64>,
    /// Razão estável da rejeição.
    pub reason_code: String,
    /// Mensagem de interpretação sem credenciais.
    pub message: String,
    /// Revisão e arquivo de origem.
    pub provenance: OperationProvenance,
}
/// Publicação potencial identificada por descoberta leve.
#[derive(Serialize, ToSchema)]
pub struct PendingView {
    /// UUID da pendência.
    pub id: String,
    /// Canal cadastrado.
    pub channel_id: Option<String>,
    /// Família B3.
    pub source_file_type: String,
    /// Data de referência quando singular.
    pub reference_date: Option<NaiveDate>,
    /// Estado persistido da pendência.
    pub status: String,
    /// Momento de descoberta, UTC.
    pub detected_at: DateTime<Utc>,
}
impl QueryService {
    pub async fn revision(&self, id: Uuid) -> Result<RevisionView, QueryError> {
        let r = sqlx::query("SELECT id,base_revision_id,period_start,period_end,status,expected_members,rejected,conflicts,created_at,published_at,source_file_id,'cotahist'::text AS family FROM b3_history_revisions WHERE id=$1 UNION ALL SELECT id,base_snapshot_id,reference_date,reference_date,status,expected_members,rejected,0::bigint,created_at,published_at,source_file_id,'registry'::text FROM b3_registry_snapshots WHERE id=$1 AND NOT EXISTS(SELECT 1 FROM b3_history_revisions WHERE id=$1)")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(QueryError::NotFound)?;
        let status = serde_json::from_value(serde_json::Value::String(r.get("status")))
            .map_err(|_| QueryError::Unavailable)?;
        Ok(RevisionView {
            family: serde_json::from_value(serde_json::Value::String(r.get("family")))
                .map_err(|_| QueryError::Unavailable)?,
            id: r.get("id"),
            base_revision_id: r.get("base_revision_id"),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            status,
            expected_members: r.get("expected_members"),
            rejected: r.get("rejected"),
            conflicts: r.get("conflicts"),
            created_at: r.get("created_at"),
            published_at: r.get("published_at"),
            provenance: OperationProvenance {
                source_file_id: Some(r.get("source_file_id")),
                data_revision: Some(id),
            },
        })
    }
    pub async fn conflicts(
        &self,
        id: Uuid,
        filters: ListFilters,
    ) -> Result<OperationPage<ConflictView>, QueryError> {
        self.revision(id).await?;
        let (l, o) = filters.validate()?;
        let rows=sqlx::query("SELECT c.*,m.source_file_id FROM b3_history_conflicts c JOIN b3_source_file_members m ON m.id=c.source_member_id WHERE revision_id=$1 ORDER BY fact_id,incoming_version_id LIMIT $2 OFFSET $3").bind(id).bind(l).bind(o).fetch_all(&self.pool).await?;
        Ok(OperationPage {
            items: rows
                .into_iter()
                .map(|r| ConflictView {
                    fact_id: r.get("fact_id"),
                    existing_version_id: r.get("existing_version_id"),
                    incoming_version_id: r.get("incoming_version_id"),
                    source_member_id: r.get("source_member_id"),
                    record_index: r.get("record_index"),
                    provenance: OperationProvenance {
                        source_file_id: Some(r.get("source_file_id")),
                        data_revision: Some(id),
                    },
                })
                .collect(),
            limit: l,
            offset: o,
        })
    }
    pub async fn rejections(
        &self,
        id: Uuid,
        filters: ListFilters,
    ) -> Result<OperationPage<RejectionView>, QueryError> {
        self.revision(id).await?;
        let (l, o) = filters.validate()?;
        let rows=sqlx::query("SELECT r.*,m.source_file_id FROM (SELECT source_member_id,record_index,byte_offset,reason_code,message FROM b3_history_rejections WHERE revision_id=$1 UNION ALL SELECT source_member_id,record_index,byte_offset,reason_code,message FROM b3_registry_rejections WHERE snapshot_id=$1) r JOIN b3_source_file_members m ON m.id=r.source_member_id ORDER BY source_member_id,record_index LIMIT $2 OFFSET $3").bind(id).bind(l).bind(o).fetch_all(&self.pool).await?;
        Ok(OperationPage {
            items: rows
                .into_iter()
                .map(|r| RejectionView {
                    source_member_id: r.get("source_member_id"),
                    record_index: r.get("record_index"),
                    byte_offset: r.get("byte_offset"),
                    reason_code: r.get("reason_code"),
                    message: r.get("message"),
                    provenance: OperationProvenance {
                        source_file_id: Some(r.get("source_file_id")),
                        data_revision: Some(id),
                    },
                })
                .collect(),
            limit: l,
            offset: o,
        })
    }
    pub async fn source_files(
        &self,
        filters: ListFilters,
    ) -> Result<OperationPage<SourceFileView>, QueryError> {
        let (l, o) = filters.validate()?;
        let rows=sqlx::query("SELECT id,source_file_type,acquisition_channel,sha256,byte_size,parser_version,status,member_count,row_count_raw,row_count_inserted,row_count_changed,row_count_unchanged,row_count_duplicate,row_count_quarantined FROM b3_source_files ORDER BY created_at,id LIMIT $1 OFFSET $2").bind(l).bind(o).fetch_all(&self.pool).await?;
        Ok(OperationPage {
            items: rows
                .into_iter()
                .map(|r| SourceFileView {
                    id: r.get("id"),
                    source_file_type: r.get("source_file_type"),
                    acquisition_channel: r.get("acquisition_channel"),
                    sha256: r.get("sha256"),
                    byte_size: r.get("byte_size"),
                    parser_version: r.get("parser_version"),
                    status: r.get("status"),
                    member_count: r.get("member_count"),
                    row_count_raw: r.get("row_count_raw"),
                    row_count_inserted: r.get("row_count_inserted"),
                    row_count_changed: r.get("row_count_changed"),
                    row_count_unchanged: r.get("row_count_unchanged"),
                    row_count_duplicate: r.get("row_count_duplicate"),
                    row_count_quarantined: r.get("row_count_quarantined"),
                    provenance: OperationProvenance {
                        source_file_id: Some(r.get("id")),
                        data_revision: None,
                    },
                })
                .collect(),
            limit: l,
            offset: o,
        })
    }
    pub async fn members(
        &self,
        id: String,
        filters: ListFilters,
    ) -> Result<OperationPage<MemberView>, QueryError> {
        let (l, o) = filters.validate()?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_source_files WHERE id=$1)")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        if !exists {
            return Err(QueryError::NotFound);
        }
        let rows=sqlx::query("SELECT id,member_name,member_index,parent_index,sha256,byte_size,selected,exclusion_reason,status,row_count_raw,row_count_inserted,row_count_changed,row_count_unchanged,row_count_duplicate,row_count_quarantined FROM b3_source_file_members WHERE source_file_id=$1 ORDER BY member_index,id LIMIT $2 OFFSET $3").bind(&id).bind(l).bind(o).fetch_all(&self.pool).await?;
        Ok(OperationPage {
            items: rows
                .into_iter()
                .map(|r| MemberView {
                    id: r.get("id"),
                    member_name: r.get("member_name"),
                    member_index: r.get("member_index"),
                    parent_index: r.get("parent_index"),
                    sha256: r.get("sha256"),
                    byte_size: r.get("byte_size"),
                    selected: r.get("selected"),
                    exclusion_reason: r.get("exclusion_reason"),
                    status: r.get("status"),
                    row_count_raw: r.get("row_count_raw"),
                    row_count_inserted: r.get("row_count_inserted"),
                    row_count_changed: r.get("row_count_changed"),
                    row_count_unchanged: r.get("row_count_unchanged"),
                    row_count_duplicate: r.get("row_count_duplicate"),
                    row_count_quarantined: r.get("row_count_quarantined"),
                    provenance: OperationProvenance {
                        source_file_id: Some(id.clone()),
                        data_revision: None,
                    },
                })
                .collect(),
            limit: l,
            offset: o,
        })
    }
    pub async fn pending(
        &self,
        filters: ListFilters,
    ) -> Result<OperationPage<PendingView>, QueryError> {
        let (l, o) = filters.validate()?;
        let rows=sqlx::query("SELECT id,channel_id,source_file_type,reference_date,status,detected_at FROM b3_pending_updates ORDER BY detected_at,id LIMIT $1 OFFSET $2").bind(l).bind(o).fetch_all(&self.pool).await?;
        Ok(OperationPage {
            items: rows
                .into_iter()
                .map(|r| PendingView {
                    id: r.get("id"),
                    channel_id: r.get("channel_id"),
                    source_file_type: r.get("source_file_type"),
                    reference_date: r.get("reference_date"),
                    status: r.get("status"),
                    detected_at: r.get("detected_at"),
                })
                .collect(),
            limit: l,
            offset: o,
        })
    }
}
