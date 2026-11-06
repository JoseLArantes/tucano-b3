# Guia de consumo da API Tucano B3

Status: referência do contrato HTTP implementado no release ``0.0.1``, revisada em **06/11/2026**. Este guia descreve as rotas registradas pelo router atual, seus filtros, respostas, permissões e relações. O PRD descreve capacidades-alvo; uma rota presente no PRD, em código legado ou numa fonte B3 não está disponível por isso. O [OpenAPI gerado](#1-descoberta-da-api-e-ambiente) e o router do release são a referência para confirmar o contrato executado.

O foco é permitir que uma interface web carregue dados de forma previsível, mantenha identidade e revisão entre telas, e explique ao usuário quando um dado está ausente, ambíguo ou incompleto. Exemplos com ``EXMP3`` e IDs fictícios são sintéticos.

## 1. Descoberta da API e ambiente

Use a origem configurada para o ambiente. No Compose de desenvolvimento, a API fica em ``http://localhost:8010``; não fixe essa origem em um bundle de produção.

| Recurso | Caminho | Uso |
|---|---|---|
| OpenAPI 3.1 | ``GET /openapi.json`` | Contratos e schemas do binário em execução; rota pública. |
| Página de documentação | ``GET /docs`` ou ``GET /docs/`` | Página simples com link para ``/openapi.json``; não é um console Swagger interativo. |
| Liveness | ``GET /healthz`` | Confirma que o processo HTTP respondeu. Não consulta PostgreSQL nem mede cobertura. |
| Readiness de banco | ``GET /api/v1/health`` | Executa ``SELECT 1`` no PostgreSQL. Não mede se há dados ou revisão publicada. |

Todas as rotas de domínio usam o prefixo ``/api/v1/b3``. Prefixos e métodos não listados abaixo não são contratos deste release. A ingestão cadastral ``BVBG.028``/TSL está ``deferred`` e os endpoints de cadastro não oferecem dados neste release; consulte ``/capabilities`` antes de depender deles. As famílias ``BVBG.086``, ``BVBG.186``, ``BVBG.087`` e composições de carteiras também não têm endpoints de consulta neste router; não construa URLs para elas a partir do PRD.

## 2. Autenticação e permissões

Rotas protegidas usam ``Authorization: Bearer <token>``. Há duas credenciais distintas, configuradas no servidor: leitura (``TUCANO_B3_READ_TOKEN``) e administração (``TUCANO_B3_ADMIN_TOKEN``). A credencial antiga ``TUCANO_B3_TOKEN`` não concede acesso ao contrato atual. Se as duas credenciais atuais não estiverem configuradas, forem vazias ou forem iguais, uma rota protegida retorna ``503 AUTH_NOT_CONFIGURED``.

| Nível | Rotas |
|---|---|
| Público, somente GET | ``/healthz``, ``/api/v1/health``, ``/openapi.json``, ``/docs``, ``/docs/`` |
| Leitura | Consultas COTAHIST, cadastro, capacidades e diagnóstico de cobertura descritos neste guia. |
| Administração | Todos os ``POST`` e as consultas operacionais de jobs, sync-runs, arquivos, members, pendências e revisões. |

Uma credencial válida de leitura em rota administrativa recebe ``403 ADMIN_REQUIRED``; a credencial administrativa também pode acessar rotas de leitura. Toda rota protegida sem credencial válida recebe ``401 UNAUTHORIZED``. O cliente web não deve embutir tokens do servidor em JavaScript, local storage ou código distribuído. Para uma aplicação de navegador, faça as chamadas autenticadas por um backend da própria aplicação; mantenha a credencial administrativa exclusivamente no servidor.

## 3. Convenções comuns

### Datas, decimais e valores ausentes

- Datas de mercado e de referência usam ``YYYY-MM-DD``, sem timezone. Intervalos COTAHIST são inclusivos nas duas pontas.
- Timestamps operacionais usam UTC com ``Z``, por exemplo ``2026-11-06T14:20:00Z``.
- Valores financeiros e quantidades decimais são strings JSON, por exemplo ``"30.20"``. Não os converta para ``Number``/``float`` em JavaScript para cálculos ou comparação exata. Use decimal arbitrário ou preserve e formate a string.
- Contagens inteiras, como ``trades_count``, são números JSON inteiros. Códigos B3 como ``market_type`` são strings: preserve zeros à esquerda (``"010"``).
- Campo opcional ausente é ``null`` quando o schema o expõe. ``null`` não significa zero, valor padrão ou evento inexistente.
- UUIDs chegam como strings. Compare IDs sem alterar a forma retornada.

### Paginação

Rotas de lista aceitam ``limit`` e ``offset``, exceto quando a tabela de filtros do endpoint diz o contrário.

| Parâmetro | Default | Regra |
|---|---:|---|
| ``limit`` | ``100`` | Inteiro de ``1`` a ``1000``. Fora do intervalo retorna ``400``; o servidor não aumenta nem reduz silenciosamente. |
| ``offset`` | ``0`` | Inteiro não negativo. Valor negativo retorna ``400``. |

Respostas de consulta de cotação e cadastro têm ``total``; use esse total para encerrar a paginação. Páginas operacionais têm ``items``, ``limit`` e ``offset``, sem ``total``; continue até receber menos itens que o ``limit`` efetivo. Não existe cursor de página neste release. Os endpoints operacionais podem mudar enquanto são paginados.

As ordenações são determinísticas para um estado de dados fixo. Paginação por ``offset`` não congela automaticamente o estado de listas operacionais ou de cadastro. Para cadastros, fixe uma fotografia (``snapshot_id``) quando precisar percorrer exatamente uma publicação. Para COTAHIST, fixe ``data_revision`` em todas as páginas.

### Erros

Erros da API têm o envelope:

```json
{
  "code": "INVALID_FILTER",
  "message": "Filtro, intervalo ou paginação inválidos.",
  "request_id": "00000000-0000-4000-8000-000000000001"
}
```

``request_id`` é novo por ocorrência e serve para correlação. Mensagens são seguras para exibição, mas o cliente deve decidir o comportamento pelo HTTP status e ``code``, não pelo texto da mensagem.

| HTTP | Interpretação para a interface |
|---:|---|
| ``200`` | Consulta síncrona concluída. ``items: []`` significa que a combinação válida não retornou registros. |
| ``202`` | Trabalho administrativo persistido para execução. Não significa que aquisição, parsing ou publicação terminou. |
| ``400`` | Path, filtros, paginação, JSON, enum ou intervalo inválido. Corrija a requisição antes de tentar novamente. |
| ``401`` | Bearer ausente ou inválido. |
| ``403`` | Bearer sem papel administrativo ou fonte/finalidade não elegível para aquela consulta. |
| ``404`` | Recurso identificado por path/ID não encontrado, revisão inexistente ou rota não publicada. |
| ``409`` | Conflito operacional, chave idempotente incompatível ou série COTAHIST ambígua. Não repita sem mudar a seleção. |
| ``500`` | Falha interna ou de persistência. Apresente ``request_id``; não exiba detalhes de banco. |
| ``503`` | Banco/readiness, revisão, fotografia ou dependência de consulta indisponível. Em geral, pode ser transitório; não interprete como lista vazia. |

``GET /healthz`` responde ``{ "healthy": true }``. ``GET /api/v1/health`` responde o mesmo formato quando o banco está acessível; banco indisponível retorna ``503 DATABASE_UNAVAILABLE``.

## 4. Mapa de rotas publicado

### Leitura de dados

| Método e caminho | Papel | Resposta de sucesso | Relação principal |
|---|---|---|---|
| ``GET /api/v1/b3/capabilities`` | Leitura | ``OperationPage<Capability>`` | Descobre famílias implementadas, habilitadas ou adiadas. |
| ``GET /api/v1/b3/diagnostics/coverage`` | Leitura | ``DatasetDiagnostics`` | Obtém uma revisão ativa ou fixa e seus limites gerais de cobertura. |
| ``GET /api/v1/b3/cotahist/quotes`` | Leitura | ``QuotePage`` | Retorna fatos COTAHIST; pode retornar várias linhas no mesmo pregão. |
| ``GET /api/v1/b3/cotahist/series`` | Leitura | ``QuotePage`` | Mesmos fatos/filtros; exige identidade de busca e uma linha inequívoca por data. |
| ``GET /api/v1/b3/instruments`` | Leitura | ``InstrumentPage`` | Rota implementada, mas fontes cadastrais adiadas; não há snapshot no fluxo suportado do ``0.0.1``. |
| ``GET /api/v1/b3/instruments/{id}`` | Leitura | ``InstrumentPage`` | Rota cadastral sem evidência publicada neste release. |
| ``GET /api/v1/b3/instruments/{id}/identifiers`` | Leitura | ``InstrumentIdentifierPage`` | Rota cadastral sem evidência publicada neste release. |
| ``GET /api/v1/b3/tradable-securities`` | Leitura | ``InstrumentPage`` | Rota TSL sem dados disponíveis; nunca usa BVBG.028 como fallback. |

### Operação e inspeção administrativa

| Método e caminho | Papel | Resposta de sucesso | Relação principal |
|---|---|---|---|
| ``GET /api/v1/b3/jobs`` | Admin | ``OperationPage<JobView>`` | Lista os mesmos trabalhos que ``/sync-runs``. |
| ``POST /api/v1/b3/jobs`` | Admin | ``202 AcceptedJob`` | Enfileira comando fechado e idempotente. |
| ``GET /api/v1/b3/jobs/{id}`` | Admin | ``JobView`` | Acompanhamento canônico do trabalho. |
| ``POST /api/v1/b3/jobs/{id}/cancel`` | Admin | ``202 AcceptedJob`` | Solicita cancelamento durável do trabalho. |
| ``POST /api/v1/b3/jobs/{id}/retry`` | Admin | ``202 AcceptedJob`` | Cria nova execução ligada a job falho/cancelado. |
| ``GET /api/v1/b3/sync-runs`` | Admin | ``OperationPage<JobView>`` | Projeção dos jobs atuais; não é a tabela de runs legados. |
| ``GET /api/v1/b3/sync-runs/{id}`` | Admin | ``JobView`` | Alias de acompanhamento com o mesmo ID de job. |
| ``GET /api/v1/b3/source-files`` | Admin | ``OperationPage<SourceFileView>`` | Lista artefatos brutos persistidos. |
| ``GET /api/v1/b3/source-files/{id}/members`` | Admin | ``OperationPage<MemberView>`` | Abre o inventário de members de um artefato. |
| ``GET /api/v1/b3/pending-updates`` | Admin | ``OperationPage<PendingView>`` | Lista referências remotas encontradas pelo scanner. |
| ``POST /api/v1/b3/pending-updates/{id}/acknowledge`` | Admin | ``PendingAcknowledged`` | Vincula pendência a arquivo já publicado, com justificativa. |
| ``POST /api/v1/b3/discovery/scan`` | Admin | ``DiscoveryResult`` | Executa checagem leve; não baixa nem ingere. |
| ``GET /api/v1/b3/data-revisions/{id}`` | Admin | ``RevisionView`` | Inspeciona a revisão COTAHIST no release; o schema reserva family ``registry`` para evolução. |
| ``GET /api/v1/b3/data-revisions/{id}/conflicts`` | Admin | ``OperationPage<ConflictView>`` | Lista divergências COTAHIST sem eleger vencedora. |
| ``GET /api/v1/b3/data-revisions/{id}/quarantine`` | Admin | ``OperationPage<RejectionView>`` | Lista rejeições localizadas da revisão/snapshot. |

## 5. Fluxos recomendados para interfaces

### 5.1 Inicializar uma tela de histórico

1. Faça ``GET /healthz`` se precisar verificar apenas se o processo responde. Faça ``GET /api/v1/health`` quando a tela também depender de banco. Esses checks não provam que exista uma revisão publicada.
2. Faça ``GET /api/v1/b3/capabilities?limit=100&offset=0``. Para COTAHIST, verifique ``implemented``, ``enabled`` e ``status``. ``enabled`` não significa que haja linhas para o filtro pretendido.
3. Faça ``GET /api/v1/b3/diagnostics/coverage``. Guarde ``data_revision`` e mostre os dados de ``coverage`` como limites conhecidos, não como prova de continuidade.
4. Faça ``GET /api/v1/b3/cotahist/quotes`` com filtros explícitos de ticker ou instrumento, intervalo de datas, mercado e demais discriminadores relevantes. Envie o ``data_revision`` obtido no passo anterior.
5. Para a próxima página, mantenha todos os filtros e o mesmo ``data_revision``; altere somente ``offset``. Encerre quando ``offset + items.length >= total``.
6. Se a tela precisa de uma única observação por pregão, use ``/cotahist/series``. Em ``409 AMBIGUOUS_SERIES``, não escolha a primeira linha: acrescente ``market_type``, ``distribution_number``, ``term_days`` e/ou ``bdi_code``, ou mostre os fatos alternativos obtidos por ``/cotahist/quotes``.

### 5.2 Pesquisar instrumento e ver seus identificadores

1. Para ticker/ISIN conhecido, prefira filtros exatos em ``GET /instruments``; para busca textual ampla, use ``query``.
2. Selecione uma família usando ``source_file_type`` quando a interface precisa de semântica única. Exemplo: ``TRADABLE_SECURITY_LIST`` ou ``BVBG_028_INSTRUMENTS_FILE``.
3. Se a resposta trouxer mais de um ``snapshot_id``, a consulta cobriu mais de uma fotografia/família ativa. Para paginação de uma publicação, repita com um único ``snapshot_id`` e mantenha-o em todas as páginas. O contrato não aceita uma lista de snapshots em um único pedido.
4. Se um item tiver ``instrument_id``, abra ``GET /instruments/{id}`` para outras evidências e ``GET /instruments/{id}/identifiers`` para ticker, ISIN e códigos qualificados. Se ``instrument_id`` for ``null``, não há ID canônico para usar nesses paths.
5. ``reference_date`` informa a data da fotografia. Só ``valid_from``/``valid_to`` de ``/identifiers`` informam uma janela de validade explicitamente registrada; não prolongue a fotografia até outra data por inferência.

### 5.3 Processar uma atualização no console operacional

1. Um administrador executa ``POST /discovery/scan``. A resposta ``200`` informa ``checked_at`` e ``scheduled: false``; a checagem pode criar pendências, mas não cria arquivos nem jobs de ingestão.
2. Consulte ``GET /pending-updates``. Uma pendência representa uma possível publicação nova, não alteração confirmada de cotação.
3. Para adquirir/processar, submeta ``POST /jobs`` com operação ``ingest``, família e intervalo de datas. A API seleciona canais configurados no servidor; o consumidor não envia URL de download. Confirme ``enabled`` em capabilities e não envie família ``deferred``.
4. Guarde ``job_id`` e ``status_url`` do corpo. ``POST /jobs`` também retorna ``Location``. Consulte ``GET /jobs/{id}`` com credencial administrativa até status terminal. Não use a tela de sucesso do ``202`` como confirmação de ingestão.
5. Examine ``outputs``. ``source_file`` aponta para um artefato visível em ``/source-files``; abra seus members em ``/source-files/{id}/members``. ``revision`` aponta para ``/data-revisions/{id}``; inspecione rejeições e conflitos antes de publicar.
6. Publique uma revisão ``ready`` usando outro ``POST /jobs`` com ``publish_revision``. Se houver rejeições, ``acknowledge_issues`` precisa ser explicitamente ``true`` para permitir a publicação com problemas; conflitos bloqueiam a publicação. A revisão ativa anterior continua em uso até a promoção ser confirmada.
7. Depois da publicação, use o novo ``data_revision`` em ``/diagnostics/coverage`` e ``/cotahist/quotes``. A revisão retornada na primeira consulta pode mudar entre telas se o consumidor omitir o ID.

### 5.4 Cancelar, repetir ou reconhecer trabalho

- Cancelamento: ``POST /jobs/{id}/cancel``, sem corpo. Recebe ``202`` com ``job_id`` e ``status_url``; esta rota não envia header ``Location``. O worker observa o pedido nos pontos seguros. O estado pode continuar ``running`` com ``cancel_requested_at`` preenchido até que o cancelamento seja aplicado; trabalho que já confirmou atomicamente pode concluir.
- Retry: apenas status ``failed`` ou ``cancelled`` aceita ``POST /jobs/{id}/retry``. Use uma nova ``idempotency_key``; o retry mantém prioridade, comando e limite de tentativas e aparece como novo ``job_id``, com ``retry_of`` apontando ao original.
- Reconhecimento de pendência: use ``POST /pending-updates/{id}/acknowledge`` somente quando um ``source_file`` já publicado do mesmo canal sustenta o reconhecimento. Isso registra motivo e referência; não baixa arquivo, não gera contadores fictícios e não altera cotações.

## 6. Contratos de consulta

### 6.1 COTAHIST: filtros e paginação fixada

Rotas: ``GET /api/v1/b3/cotahist/quotes`` e ``GET /api/v1/b3/cotahist/series``.

| Query | Tipo | Semântica |
|---|---|---|
| ``ticker_symbol`` | string | Código publicado; normalizado para maiúsculas. Não implica instrumento único. |
| ``instrument_id`` | UUID em string | UUID canônico, quando há vínculo temporal comprovado. |
| ``isin`` | string | ISIN da ocorrência, normalizado para maiúsculas. |
| ``trade_date_from``, ``trade_date_to`` | data ISO | Limites inclusivos; ``from`` posterior a ``to`` retorna ``400``. |
| ``market_type`` | string | Código de mercado, incluindo zeros à esquerda. |
| ``bdi_code`` | string | Código BDI oficial. |
| ``distribution_number`` | string | ``DISMES``; discriminador de distribuição. |
| ``term_days`` | string | Prazo do mercado a termo. |
| ``data_revision`` | UUID | Revisão publicada a fixar. Se omitida, cada request resolve a revisão ativa no momento da chamada. |
| ``limit``, ``offset`` | inteiros | Regras comuns de paginação. |
| ``include_internal`` | boolean | Default ``false``; quando ``true``, acrescenta ``internal.version_id`` e ``internal.business_hash``. Não altera autorização. |

Os filtros são combinados por AND. Omita filtros não usados; não envie ``raw_sql`` ou filtros desconhecidos. ``/quotes`` aceita uma busca sem ticker. ``/series`` também aceita os mesmos parâmetros, mas exige pelo menos um de ``ticker_symbol``, ``instrument_id`` ou ``isin``, e retorna ``409 AMBIGUOUS_SERIES`` se restar mais de um fato em qualquer pregão.

O resultado é ``QuotePage``:

| Campo | Conteúdo |
|---|---|
| ``items`` | Cotações na ordem ``trade_date``, ``id``; cada ``id`` identifica o fato estável, não uma versão específica. |
| ``total``, ``limit``, ``offset`` | Contagem filtrada e paginação efetiva. |
| ``data_revision`` | UUID da revisão usada; copie para todas as páginas seguintes. |
| ``disclaimer`` | Aviso obrigatório de preços nominais sem ajustes. |
| ``coverage`` | Limites de cobertura conhecidos para a resposta/revisão. |

Cada item ``HistoricalQuote`` contém: ``id``, ``trade_date``, ``ticker_symbol``, ``market_type``, ``bdi_code``, ``distribution_number``, ``term_days``, ``issuer_short_name``, ``paper_specification``, ``currency``, ``open_price``, ``high_price``, ``low_price``, ``average_price``, ``last_price``, ``best_bid_price``, ``best_ask_price``, ``trades_count``, ``traded_quantity``, ``traded_volume``, ``option_exercise_price``, ``price_correction_indicator``, ``maturity_date``, ``quotation_factor``, ``exercise_price_points``, ``isin``, ``identity``, ``provenance`` e, opcionalmente, ``internal``.

``identity`` tem ``instrument_id`` nullable e ``status`` (``resolved``, ``unresolved`` ou ``ambiguous``). Um estado não resolvido não remove o fato. ``provenance`` contém ``source_file_id``, ``source_file_type``, SHA-256, ``source_member_id``, ``source_member_name`` e ``record_index`` iniciado em 1. O vínculo navegável é: item → ``provenance.source_file_id`` → ``/source-files/{id}/members`` → ``provenance.source_member_id``; ``record_index`` localiza a linha/registro dentro do member.

``coverage`` contém ``requested_from``, ``requested_to``, ``status``, ``first_observed``, ``last_observed``, ``rejected``, ``unresolved`` e ``calendar_status``. No runtime atual, ``status`` é ``partial`` e ``calendar_status`` é ``unknown``; min/max não demonstram pregões contínuos. Em uma página filtrada, datas observadas e não resolvidos refletem os itens filtrados; ``rejected`` é o total de rejeições da revisão, não uma contagem restrita ao intervalo pedido.

Exemplo de consulta fixada:

```http
GET /api/v1/b3/cotahist/quotes?ticker_symbol=EXMP3&market_type=010&trade_date_from=2025-01-02&trade_date_to=2025-01-31&limit=100&offset=0&data_revision=00000000-0000-4000-8000-000000000001
Authorization: Bearer <read-token>
```

Exemplo de resposta abreviado:

```json
{
  "items": [{
    "id": "00000000-0000-4000-8000-000000000010",
    "trade_date": "2025-01-02",
    "ticker_symbol": "EXMP3",
    "market_type": "010",
    "currency": "BRL",
    "open_price": "30.10",
    "last_price": "30.20",
    "traded_quantity": "1000",
    "traded_volume": "30200.00",
    "identity": {"instrument_id": null, "status": "unresolved"},
    "provenance": {
      "source_file_id": "00000000-0000-4000-8000-000000000011",
      "source_file_type": "COTAHIST_YEARLY",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "source_member_id": "00000000-0000-4000-8000-000000000012",
      "source_member_name": "COTAHIST.2025.TXT",
      "record_index": 42
    },
    "internal": null
  }],
  "total": 1,
  "limit": 100,
  "offset": 0,
  "data_revision": "00000000-0000-4000-8000-000000000001",
  "disclaimer": "Preços históricos nominais brutos, na moeda e forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos.",
  "coverage": {
    "requested_from": "2025-01-02",
    "requested_to": "2025-01-31",
    "status": "partial",
    "first_observed": "2025-01-02",
    "last_observed": "2025-01-02",
    "rejected": 0,
    "unresolved": 1,
    "calendar_status": "unknown"
  }
}
```

O exemplo é sintético e abreviado; uma resposta real pode incluir ``null`` em mais campos. ``last_price`` corresponde a ``PREULT``, o último negócio publicado. Não é preço ajustado, retorno total ou garantia de fechamento oficial.

### 6.2 Cadastro e fotografias

Rotas: ``GET /instruments``, ``GET /instruments/{id}``, ``GET /instruments/{id}/identifiers`` e ``GET /tradable-securities``.

Essas rotas permanecem documentadas no contrato técnico, mas as famílias cadastrais estão adiadas e não podem ser importadas no ``0.0.1``; COTAHIST pode retornar cotações com identidade ``unresolved``.

``/instruments`` e ``/tradable-securities`` aceitam:

| Query | Tipo | Semântica |
|---|---|---|
| ``instrument_id`` | UUID em string | Filtra pelo instrumento canônico se resolvido. |
| ``query`` | string | Busca parcial sem diferenciar maiúsculas/minúsculas em ticker, security ID, ISIN ou descrição. |
| ``ticker_symbol`` | string | Ticker exato publicado, normalizado para maiúsculas. |
| ``security_id`` | string | Código exato no namespace/qualificador registrado. |
| ``isin`` | string | ISIN exato, normalizado para maiúsculas. |
| ``snapshot_id`` | UUID | Fotografia publicada singular para uma consulta reprodutível. |
| ``source_file_type`` | enum string | ``BVBG_028_INSTRUMENTS_FILE`` ou ``TRADABLE_SECURITY_LIST``. |
| ``limit``, ``offset`` | inteiros | Regras comuns de paginação. |

``/tradable-securities`` fixa a família em ``TRADABLE_SECURITY_LIST``. Se o cliente também enviar outro ``source_file_type``, recebe ``400``; se não houver snapshot elegível, não cai para BVBG.028. Para ``/instruments``, sem ``snapshot_id``, são selecionadas as fotografias ativas publicadas por família. O servidor ordena por fotografia, member e índice de registro. ``InstrumentPage`` retorna ``items``, ``total``, ``limit``, ``offset`` e ``snapshots`` (IDs das fotografias selecionadas).

Cada ``InstrumentEvidence`` contém ``instrument_id`` nullable, ``snapshot_id``, ``reference_date``, ``security_id``, ``security_source``, ``market_identifier_code``, ``ticker_symbol``, ``isin``, ``asset_description``, ``market_code``, ``instrument_type``, ``security_category``, ``asset``, ``segment_code``, ``cfi_code``, ``instrument_status``, ``maturity_date``, ``tradable_indicator``, ``trading_start_date``, ``trading_end_date`` e ``provenance``. ``reference_date`` é data da fotografia, não validade presumida. Os campos permanecem específicos à evidência disponível; ausência em uma família não autoriza copiar valor de outra.

``GET /instruments/{id}`` usa o UUID do path e os filtros de cadastro opcionais. Se ``instrument_id`` na query não for o mesmo UUID, retorna ``400``. Uma implementação futura retornará evidências ativas publicadas para o instrumento; não há ingestão habilitada que crie essas evidências neste release.

``GET /instruments/{id}/identifiers`` mantém o schema preparado para evidências futuras, mas não há ingestão cadastral habilitada nem itens suportados pelo ``0.0.1``. Não use esta rota como catálogo nem derive uma janela de validade para fatos históricos.

### 6.3 Capacidades e diagnóstico

``GET /capabilities`` aceita ``limit`` e ``offset``; a resposta é ``OperationPage<Capability>`` sem ``total``. Cada item tem ``source_file_type``, ``implemented``, ``enabled`` e ``status`` (``enabled``, ``not_enabled`` ou ``deferred``). O catálogo conhece ``BVBG_028_INSTRUMENTS_FILE``, ``TRADABLE_SECURITY_LIST``, as três granularidades COTAHIST, ``BVBG_086_PRICE_REPORT``, ``BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT``, ``BVBG_087_INDEX_REPORT`` e ``B3_INDEX_PORTFOLIO_COMPOSITION``. Em ``0.0.1``, somente as três granularidades COTAHIST implementam ingestão; BVBG.028/TSL estão adiadas até suportarem cortes por data interna de cada registro, e as outras famílias permanecem adiadas por escopo. ``enabled`` exige canal ligado e elegibilidade para REST e distribuição a terceiros sem custo; não comprova que haja registro para uma data ou ticker.

``GET /diagnostics/coverage`` aceita apenas ``data_revision`` opcional (UUID). Sem o parâmetro, escolhe a revisão COTAHIST ativa no momento; com ele, consulta essa revisão publicada. A resposta contém ``data_revision`` e ``coverage`` para a revisão inteira. Para uma janela filtrada, use ``coverage`` da resposta de ``/quotes``.

## 7. Fluxos e schemas administrativos

### 7.1 Submeter comandos

``POST /api/v1/b3/jobs`` recebe um objeto fechado. Campos extras são rejeitados.

```json
{
  "idempotency_key": "cotahist-2025-refresh-v1",
  "command": {
    "operation": "ingest",
    "source": "COTAHIST_YEARLY",
    "from": "2025-01-01",
    "to": "2025-12-31"
  },
  "priority": "backfill",
  "max_attempts": 3
}
```

Regras do envelope:

- ``idempotency_key``: obrigatório, não branco, até 255 bytes. Reutilizar a chave com o mesmo comando, prioridade, tentativas e linhagem retorna o mesmo trabalho; reuso incompatível ou colisão de escopo retorna ``409``.
- ``command``: enum fechado abaixo. Datas de intervalo devem satisfazer ``from <= to``.
- ``priority``: obrigatório; ``maintenance``, ``backfill`` ou ``rebuild``. A lane de manutenção preserva capacidade reservada.
- ``max_attempts``: opcional, default ``3``, intervalo ``1..=100``.
- Resposta real de submissão: ``202``, corpo ``{ "job_id": "...", "status_url": "/api/v1/b3/jobs/..." }`` e header ``Location`` com o mesmo path. O handler não executa o comando durante o request.

Comandos aceitos no schema:

| ``command.operation`` | Campos de ``command`` | Efeito solicitado |
|---|---|---|
| ``ingest`` | ``source`` (COTAHIST no ``0.0.1``), ``from``, ``to`` | Adquire/processa canais COTAHIST habilitados e corta por ``DATA DO PREGÃO``; família sem data interna suportada retorna ``400``. A URL é definida pela configuração do canal no servidor. |
| ``reparse`` | ``source_file_id`` (UUID) | Reprocessa artefato bruto já preservado com parser atual. |
| ``replay_quarantine`` | ``source_file_id`` (UUID) | Reinterpreta evidências de arquivo preservado para recuperar rejeições, mantendo histórico. |
| ``rebuild_identity`` | ``from``, ``to`` | Reconstrói relações temporais de identidade no período. |
| ``publish_revision`` | ``revision_id`` (UUID), ``acknowledge_issues`` (boolean) | Promove revisão ``ready`` depois das validações; sinaliza explicitamente se aceita rejeições. |
| ``rollback_revision`` | ``revision_id`` (UUID) | Reaponta a publicação ativa para uma revisão publicada anterior. |

Valores do enum ``source``: ``BVBG_028_INSTRUMENTS_FILE``, ``TRADABLE_SECURITY_LIST``, ``BVBG_086_PRICE_REPORT``, ``BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT``, ``BVBG_087_INDEX_REPORT``, ``B3_INDEX_PORTFOLIO_COMPOSITION``, ``COTAHIST_YEARLY``, ``COTAHIST_MONTHLY``, ``COTAHIST_DAILY``. A enum valida o nome, não a capacidade: antes de enviar ``ingest``, confira ``GET /capabilities``; famílias ``deferred`` não estão disponíveis neste release.

### 7.2 Job e estados

``GET /jobs/{id}`` é o endpoint canônico para acompanhar; requer admin. ``GET /sync-runs/{id}`` retorna o mesmo ``JobView``. ``GET /jobs`` e ``/sync-runs`` listam os mesmos trabalhos; ``/sync-runs`` não deve ser interpretado como endpoint separado de execução legada.

``JobView`` contém:

| Campo | Uso na interface |
|---|---|
| ``id``, ``retry_of`` | ID do job e job terminal que originou um retry, quando houver. |
| ``command``, ``priority`` | Pedido tipado e classe de capacidade. |
| ``status`` | ``queued``, ``running``, ``succeeded``, ``succeeded_with_issues``, ``failed`` ou ``cancelled``. |
| ``phase`` | ``queued``, ``acquisition``, ``inventory``, ``parsing``, ``staging``, ``sealing``, ``ready``, ``publication``, ``identity`` ou ``completed``. |
| ``attempt``, ``max_attempts`` | Tentativas iniciadas e limite configurado. |
| ``generation`` | Geração de claim/fencing; informativa, não é token de concorrência do consumidor. |
| ``heartbeat_at``, ``cancel_requested_at``, ``next_attempt_at`` | Sinais operacionais UTC; podem ser nulos quando não aplicáveis. |
| ``error_code``, ``error_message`` | Última falha segura; podem ser nulos. Não incluem SQL ou credenciais. |
| ``created_at``, ``finished_at`` | Criação e encerramento UTC. ``finished_at`` é nulo enquanto não terminal. |
| ``outputs`` | Recursos realmente persistidos, cada um com ``kind`` e ``id``. |

``status`` e ``phase`` são dimensões diferentes. Enquanto status for ``running``, use ``phase`` para apresentar progresso; uma tentativa em backoff pode permanecer ``queued`` com ``next_attempt_at`` futuro. ``succeeded_with_issues`` é terminal e não é elegível ao endpoint explícito de retry, que aceita apenas ``failed`` e ``cancelled``.

Tipos de output: ``source_file`` → ``/source-files/{id}`` aparece no inventário de arquivos e sua linhagem abre em ``/source-files/{id}/members``; ``revision`` → ``/data-revisions/{id}``; ``identity_rebuild`` é um recibo de operação e não tem endpoint de detalhe próprio neste router.

### 7.3 Inspeção de arquivos, members e revisões

As rotas de lista abaixo aceitam somente ``limit`` e ``offset`` e respondem ``OperationPage``, sem ``total``.

| Rota | Campos de cada item | Ordenação |
|---|---|---|
| ``GET /source-files`` | ``id``, ``source_file_type``, ``acquisition_channel``, ``sha256``, ``byte_size``, ``parser_version``, ``status``, contadores (``member_count``, ``row_count_raw``, ``row_count_inserted``, ``row_count_changed``, ``row_count_unchanged``, ``row_count_duplicate``, ``row_count_quarantined``) e ``provenance``. | Criação e ID. |
| ``GET /source-files/{id}/members`` | ``id``, ``member_name``, ``member_index``, ``parent_index``, ``sha256``, ``byte_size``, ``selected``, ``exclusion_reason``, ``status``, contadores de registros e ``provenance``. | ``member_index``, ID. |
| ``GET /pending-updates`` | ``id``, ``channel_id``, ``source_file_type``, ``reference_date``, ``status``, ``detected_at``. | Detecção e ID. |
| ``GET /jobs`` ou ``/sync-runs`` | ``JobView`` integral. | Criação e ID. |

``SourceFileView.provenance.source_file_id`` repete o ``id`` do arquivo. Arquivos não expõem URL tokenizada nem caminho privado. Em ``MemberView``, ``parent_index`` aponta ao índice do container pai; ``member_name`` é o caminho lógico e nomes iguais em containers diferentes permanecem members separados. ``selected=false`` e ``exclusion_reason`` explicam por que uma entrada inventariada não foi interpretada.

``GET /data-revisions/{id}`` responde ``RevisionView`` com ``family`` (``cotahist`` ou ``registry``), ``id``, ``base_revision_id``, ``period_start``, ``period_end``, ``status`` (``draft``, ``ready``, ``published``, ``invalid``), ``expected_members``, ``rejected``, ``conflicts``, ``created_at``, ``published_at`` e ``provenance``. Em fotografia cadastral, início e fim do período são a mesma data de referência. Essa rota é somente leitura; publicar/rollback exige job.

``GET /data-revisions/{id}/conflicts`` responde ``ConflictView``: ``fact_id``, ``existing_version_id``, ``incoming_version_id``, ``source_member_id``, ``record_index``, ``provenance``. Divergência é mantida para decisão; a ordenação não é precedência.

``GET /data-revisions/{id}/quarantine`` responde ``RejectionView``: ``source_member_id``, ``record_index``, ``byte_offset``, ``reason_code``, ``message``, ``provenance``. O payload bruto não é exposto nessa listagem. Reparse/replay é comando administrativo separado; a leitura não resolve a rejeição.

``OperationProvenance`` tem ``source_file_id`` nullable e ``data_revision`` nullable. A presença de um valor depende de qual recurso originou o item.

### 7.4 Descoberta e pendências

``POST /api/v1/b3/discovery/scan`` não recebe body. Quando concluído, retorna ``200 { "checked_at": "...Z", "scheduled": false }``. Checa canais habilitados conforme capacidades/elegibilidade, registra resultado e pendências, mas não baixa corpos de datasets, não faz parsing, não publica e não inicia job. Depois, liste ``/pending-updates``.

``POST /api/v1/b3/pending-updates/{id}/acknowledge`` recebe:

```json
{
  "source_file_id": "00000000-0000-4000-8000-000000000020",
  "reason": "O arquivo publicado já cobre a referência detectada."
}
```

``reason`` é obrigatório, não pode ser branco e tem limite de 1024 bytes. O arquivo deve estar publicado, ingestado e pertencer ao mesmo canal. Resposta ``200``: ``{ "id": "<pending-id>", "source_file_id": "<file-id>" }``. Se a evidência não for compatível, a API retorna ``409``; pendência ou arquivo ausente retorna ``404``.

## 8. Eficiência, consistência e comportamento de interface

1. **Pagine no servidor.** Use filtros exatos e limite que caiba na tela; a API admite até 1000 itens. Não carregue o histórico completo para filtrar no navegador.
2. **Fixe a revisão de cotação.** Obtenha-a uma vez por pesquisa, guarde-a com o estado de tela e envie-a em todas as páginas e consultas correlatas. Se a revisão não estiver publicada ou não estiver acessível, mostre erro e reinicie o fluxo de revisão; não concatene páginas de revisões diferentes.
3. **Desambigue antes de plotar.** Uma série exige no máximo um fato por pregão. Preserve mercado, distribuição, prazo e BDI no estado do gráfico e nos links compartilháveis. Se o usuário escolher “todos os fatos”, use ``/quotes``, não esconda duplicatas para ajustar um gráfico.
4. **Mantenha precisão financeira.** Ordene e calcule com decimal arbitrário. A string ``"100.00"`` não deve virar float binário para depois ser apresentada como valor exato.
5. **Mostre a fonte e o estado de identidade.** Use ``provenance`` para abrir uma visão de arquivo/member/registro quando a tela suportar auditoria. Exiba ``unresolved``/``ambiguous``; não oculte itens sem ``instrument_id``.
6. **Trate ``200`` vazio e ``503`` de forma diferente.** Vazio é consulta válida sem fatos na revisão/filtro. ``503`` significa que a revisão, fotografia ou dependência não está servível.
7. **Não extrapole cobertura.** ``first_observed`` e ``last_observed`` são extremos conhecidos, não calendário completo. A API atualmente informa calendário ``unknown`` e cobertura ``partial``.
8. **Faça backoff em ``503`` e falhas ``500``.** O processamento analítico é limitado no servidor e pode falhar por indisponibilidade; preservar o mesmo ``data_revision`` ao repetir uma leitura. Não faça retry automático em ``400``, ``401``, ``403`` ou ``409`` sem corrigir a causa.
9. **Evite polling excessivo de jobs.** Use ``status_url`` e intervalos crescentes, com limite no cliente; pare quando o status chegar a ``succeeded``, ``succeeded_with_issues``, ``failed`` ou ``cancelled``. O campo de heartbeat é sinal de vida do executor, não promessa de percentual.
10. **Não confunda capacidades e disponibilidade de dados.** ``implemented=true`` fala sobre suporte no release; ``enabled=true`` fala sobre canal/elegibilidade configurados; apenas uma consulta bem-sucedida demonstra registros para seus filtros.

## 9. Rotas e comportamentos que não são contratos

- Rotas antigas descritas em módulos não montados, como ``/api/v1/b3/price-report/...``, ``/index-report/...``, ``/index-composition/...``, ``/sync``, ``/quarantine`` ou ``/company-data``, não estão registradas no router do release atual. Use apenas o inventário da seção 4.
- ``/docs`` hoje não oferece “Try it out”; use ``/openapi.json`` como spec.
- Não há paginação cursor-based, endpoint de autenticação OAuth, filtro arbitrário SQL, transporte HTTP para MCP, atualização em tempo real ou endpoint de upload de arquivo neste contrato.
- MCP é transporte analítico somente leitura separado do REST; não o trate como rota HTTP.
- O PRD pode descrever endpoints futuros por família. A existência de schemas/migrations ou de fonte B3 não torna esses endpoints consultáveis.
- Dados B3 acessíveis no ambiente continuam sujeitos a elegibilidade por fonte, canal e finalidade. A presença da rota não atesta autorização externa para redistribuição.

## 10. Referências do projeto

- [Contrato resumido de consumo](consumer_api.md)
- OpenAPI gerado: disponível em ``/openapi.json`` durante a execução do serviço.
- [README e configuração local](../README.md)
- [PRD de produto](b3_prd.md), especialmente escopo de fontes, contratos REST e revisão/publicação
- [Elegibilidade das fontes para o release](release_source_eligibility.md)
- [Pesquisa de fontes oficiais](RESEARCH.md)
- [Guia de operação](release_operations.md)
- [Changelog de contratos](frontend_api_changelog.md)
- [Guia de contexto B3](B3.md) e [referência de campos](ref_b3.md)
