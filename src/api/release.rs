use super::{release_error::ApiError, AppState};
use crate::services::query::{
    InstrumentFilters, InstrumentIdentifierPage, InstrumentPage, ListFilters,
};
use crate::services::{
    jobs::{JobCommand, JobPriority, JobService},
    query::{JobView, QueryError, QueryPurpose, QueryService, QuoteFilters, QuotePage},
    quote_export::{ExportError, QuoteExportEvent, QuoteExportFilters, QuoteExportService},
};
use axum::{
    body::Body,
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, Response, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(get,path="/api/v1/b3/instruments",tag="Cadastro",summary="Buscar evidências cadastrais oficiais publicadas por fotografia",description="Consulta BVBG.028/Tradable Security List em fotografias explicitamente publicadas.\nNo 0.0.1, ambas as fontes estão deferred e não podem ser importadas: os cortes from/to ainda não podem ser aplicados a uma data adequada dentro de cada registro. A rota fica no schema para evolução e pode permanecer indisponível sem snapshot suportado.\nPreserva namespace, mercado, data de referência e proveniência; não inventa início de validade.\nFamílias distintas permanecem distinguíveis; ausência de código forte não fabrica security_id.\nFiltros e paginação não disparam aquisição, resolução pesada ou publicação.",params(InstrumentFilters),responses((status=200,description="Página de evidências",body=InstrumentPage),(status=400,description="Filtro inválido"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível ou deferred"),(status=404,description="Fotografia inexistente"),(status=409,description="Conflito de identidade não escolhido arbitrariamente"),(status=500,description="Falha interna"),(status=503,description="Nenhuma fotografia publicada")),security(("bearer_auth"=[])))]
pub async fn instruments(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instruments(filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query all currently published observations for one canonical instrument.
#[utoipa::path(get,path="/api/v1/b3/instruments/{id}",tag="Cadastro",summary="Consultar evidências publicadas para um instrumento canônico",description="Retorna todas as evidências cadastrais ativas para o UUID canônico informado, sem escolher arbitrariamente uma fotografia ou família.\nNo 0.0.1, as famílias de registro estão deferred e não há ingestão suportada que produza essas evidências. A rota fica no schema para evolução.\nCada item preserva snapshot, referência, qualificador e proveniência; datas de fotografia não são convertidas em validade histórica.\nFiltros cadastrais podem reduzir a página, mas o ID da rota prevalece. A consulta não dispara aquisição, parsing ou rebuild.",params(("id"=Uuid,Path,description="UUID estável do instrumento"),InstrumentFilters),responses((status=200,description="Evidências publicadas paginadas",body=InstrumentPage),(status=400,description="ID ou filtros inválidos"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Instrumento sem evidência publicada"),(status=409,description="Conflito de identidade"),(status=500,description="Falha de dependência"),(status=503,description="Registro cadastral indisponível")),security(("bearer_auth"=[])))]
pub async fn instrument(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instrument(id, filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query temporal identifier evidence from all retained published snapshots.
#[utoipa::path(get,path="/api/v1/b3/instruments/{id}/identifiers",tag="Cadastro",summary="Consultar identificadores qualificados e janelas comprovadas",description="Lista ticker, ISIN e código de segurança com qualificadores de mercado, validade comprovada e fotografia de origem.\nNo 0.0.1, a ingestão de registros está deferred e não há evidências cadastrais produzidas pelo fluxo suportado. A rota fica no schema para evolução.\nA data de referência da fotografia não é promovida a intervalo histórico presumido.\nA paginação é determinística e cada item aponta ao artefato, member e registro disponíveis.\nA consulta somente leitura bloqueia fontes sem elegibilidade para REST e terceiros.",params(("id"=Uuid,Path,description="UUID estável do instrumento"),ListFilters),responses((status=200,description="Página de evidências temporais",body=InstrumentIdentifierPage),(status=400,description="ID ou paginação inválidos"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Instrumento sem identificadores publicados"),(status=409,description="Conflito não aplicável à consulta"),(status=500,description="Falha de dependência")),security(("bearer_auth"=[])))]
pub async fn instrument_identifiers(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    filters: Result<Query<ListFilters>, QueryRejection>,
) -> Result<Json<InstrumentIdentifierPage>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Paginação inválida.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instrument_identifiers(id, filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query one registry family without falling back to BVBG.028.
#[utoipa::path(get,path="/api/v1/b3/tradable-securities",tag="Cadastro",summary="Consultar evidências publicadas da Tradable Security List",description="Aplica exclusivamente a família Tradable Security List por meio do serviço cadastral compartilhado.\nNo 0.0.1, a família está deferred e não há ingestão suportada que produza snapshots consultáveis; a rota fica no schema para evolução.\nPreserva fotografia e proveniência, e não fabrica indicadores ausentes da projeção tipada.\nNão consulta BVBG.028 como fallback e bloqueia fontes/finalidades sem elegibilidade.\nFiltros e paginação não iniciam aquisição ou parsing.",params(InstrumentFilters),responses((status=200,description="Página da Tradable Security List",body=InstrumentPage),(status=400,description="Filtro inválido ou família conflitante"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Snapshot inexistente"),(status=409,description="Conflito de seleção"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma fotografia publicada")),security(("bearer_auth"=[])))]
pub async fn tradable_securities(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Query(mut filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    let family = crate::db::enums::SourceFileType::TradableSecurityList;
    if filters
        .source_file_type
        .is_some_and(|value| value != family)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Esta rota aceita somente a família Tradable Security List.",
        ));
    }
    filters.source_file_type = Some(family);
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instruments(filters, QueryPurpose::Rest)
            .await?,
    ))
}

impl From<QueryError> for ApiError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::Invalid(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FILTER",
                "Filtro, intervalo ou paginação inválidos.",
            ),
            QueryError::Unavailable => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "REVISION_UNAVAILABLE",
                "Nenhuma revisão publicável está disponível.",
            ),
            QueryError::NotFound => Self::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Recurso individual não encontrado.",
            ),
            QueryError::Forbidden => Self::new(
                StatusCode::FORBIDDEN,
                "SOURCE_NOT_ELIGIBLE",
                "A fonte não está habilitada para esta finalidade.",
            ),
            QueryError::Ambiguous => Self::new(
                StatusCode::CONFLICT,
                "AMBIGUOUS_SERIES",
                "Selecione mercado, distribuição e prazo para obter uma série inequívoca.",
            ),
            QueryError::Database(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_FAILED",
                "Falha na dependência de consulta.",
            ),
        }
    }
}

/// Submissão explícita de trabalho administrativo; não executa parsing no request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitJob {
    /// Chave idempotente; reuso com comando diferente retorna 409.
    pub idempotency_key: String,
    /// Operação fechada sobre fontes oficiais, arquivos e revisões.
    pub command: JobCommand,
    /// Classe de prioridade; manutenção dispõe de capacidade reservada.
    pub priority: JobPriority,
    /// Limite de tentativas, padrão 3; intervalo 1–100.
    #[schema(default = 3, minimum = 1, maximum = 100)]
    pub max_attempts: Option<i32>,
}
/// Referência de acompanhamento; 202 não significa ingestão ou publicação concluída.
#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptedJob {
    /// UUID do trabalho durável.
    pub job_id: Uuid,
    /// URL relativa de acompanhamento retornada também em Location.
    pub status_url: String,
}

#[utoipa::path(get,path="/api/v1/b3/cotahist/quotes",tag="COTAHIST",summary="Consultar fatos históricos nominais COTAHIST com revisão fixada",description="Consulta registros dos arquivos oficiais COTAHIST anual, mensal e diário da B3.\nPreserva moeda, fator de cotação, distribuição e prazo; não aplica ajustes corporativos.\nFiltros temporais são inclusivos; equivalência não duplica fatos. A revisão fixa resultados entre páginas.\nToda linha contém proveniência até member/registro e estado da identidade. Não dispara ingestão.",params(QuoteFilters),responses((status=200,description="Página na revisão selecionada",body=QuotePage),(status=400,description="Filtros inválidos",body=super::release_error::ErrorResponse),(status=401,description="Bearer ausente/inválido"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Revisão inexistente"),(status=409,description="Conflito de seleção"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma revisão disponível")),security(("bearer_auth"=[])))]
pub async fn quotes(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Rest, false)
            .await?,
    ))
}

/// Private workstation query for locally retained and processed history.
#[utoipa::path(get,path="/api/v1/b3/internal/cotahist/quotes",tag="Operação",summary="Consultar cotações COTAHIST para uso interno",description="Consulta uma revisão publicada somente em contexto administrativo interno. Exige elegibilidade de processamento e retenção bruta para todas as fontes da revisão; não habilita REST, MCP nem entrega a terceiros. Use as rotas públicas /cotahist/quotes somente quando REST e third_party_delivery também estiverem elegíveis.",params(QuoteFilters),responses((status=200,description="Página interna na revisão selecionada",body=QuotePage),(status=400,description="Filtros inválidos"),(status=401,description="Bearer ausente/inválido"),(status=403,description="Acesso administrativo ou elegibilidade interna exigidos"),(status=404,description="Revisão inexistente"),(status=409,description="Conflito de seleção"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma revisão disponível")),security(("bearer_auth"=[])))]
pub async fn internal_quotes(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Internal, false)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes/export",
    tag = "COTAHIST",
    summary = "Exportar todo o histórico COTAHIST de um ticker em NDJSON",
    description = "Origem e autoridade: exporta ocorrências das publicações oficiais COTAHIST diárias, mensais ou anuais da B3. A revisão publicada é imutável e PostgreSQL permanece a fonte de verdade.\n\nEscopo e seleção: ticker_symbol é obrigatório, normalizado com remoção de espaços externos e maiúsculas ASCII, e deve ter de 1 a 64 caracteres. O ticker é um código contextual, não uma identidade permanente; o resultado inclui suas ocorrências elegíveis na revisão selecionada, inclusive mercados, BDI, distribuições e prazos distintos. Não há filtro de data, limit ou offset neste endpoint integral.\n\nRevisão e ordenação: sem data_revision ou cursor, a revisão ativa é escolhida uma vez no início e aparece em meta. Os fatos são ordenados crescentemente por trade_date e pelo UUID único id. Cada lote usa paginação keyset limitada a 500 fatos. O fluxo não mistura revisões se outra revisão for publicada durante a leitura.\n\nRetomada e idempotência: checkpoint contém cursor opaco URL-safe sem padding, vinculado a versão do schema, ticker, revisão e última chave (trade_date,id). A chave é exclusiva: retomar continua depois do último fato confirmado. Envie o mesmo ticker e cursor; data_revision é opcional, mas, se fornecida, deve coincidir com a revisão do cursor. Repetir a mesma revisão, ticker, projeção e cursor produz a mesma representação NDJSON.\n\nFormato e interpretação: Content-Type é application/x-ndjson; charset=utf-8, com um objeto JSON por linha LF. Eventos são meta, quote, checkpoint, complete e, para uma falha após o início do corpo, error. Cada quote.data segue HistoricalQuote e conserva identidade (resolved, unresolved ou ambiguous), provenance até arquivo/member/registro e decimais como strings. Datas são ISO 8601. Os preços COTAHIST são nominais, na moeda e fator de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos; PREULT é o último negócio publicado e não deve ser renomeado como fechamento oficial.\n\nIntegridade: complete.record_count conta apenas eventos quote desta resposta. complete.content_sha256 é SHA-256 dos bytes UTF-8 de todas as linhas anteriores, inclusive seus LF finais e excluindo a linha complete. Um status HTTP 200 sem evento complete, ou com evento error, não representa exportação concluída; erros de transporte também podem truncar o fluxo.\n\nCache e eficiência: uma geração consulta PostgreSQL em lotes limitados e transmite conforme grava um artefato derivado; consumidores concorrentes da mesma chave compartilham a geração. O artefato é reconstruível, expira após 30 dias e sua chave considera revisão, ticker, schema, include_internal e cursor. REST e third_party_delivery são revalidados antes de cada leitura de cache. Respostas são privadas e variam por Authorization e Accept-Encoding; não as compartilhe em CDN/cache HTTP público. Accept-Encoding: gzip pode selecionar um artefato gzip já pronto; uma geração inicial pode ser entregue sem compressão. ETag e Content-Length só são enviados quando a representação está pronta. A API não publica SLA de latência para esta operação.",
    params(
        QuoteExportFilters,
        ("If-None-Match" = Option<String>, Header, description = "ETag forte da mesma representação (NDJSON ou gzip) recebida anteriormente. Exemplo: \"<sha256>\". Retorna 304 sem corpo somente se a representação selecionada já estiver pronta e corresponder."),
        ("Accept-Encoding" = Option<String>, Header, description = "Informe gzip para aceitar compressão quando o artefato gzip estiver pronto. Durante geração inicial, a resposta pode usar NDJSON sem compressão. Exemplo: gzip.")
    ),
    responses(
        (status = 200, description = "Corpo application/x-ndjson, uma linha UTF-8 por evento; só é completo quando contém complete e seu checksum confere. error pode aparecer depois do início do corpo.", content_type = "application/x-ndjson", body = QuoteExportEvent),
        (status = 304, description = "A representação pronta corresponde ao If-None-Match; não há corpo."),
        (status = 400, description = "Ticker vazio/maior que 64 caracteres, parâmetro desconhecido, cursor inválido ou incompatibilidade entre ticker, cursor e revisão", body = super::release_error::ErrorResponse),
        (status = 401, description = "Bearer de leitura ausente ou inválido", body = super::release_error::ErrorResponse),
        (status = 403, description = "A revisão ou finalidade não está elegível para REST/third_party_delivery", body = super::release_error::ErrorResponse),
        (status = 404, description = "Ticker sem fatos na revisão publicada selecionada", body = super::release_error::ErrorResponse),
        (status = 500, description = "Falha interna antes do início do fluxo; depois dos headers a falha é comunicada pelo evento error ou pela interrupção do corpo", body = super::release_error::ErrorResponse),
        (status = 503, description = "Credenciais não configuradas, revisão/PostgreSQL/storage indisponível ou dependência temporariamente indisponível", body = super::release_error::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn quote_export(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteExportFilters>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response<Body>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de exportação inválidos.",
        )
    })?;
    quote_export_for_purpose(state, filters, headers, QueryPurpose::Rest).await
}

/// Private workstation export. Authorization is enforced before this handler runs.
#[utoipa::path(get,path="/api/v1/b3/internal/cotahist/quotes/export",tag="Operação",summary="Exportar COTAHIST em NDJSON para uso interno",description="Mesmo contrato verificável de NDJSON da exportação pública, disponível somente com credencial administrativa e elegibilidade de processamento/retenção bruta. Não habilita REST, MCP ou entrega a terceiros. A cache de artefatos é revalidada contra essa finalidade antes de cada leitura.",params(QuoteExportFilters,("If-None-Match"=Option<String>,Header,description="ETag forte da mesma representação recebida anteriormente."),("Accept-Encoding"=Option<String>,Header,description="Informe gzip para aceitar compressão quando disponível.")),responses((status=200,description="Fluxo NDJSON verificável",content_type="application/x-ndjson",body=QuoteExportEvent),(status=304,description="Representação pronta inalterada"),(status=400,description="Filtro ou cursor inválido",body=super::release_error::ErrorResponse),(status=401,description="Bearer administrativo ausente ou inválido",body=super::release_error::ErrorResponse),(status=403,description="Acesso administrativo ou elegibilidade interna exigidos",body=super::release_error::ErrorResponse),(status=404,description="Ticker sem fatos na revisão selecionada",body=super::release_error::ErrorResponse),(status=500,description="Falha interna",body=super::release_error::ErrorResponse),(status=503,description="Exportação indisponível",body=super::release_error::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn internal_quote_export(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteExportFilters>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response<Body>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de exportação inválidos.",
        )
    })?;
    quote_export_for_purpose(state, filters, headers, QueryPurpose::Internal).await
}

async fn quote_export_for_purpose(
    state: Arc<AppState>,
    filters: QuoteExportFilters,
    headers: HeaderMap,
    purpose: QueryPurpose,
) -> Result<Response<Body>, ApiError> {
    let query = QueryService::new(state.pool.clone());
    let service =
        QuoteExportService::new(state.pool.clone(), &state.settings.b3_processed_data_dir)
            .map_err(|_| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "EXPORT_STORAGE_UNAVAILABLE",
                    "Armazenamento de exportação indisponível.",
                )
            })?;
    let prepared = service
        .prepare(&query, filters, purpose)
        .await
        .map_err(map_export_error)?;

    let accept_gzip = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|encoding| {
                let mut parts = encoding.trim().split(';');
                parts
                    .next()
                    .is_some_and(|name| name.trim().eq_ignore_ascii_case("gzip"))
                    && !parts.any(|part| {
                        part.trim()
                            .strip_prefix("q=")
                            .and_then(|value| value.parse::<f32>().ok())
                            .is_some_and(|quality| quality <= 0.0)
                    })
            })
        });
    let representation = service.ready_representation(&prepared, accept_gzip);
    if let Some((_, sha256, _, _)) = &representation {
        let etag = format!("\"{sha256}\"");
        let not_modified = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value.split(',').any(|candidate| {
                    let candidate = candidate.trim();
                    candidate == "*" || candidate == etag || candidate == format!("W/{etag}")
                })
            });
        if not_modified {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            set_export_headers(response.headers_mut(), Some(&etag), None, false);
            return Ok(response);
        }
    }

    let mut response = Response::new(Body::from_stream(
        service.stream(prepared, representation.clone()),
    ));
    *response.status_mut() = StatusCode::OK;
    let (etag, length, gzip) = representation
        .map(|(_, sha, size, gzip)| (Some(format!("\"{sha}\"")), Some(size), gzip))
        .unwrap_or((None, None, false));
    set_export_headers(response.headers_mut(), etag.as_deref(), length, gzip);
    Ok(response)
}

fn set_export_headers(
    headers: &mut axum::http::HeaderMap,
    etag: Option<&str>,
    content_length: Option<u64>,
    gzip: bool,
) {
    headers.insert(
        header::CONTENT_TYPE,
        "application/x-ndjson; charset=utf-8"
            .parse()
            .expect("static header"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        "private, max-age=0, must-revalidate"
            .parse()
            .expect("static header"),
    );
    headers.insert(
        header::VARY,
        "Authorization, Accept-Encoding"
            .parse()
            .expect("static header"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"cotahist-quotes.ndjson\""
            .parse()
            .expect("static header"),
    );
    if let Some(etag) = etag {
        if let Ok(value) = etag.parse() {
            headers.insert(header::ETAG, value);
        }
    }
    if let Some(length) = content_length {
        if let Ok(value) = length.to_string().parse() {
            headers.insert(header::CONTENT_LENGTH, value);
        }
    }
    if gzip {
        headers.insert(
            header::CONTENT_ENCODING,
            "gzip".parse().expect("static header"),
        );
    }
}

fn map_export_error(error: ExportError) -> ApiError {
    match error {
        ExportError::Invalid => ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Ticker, revisão ou cursor de exportação inválido.",
        ),
        ExportError::NotFound => ApiError::new(
            StatusCode::NOT_FOUND,
            "TICKER_NOT_FOUND",
            "Ticker sem fatos na revisão selecionada.",
        ),
        ExportError::Query(error) => error.into(),
        ExportError::Unavailable | ExportError::Storage | ExportError::Database(_) => {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "EXPORT_UNAVAILABLE",
                "A exportação está temporariamente indisponível.",
            )
        }
    }
}

#[utoipa::path(get,path="/api/v1/b3/cotahist/series",tag="COTAHIST",summary="Consultar série COTAHIST inequívoca com preços nominais sem ajuste",description="Seleciona fatos oficiais da família COTAHIST para ticker, ISIN ou instrumento temporal.\nExige desambiguação se mais de um mercado/distribuição/prazo permanecer no mesmo pregão.\nPREULT significa último negócio publicado, sem redefinir fechamento oficial.\nPaginação, precisão, proveniência e revisão seguem a mesma consulta de fatos; não faz forward-fill ou ingestão.",params(QuoteFilters),responses((status=200,description="Página da série selecionada",body=QuotePage),(status=400,description="Filtros inválidos"),(status=401,description="Não autenticado"),(status=403,description="Finalidade não elegível"),(status=404,description="Revisão inexistente"),(status=409,description="Série ambígua"),(status=500,description="Falha interna"),(status=503,description="Revisão indisponível")),security(("bearer_auth"=[])))]
pub async fn series(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Rest, true)
            .await?,
    ))
}

/// Single-observation-per-session internal series query.
#[utoipa::path(get,path="/api/v1/b3/internal/cotahist/series",tag="Operação",summary="Consultar série COTAHIST para uso interno",description="Seleciona uma observação inequívoca por pregão em uma revisão publicada. Exige credencial administrativa e elegibilidade interna de processamento e retenção bruta; não habilita entrega externa.",params(QuoteFilters),responses((status=200,description="Página interna da série selecionada",body=QuotePage),(status=400,description="Filtros inválidos"),(status=401,description="Não autenticado"),(status=403,description="Acesso administrativo ou elegibilidade interna exigidos"),(status=404,description="Revisão inexistente"),(status=409,description="Série ambígua"),(status=500,description="Falha de dependência"),(status=503,description="Revisão indisponível")),security(("bearer_auth"=[])))]
pub async fn internal_series(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Internal, true)
            .await?,
    ))
}

#[utoipa::path(post,path="/api/v1/b3/jobs",tag="Operação",summary="Enfileirar operação administrativa durável sobre fontes e revisões B3",description="Persiste a intenção em PostgreSQL, sem baixar ou interpretar arquivos durante o request.\nNo 0.0.1, ingest aceita somente COTAHIST anual, mensal ou diário, com cortes inclusivos aplicados a DATA DO PREGÃO de cada detalhe, ainda que o arquivo adquirido cubra um ano inteiro. BVBG.028 e Tradable Security List permanecem deferred até que a data interna de cada registro sustente esses mesmos cortes; data de fotografia/publicação não substitui esse dado. Upload por consumidores não é aceito.\nUm import limpo passa pelos gates de qualidade, processamento e retenção raw e publica a revisão completa automaticamente. Rejeições ou conflitos mantêm a candidata fora da visão ativa e retornam succeeded_with_issues para decisão administrativa. Publicação seleciona a revisão internamente; não concede elegibilidade para REST, MCP ou entrega a terceiros, que continuam sendo verificadas por finalidade.\nwithdraw_window remove fatos das datas inclusivas da visão ativa por uma nova revisão lógica; raw, versões, proveniência e revisões anteriores são preservadas. Não há hard purge neste contrato.\nA mesma chave e comando retorna o mesmo trabalho; pedido incompatível retorna 409. Requer credencial administrativa. O operador acompanha fase e resultado pela URL Location.",request_body=SubmitJob,responses((status=202,description="Trabalho aceito",body=AcceptedJob),(status=200,description="Consulta do trabalho pela localização indicada"),(status=400,description="Pedido inválido ou fonte sem corte por data interna"),(status=401,description="Não autenticado"),(status=403,description="Credencial sem administração"),(status=404,description="Recurso referenciado ausente durante execução"),(status=409,description="Conflito idempotente ou de escopo"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn submit_job(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<SubmitJob>, JsonRejection>,
) -> Result<(StatusCode, HeaderMap, Json<AcceptedJob>), ApiError> {
    let Json(payload) = payload.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Comando administrativo inválido.",
        )
    })?;
    let id = JobService::new(state.pool.clone())
        .submit(
            &payload.idempotency_key,
            payload.command,
            payload.priority,
            payload.max_attempts.unwrap_or(3),
        )
        .await?;
    let status_url = format!("/api/v1/b3/jobs/{id}");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::LOCATION,
        status_url.parse().map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LOCATION_FAILED",
                "Falha ao construir acompanhamento.",
            )
        })?,
    );
    Ok((
        StatusCode::ACCEPTED,
        headers,
        Json(AcceptedJob {
            job_id: id,
            status_url,
        }),
    ))
}

#[utoipa::path(get,path="/api/v1/b3/jobs/{id}",tag="Operação",summary="Acompanhar trabalho administrativo, tentativas e heartbeat",description="Lê o estado durável PostgreSQL de operação sobre fontes oficiais B3.\nExpõe apenas comando fechado e códigos de erro seguros, sem SQL ou tokens.\nNão interpreta arquivos nem executa retries durante a consulta; leitura não altera o trabalho.\nRequer credencial administrativa.",params(("id"=Uuid,Path,description="UUID do trabalho")),responses((status=200,description="Estado durável",body=JobView),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Trabalho inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn job(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
) -> Result<Json<JobView>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    Ok(Json(QueryService::new(state.pool.clone()).job(id).await?))
}

#[utoipa::path(post,path="/api/v1/b3/jobs/{id}/cancel",tag="Operação",summary="Solicitar cancelamento durável de trabalho administrativo",description="Persiste cancelamento de job enfileirado ou em execução.\nExecutores verificam a solicitação nos limites de download/member/lote/publicação.\nConfirmações anteriores preservam sua evidência; não apaga raw nem auditoria.\nOperação idempotente e exclusiva de administração; não garante interrupção antes de um commit que já venceu a corrida.",params(("id"=Uuid,Path,description="UUID do trabalho")),responses((status=202,description="Solicitação registrada",body=AcceptedJob),(status=200,description="Acompanhamento disponível em Location"),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Trabalho inexistente"),(status=409,description="Trabalho já terminal"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn cancel_job(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
) -> Result<(StatusCode, Json<AcceptedJob>), ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    QueryService::new(state.pool.clone()).job(id).await?;
    if !JobService::new(state.pool.clone())
        .request_cancel(id)
        .await?
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "JOB_TERMINAL",
            "Trabalho já encerrado.",
        ));
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(AcceptedJob {
            job_id: id,
            status_url: format!("/api/v1/b3/jobs/{id}"),
        }),
    ))
}
