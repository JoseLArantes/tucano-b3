use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Parâmetros de paginação e controle de metadados internos para listagens da API B3.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationParams {
    /// Quantidade máxima de registros retornados na página (padrão: 100, mín: 1, máx: 1000).
    #[param(default = 100, minimum = 1, maximum = 1000, example = 100)]
    pub limit: Option<i64>,

    /// Quantidade de registros a ignorar a partir do início da consulta (offset para paginação sequencial).
    #[param(default = 0, minimum = 0, example = 0)]
    pub offset: Option<i64>,

    /// Se verdadeiro (`true`), inclui metadados operacionais internos (`internal_metadata`) como timestamps granulares de alteração por campo e status de resolução.
    #[param(default = false, example = false)]
    pub include_internal: Option<bool>,
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).clamp(1, 1000)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn include_internal(&self) -> bool {
        self.include_internal.unwrap_or(false)
    }
}

/// Metadados de rastreabilidade e proveniência obrigatórios em todos os registros de dados de mercado da B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Provenance {
    /// Identificador da família oficial do arquivo fonte B3 (ex.: `BVBG_028_INSTRUMENTS_FILE`, `BVBG_086_PRICE_REPORT`, `COTAHIST_DAILY`).
    #[schema(example = "BVBG_028_INSTRUMENTS_FILE")]
    pub source_file_type: String,

    /// Identificador único (UUID) do registro de arquivo físico em `b3_source_files`.
    #[schema(example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")]
    pub source_file_id: String,

    /// Nome original do arquivo físico ou arquivo ZIP baixado dos canais oficiais da B3.
    #[schema(example = "IN260301.zip")]
    pub source_file_name: Option<String>,

    /// Nome do arquivo membro interno descompactado dentro do arquivo ZIP container (ex.: XML ou CSV).
    #[schema(example = "IN260301.xml")]
    pub source_member_name: Option<String>,

    /// Data de referência oficial de mercado do arquivo (formato ISO-8601: YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date: Option<NaiveDate>,

    /// Hash criptográfico SHA-256 do arquivo original para verificação de integridade e deduplicação.
    #[schema(example = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")]
    pub sha256: Option<String>,

    /// Timestamp UTC de quando o arquivo foi processado e persistido no banco de dados.
    #[schema(example = "2026-03-01T22:30:00Z")]
    pub ingested_at: Option<DateTime<Utc>>,
}

/// Metadados internos e operacionais do ciclo de vida do registro (exibidos quando `include_internal=true`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalMetadata {
    /// Timestamp UTC da última modificação real em algum campo de negócio do registro. Não se altera quando o registro é apenas re-observado sem mudanças.
    #[schema(example = "2026-03-01T22:35:10Z")]
    pub data_changed_at: Option<DateTime<Utc>>,

    /// Objeto JSON com timestamps UTC específicos para cada coluna/campo alterado individualmente.
    #[schema(example = json!({"asset_description": "2026-03-01T22:35:10Z"}))]
    pub field_changed_at: Option<serde_json::Value>,

    /// Status de amarração cadastral com a raiz de instrumentos (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: Option<String>,

    /// Timestamp UTC de quando o registro foi observado mais recentemente em um arquivo oficial da B3.
    #[schema(example = "2026-03-01T22:35:10Z")]
    pub last_seen_at: Option<DateTime<Utc>>,

    /// Número sequencial da linha ou índice da entrada no arquivo original de origem.
    #[schema(example = 1042)]
    pub source_line_number: Option<i32>,
}

/// Envelope paginado padrão de resposta da API Tucano B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Lista de itens do tipo retornado na página atual.
    pub items: Vec<T>,

    /// Quantidade total de registros que satisfazem os critérios de filtro fornecidos.
    #[schema(example = 1520)]
    pub total: i64,

    /// Limite de registros aplicado por página.
    #[schema(example = 100)]
    pub limit: i64,

    /// Offset de paginação aplicado.
    #[schema(example = 0)]
    pub offset: i64,

    /// Aviso regulatório ou metodológico obrigatório associado à família de dados (ex.: aviso de preços unadjusted para COTAHIST).
    #[schema(
        example = "AVISO: As cotacoes da serie historica COTAHIST representam precos brutos nominais de negociacao diaria, sem ajustes retroativos por dividendos, JCP, desdobramentos ou bonificacoes corporativas."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,
}

/// Resposta de diagnóstico de conectividade e integridade do serviço.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Status geral da aplicação (`ok` quando saudável).
    #[schema(example = "ok")]
    pub status: String,

    /// Nome identificador do componente ou serviço.
    #[schema(example = "tucano-b3")]
    pub app: String,

    /// Versão do pacote compilado da aplicação.
    #[schema(example = "0.0.1")]
    pub version: String,

    /// Timestamp UTC do momento da checagem.
    #[schema(example = "2026-11-05T23:55:00Z")]
    pub timestamp: DateTime<Utc>,

    /// Status da conexão ativa com o banco PostgreSQL 16 (`connected` ou mensagem de erro).
    #[schema(example = "connected")]
    pub database: String,
}

/// Requisição para disparo de sincronização manual ou reprocessamento forçado de arquivos B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManualSyncRequest {
    /// Tipo oficial da fonte de dados a ser sincronizada (ex.: `bvbg028_instruments_file`, `bvbg086_price_report`, `bvbg186_equities_simplified_price_report`, `bvbg087_index_report`, `tradable_security_list`, `b3_index_portfolio_composition`, `cotahist_daily`, `cotahist_yearly`).
    #[schema(example = "bvbg028_instruments_file")]
    pub source_file_type: String,

    /// Data inicial do intervalo de referência (formato YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date_from: NaiveDate,

    /// Data final do intervalo de referência (formato YYYY-MM-DD). Para uploads pontuais ou arquivos únicos, deve coincidir com `reference_date_from`.
    #[schema(example = "2026-03-01")]
    pub reference_date_to: NaiveDate,

    /// URL externa opcional de download (deve pertencer a um dos domínios em `B3_ALLOWED_HOSTS`). Se omitida, a URL padrão da B3 para a família será utilizada.
    #[schema(example = "https://www.b3.com.br/pesquisapregao/download?file=IN260301.zip")]
    pub source_url: Option<String>,

    /// Se verdadeiro (`true`), força o download do arquivo externo mesmo que já exista uma cópia local intacta.
    #[schema(example = false)]
    pub force_download: Option<bool>,

    /// Se verdadeiro (`true`), força o reprocessamento e substituição transacional dos dados mesmo que o arquivo já tenha sido importado com mesmo hash SHA-256.
    #[schema(example = false)]
    pub force_reparse: Option<bool>,
}

/// Resumo da execução de um job de sincronização de dados B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManualSyncResponse {
    /// Identificador único (UUID) da execução em `b3_sync_runs`.
    #[schema(example = "f54b6c31-9a72-4d56-8e12-32a76fbc9081")]
    pub sync_run_id: String,

    /// Status final da execução (`completed`, `failed`, `running`).
    #[schema(example = "completed")]
    pub status: String,

    /// Quantidade de registros novos inseridos no banco relacional.
    #[schema(example = 12040)]
    pub inserted: i32,

    /// Quantidade de registros existentes que tiveram ao menos um campo de negócio alterado e auditado.
    #[schema(example = 15)]
    pub changed: i32,

    /// Quantidade de registros existentes cujo conteúdo era canonicamente idêntico (zero escritas redundantes).
    #[schema(example = 108340)]
    pub unchanged: i32,

    /// Quantidade de registros que não puderam ser associados imediatamente a um instrumento conhecido.
    #[schema(example = 0)]
    pub unresolved: i32,

    /// Lista de mensagens de alerta, avisos de validação ou anomalias não-fatais encontradas durante o processamento.
    pub warnings: Vec<String>,
}

/// Resposta do processo de reconstrução retroativa de relacionamentos cadastrais.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RebuildIdentityResponse {
    /// Quantidade total de registros previamente pendentes que foram vinculados com sucesso a instrumentos cadastrais.
    #[schema(example = 342)]
    pub resolved_count: i32,

    /// Mensagem descritiva do resultado da operação.
    #[schema(example = "Rebuilt identity relationships; resolved 342 previously pending items.")]
    pub message: String,
}

/// Resposta da operação de reprocessamento em lote de registros em quarentena.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuarantineReplayResponse {
    /// Quantidade de registros da quarentena marcados para reprocessamento.
    #[schema(example = 12)]
    pub reprocessed_count: i32,

    /// Mensagem descritiva da operação executada.
    #[schema(example = "Replayed 12 quarantine records.")]
    pub message: String,
}

/// Métrica consolidada de disponibilidade temporal e cobertura por tabela/família de dados B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CoverageMetric {
    /// Nome da tabela relacional auditada (ex.: `b3_instruments`, `b3_cotahist_quotes`).
    #[schema(example = "b3_instruments")]
    pub dataset: String,

    /// Data de mercado mais antiga disponível na tabela (formato YYYY-MM-DD).
    #[schema(example = "1986-01-02")]
    pub min_date: Option<NaiveDate>,

    /// Data de mercado mais recente registrada na tabela (formato YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub max_date: Option<NaiveDate>,

    /// Quantidade total de linhas/registros persistidos nesta tabela.
    #[schema(example = 120562)]
    pub total_records: i64,

    /// Quantidade total de arquivos fontes distintos que contribuíram para os registros desta tabela.
    #[schema(example = 36)]
    pub total_files: i64,
}
