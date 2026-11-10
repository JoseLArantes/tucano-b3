# Consumo do histórico nominal 0.0.1

Status: contrato 0.0.1 qualificado com fixtures sintéticas. Consultar o OpenAPI gerado em `/openapi.json` para o conjunto exato de operações registrado pelo binário. A plataforma não depende de dados reais pré-carregados; acesso a cada fonte continua dependente de elegibilidade própria.

Para explorar o contrato HTTP do binário em execução, abra `/docs`: a interface Swagger carrega o mesmo documento gerado e publicado em `/openapi.json`. Use o JSON para gerar clientes ou automatizar validações. As chamadas feitas por “Try it out” continuam sujeitas à autenticação e às permissões de cada operação. Para a referência narrativa detalhada de endpoints, parâmetros, schemas e fluxos operacionais, consulte o [site Docusaurus](../docusaurus/docs/intro.md), que é separado da interface Swagger.

Para um inventário detalhado do contrato HTTP implementado, fluxos de frontend, filtros, paginação, permissões, erros e relações entre recursos, consulte o [guia detalhado de consumo](consumers_guides.md).

## Autenticação e filtros

Enviar `Authorization: Bearer <credencial-de-leitura>` nas consultas protegidas. Operações administrativas, incluindo `POST /api/v1/b3/discovery/scan`, exigem `TUCANO_B3_ADMIN_TOKEN`; o valor configurado como `TUCANO_B3_TOKEN` não substitui essa credencial. A janela “Authorize” do Swagger UI não lê o `.env`: cole nela o token correspondente ao papel exigido. Credencial administrativa é separada e não deve ser distribuída a consumidores. `include_internal=true` não amplia permissões nem revela tokens/caminhos privados.

O console de análise local usa `GET /api/v1/b3/internal/diagnostics/coverage`, `/internal/cotahist/quotes`, `/internal/cotahist/series` e `/internal/cotahist/quotes/export`, todos sob o prefixo `/api/v1/b3`. Exigem `TUCANO_B3_ADMIN_TOKEN` e que cada fonte da revisão esteja elegível para `processing` e `raw_retention`. Essas rotas destinam-se apenas ao uso administrativo interno; não substituem as rotas para consumidores nem habilitam REST, MCP ou entrega a terceiros. Para consumo autorizado, use as rotas `/api/v1/b3/cotahist/...` com o token de leitura.

`GET /api/v1/b3/cotahist/quotes?ticker_symbol=EXMP3&market_type=010&trade_date_from=2025-01-01&trade_date_to=2025-12-31&limit=100&offset=0`

Ticker identifica um código no contexto e período. O mesmo ticker pode ter mercados, distribuições, prazos e instrumentos diferentes. Datas são inclusivas; paginação inválida retorna 400. `limit` padrão 100, máximo 1000; `offset` padrão 0. Filtros adicionais incluem instrumento, ISIN, BDI, distribuição e prazo.

## Descoberta de arquivos e fontes COTAHIST

`POST /api/v1/b3/discovery/scan` exige `TUCANO_B3_ADMIN_TOKEN` no Bearer e não recebe corpo. A descoberta consulta canais registrados e também percorre os locais de `B3_DISCOVERY_LOCATIONS`, uma lista separada por ponto e vírgula ou vírgula. Prefira ponto e vírgula se os prefixos puderem conter vírgula. Ela lê apenas metadados: não baixa, abre, extrai, faz parsing, cria arquivo bruto nem inicia um job.

```dotenv
B3_DISCOVERY_LOCATIONS=folder:/data/raw/imports;b3_url:https://www.b3.com.br/arquivos/COTAHIST_A2025.ZIP;object_storage_bucket:s3://dados/cotahist/
AWS_REGION=us-east-1
AWS_ENDPOINT_URL_S3=https://s3.compatible.example
```

`folder` requer caminho absoluto; no Compose, `./data/imports` é montado somente leitura em `/data/raw/imports`. O scanner não segue links simbólicos e examina no máximo 10.000 entradas e 8 níveis por pasta. `b3_url` requer URL HTTPS direta para nome de arquivo COTAHIST `.TXT`/`.ZIP` em um host exato de `B3_ALLOWED_HOSTS`; usa `HEAD` com verificação de DNS e redirecionamentos e não faz fallback para GET. `host:` é alias aceito. `object_storage_bucket` aceita `s3://bucket/prefixo/`, consulta `ListObjectsV2` na cadeia padrão de credenciais AWS e exige `s3:ListBucket` somente; nenhum objeto é lido. Para endpoint compatível, informe `AWS_ENDPOINT_URL_S3`; configure região e credenciais somente leitura conforme o provedor. Até 32 locais podem ser informados e cada bucket limita a listagem a 10.000 objetos. Tipos desconhecidos ou endereços inválidos falham a validação, em vez de serem silenciosamente ignorados.

A resposta `DiscoveryResult` retorna `checked_at`, `scheduled`, `locations_scanned`, `candidate_count`, `candidates_truncated`, `locations` (tipo, rótulo sem raiz privada, resultado e truncamento) e até 1.000 `candidates`. Cada candidato tem `source_file_type`, `source_locator`, `byte_size`, `remote_etag` e `remote_last_modified`. O tipo vem do nome `COTAHIST_A{AAAA}`, `COTAHIST_M{MMAAAA}` ou `COTAHIST_D{DDMMAAAA}`; isso não valida o conteúdo, não define data de referência nem escolhe uma revisão. ETag não é SHA-256. As observações são gravadas em `b3_discovery_location_checks` e `b3_discovery_location_candidates`. Um achado ainda precisa ser registrado como canal e passar pelas regras de elegibilidade antes de aquisição ou ingestão.

Para importar um candidato da pasta local pelo console de gestão, envie o locador selecionado e uma attestation explícita:

```http
POST /api/v1/b3/discovery/candidates/register-local
Authorization: Bearer <TUCANO_B3_ADMIN_TOKEN>
Content-Type: application/json

{"source_locator":"folder:1/2026/COTAHIST_A2026.ZIP","authorize_internal_development":true}
```

A operação só aceita candidatos da última varredura manual, com até 24 horas, dentro de `B3_RAW_DATA_DIR/imports`, sem links simbólicos e sem mudança de tamanho. O nome determina a família e o período; o servidor registra um canal local e elegibilidade para aquisição manual, retenção bruta e processamento de desenvolvimento interno. REST, MCP e entrega a terceiros permanecem sem elegibilidade. Arquivos descobertos em URLs B3 ou S3 não podem ser registrados por esta rota.

O retorno `RegisteredLocalCandidate` inclui `channel_id`, família, locador sanitizado, período inclusivo e `internal_development_only=true`. Envie esse `channel_id` no objeto `command` do `POST /api/v1/b3/jobs` junto com `operation=ingest`, `source`, `from` e `to` para o job adquirir exatamente o arquivo escolhido. O campo é opcional para manter a seleção server-side dos canais configurados em chamadas existentes. Consulte o [guia detalhado de descoberta](../docusaurus/docs/operations/discovery.md) para limites, erros e operação.

## Exportação integral COTAHIST em NDJSON

Use `GET /api/v1/b3/cotahist/quotes/export` para transferir todas as ocorrências de um ticker em uma revisão COTAHIST publicada. É uma transferência histórica finita, não um feed ao vivo. Ao contrário de `/cotahist/quotes`, não recebe intervalo de datas, `limit` ou `offset`, e não está sujeito ao máximo de 1.000 itens da paginação.

### Solicitação e seleção

Envie `Authorization: Bearer <credencial-de-leitura>`. Os parâmetros aceitos são:

| Parâmetro | Tipo | Obrigatório/default | Regra e efeito |
|---|---|---|---|
| `ticker_symbol` | string | Obrigatório | De 1 a 64 caracteres. Remove espaços externos e normaliza letras ASCII para maiúsculas. Filtra o código publicado; não identifica sozinho um instrumento permanente. |
| `data_revision` | UUID | Opcional | Fixa uma revisão publicada. Sem cursor, a revisão ativa é selecionada uma vez no começo e devolvida em `meta`. Com cursor, omita para usar a revisão dele ou envie o mesmo UUID. Revisão divergente retorna `400`. |
| `cursor` | string | Opcional | Valor opaco URL-safe emitido por `checkpoint`, com até 1.024 caracteres. Retoma exclusivamente após `(trade_date, id)` e está ligado ao ticker, à revisão e à versão do schema. Não o construa, edite ou use como credencial. |
| `include_internal` | boolean | `false` | Quando `true`, inclui `internal.version_id` e `internal.business_hash` em cada `HistoricalQuote`. Não amplia autorização. |

O filtro de ticker abrange as ocorrências daquele código na revisão, mesmo que os qualificadores `market_type`, `bdi_code`, `distribution_number` ou `term_days` sejam diferentes. Cada linha mantém esses qualificadores; não os colapse para formar uma observação diária única. A ordem é crescente por `trade_date` e UUID `id`, que desempata fatos da mesma data. Uma revisão publicada durante a chamada não altera o conjunto já selecionado.

Exemplo:

```http
GET /api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3&data_revision=00000000-0000-4000-8000-000000000001
Authorization: Bearer <credencial-de-leitura>
Accept: application/x-ndjson
Accept-Encoding: gzip
```

`Accept` não escolhe outra representação: o endpoint responde `application/x-ndjson; charset=utf-8`. `Accept-Encoding: gzip` permite gzip quando o artefato comprimido está pronto; uma geração inicial pode ser enviada sem compressão.

### Protocolo de eventos

O corpo é NDJSON UTF-8: cada objeto é serializado como uma linha terminada por LF (`\n`). Limites dos chunks HTTP não correspondem necessariamente a limites de linha; acumule bytes até LF antes de interpretar JSON. A ordem lógica é:

| Evento | Campos e significado |
|---|---|
| `meta` | Primeira linha. Contém `schema_version`, `ticker_symbol`, `data_revision`, `resumed_after` (`null` na primeira chamada) e `disclaimer` nominal. |
| `quote` | Uma ocorrência em `data`, conforme o schema `HistoricalQuote`; preserva decimais textuais, `identity` e `provenance`. |
| `checkpoint` | Emitido ao final de cada página interna de até 500 fatos. `cursor` retoma após o último fato da página. `records_sent` é o total acumulado de eventos `quote` nesta resposta, começando novamente do zero em cada retomada. |
| `complete` | Confirma o fim do segmento. `record_count` conta somente `quote` nesta resposta; `last_cursor` pode ser nulo quando não há cursor final; `content_sha256` cobre os bytes UTF-8 das linhas anteriores, incluindo LF, e exclui a linha `complete`. |
| `error` | Erro seguro após o envio dos headers. Contém `code`, `message` e `request_id`. Uma resposta com `error` não é concluída, mesmo se o processo receber o fim do corpo. |

O exemplo abaixo é estrutural e usa IDs/checksum sintéticos; o `cursor` ilustra o formato de um cursor válido para os campos exibidos:

```ndjson
{"type":"meta","schema_version":1,"ticker_symbol":"EXMP3","data_revision":"00000000-0000-4000-8000-000000000001","resumed_after":null,"disclaimer":"Preços históricos nominais brutos, na moeda e forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos."}
{"type":"quote","data":{"id":"00000000-0000-4000-8000-000000000010","trade_date":"2025-01-02","ticker_symbol":"EXMP3","market_type":"010","bdi_code":null,"distribution_number":null,"term_days":null,"issuer_short_name":null,"paper_specification":null,"currency":"BRL","open_price":"30.20","high_price":"30.20","low_price":"30.20","average_price":"30.20","last_price":"30.20","best_bid_price":null,"best_ask_price":null,"trades_count":1,"traded_quantity":"1","traded_volume":"30.20","option_exercise_price":null,"price_correction_indicator":null,"maturity_date":null,"quotation_factor":1,"exercise_price_points":null,"isin":null,"identity":{"instrument_id":null,"status":"unresolved"},"provenance":{"source_file_id":"00000000-0000-4000-8000-000000000011","source_file_type":"COTAHIST_YEARLY","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source_member_id":"00000000-0000-4000-8000-000000000012","source_member_name":"COTAHIST.2025.TXT","record_index":42}}}
{"type":"checkpoint","cursor":"eyJzY2hlbWFfdmVyc2lvbiI6MSwidGlja2VyX3N5bWJvbCI6IkVYTVAzIiwiZGF0YV9yZXZpc2lvbiI6IjAwMDAwMDAwLTAwMDAtNDAwMC04MDAwLTAwMDAwMDAwMDAwMDEiLCJ0cmFkZV9kYXRlIjoiMjAyNS0wMS0wMiIsInF1b3RlX2lkIjoiMDAwMDAwMDAtMDAwMC00MDAwLTgwMDAtMDAwMDAwMDAwMDEwIn0","records_sent":1}
{"type":"complete","record_count":1,"last_cursor":"eyJzY2hlbWFfdmVyc2lvbiI6MSwidGlja2VyX3N5bWJvbCI6IkVYTVAzIiwiZGF0YV9yZXZpc2lvbiI6IjAwMDAwMDAwLTAwMDAtNDAwMC04MDAwLTAwMDAwMDAwMDAwMDEiLCJ0cmFkZV9kYXRlIjoiMjAyNS0wMS0wMiIsInF1b3RlX2lkIjoiMDAwMDAwMDAtMDAwMC00MDAwLTgwMDAtMDAwMDAwMDAwMDEwIn0","content_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
```

### Retomada, integridade e repetição

Persista o checkpoint mais recente somente depois de armazenar de forma durável todos os eventos `quote` anteriores a ele. Se a conexão cair antes do próximo checkpoint, solicite novamente usando o cursor anterior; a chave exclusiva `(trade_date,id)` impede repetir os fatos já confirmados. Envie o mesmo ticker e cursor. Se `data_revision` for explícito, deve ser igual ao UUID embutido no cursor. A resposta retomada começa com um novo `meta`, conta os próprios eventos `quote` a partir de zero e possui seu próprio `content_sha256`; some `record_count` entre os segmentos concluídos para obter a contagem total da exportação.

Finalize an incremental SHA-256 digest with the exact uncompressed bytes of every line before `complete`, preserving each LF. Compare the lowercase hexadecimal digest to `complete.content_sha256`. Do not hash decoded objects or reserialized JSON: key ordering and whitespace are part of the byte representation. Stage output as provisional until `complete` is received and the digest matches. If an `error` event arrives, the body ends early, JSON parsing fails, or the digest differs, keep the last confirmed cursor and retry from it.

For a fixed revision, ticker, `include_internal` projection, and cursor, repeating the request returns the same NDJSON segment. The service reuses or joins generation of the derived artifact; no `Idempotency-Key` header is required for this read-only operation.

### HTTP headers, cache and failures

| Header/status | Comportamento |
|---|---|
| `Content-Type` | `application/x-ndjson; charset=utf-8`. |
| `Content-Encoding` | `gzip` somente quando a representação gzip pronta for selecionada; ausente para identidade. |
| `Cache-Control` | `private, max-age=0, must-revalidate`; respostas autenticadas não devem ir para cache/CDN público. |
| `Vary` | `Authorization, Accept-Encoding`, pois autorização e compressão variam a representação/resposta. |
| `ETag` | ETag forte de bytes da representação pronta (gzip ou NDJSON). Não é emitido enquanto a representação está sendo construída. |
| `Content-Length` | Presente somente para artefato pronto; fluxo em geração não declara tamanho final. |
| `If-None-Match` | `304` sem corpo somente se a representação escolhida já estiver pronta e o ETag coincidir. Em cache miss/geração, a chamada segue como `200` em fluxo. |

Antes dos headers, o envelope comum da API cobre `400` (filtro, cursor ou revisão inválida), `401` (Bearer inválido), `403` (fonte/revisão sem elegibilidade REST e `third_party_delivery`), `404` (ticker sem fatos na revisão), `500` (falha interna) e `503` (credenciais, revisão, PostgreSQL ou storage indisponível). Depois dos headers, o status HTTP não pode mudar: leia eventos `error` e exija `complete`. `200` sem `complete` não confirma sucesso; uma falha de rede também pode encerrar o corpo sem evento `error`.

O cache é interno, derivado e reconstruível; uma entrada pronta dura até 30 dias. Mesmo em cache hit, cada chamada resolve/revalida revisão e elegibilidade e executa uma consulta indexada de existência do ticker; não relê nem serializa todo o histórico do PostgreSQL. A implementação usa o filesystem em `B3_PROCESSED_DATA_DIR/quote-exports` e páginas keyset limitadas a 500 fatos ao gerar um artefato. Instâncias que compartilham artefatos precisam compartilhar esse filesystem. Isso não é garantia de latência: a meta inferior a um segundo para a primeira cotação ainda depende de benchmark representativo e não é um SLA publicado. Consulte o contrato máquina-legível em `GET /openapi.json`, o [guia detalhado de consumo](consumers_guides.md), os [endpoints COTAHIST](../docusaurus/docs/api-endpoints/cotahist.md) e o [schema dos eventos](../docusaurus/docs/schemas/cotahist.md).

## Precisão, revisão e cobertura

Valores financeiros são strings decimais. Preservar sua precisão; não converter para ponto flutuante para cálculos de auditoria. Moeda, fator de cotação e códigos originais continuam no registro. `last_price` corresponde ao último negócio publicado, sem garantir equivalência a outras medidas de fechamento.

Os preços são nominais brutos, sem ajuste retroativo por inflação ou eventos corporativos. Não há preenchimento de lacunas nem retorno total derivado apresentado como dado oficial.

Guardar `data_revision` da primeira página e enviá-lo nas páginas seguintes para fixar o conjunto. Ausência desse parâmetro seleciona a revisão ativa de cada requisição. `provenance` localiza arquivo, member e registro; `identity.status` mantém `resolved`, `unresolved` ou `ambiguous`. Vínculo ausente não invalida o preço preservado.

Cobertura não equivale a min/max de datas. Calendário desconhecido, rejeições, ausência de vínculo e limites do corpus permanecem explícitos. Uma dependência indisponível não é convertida em lista vazia.

`/cotahist/series` exige identificação do universo e retorna 409 quando várias linhas incompatíveis permanecem no mesmo dia. Acrescentar filtros de mercado, distribuição ou prazo até a seleção ser inequívoca.

## Evidências cadastrais

No contrato técnico, `/instruments` consulta fotografias publicadas por família. Porém, no `0.0.1`, BVBG.028 e Tradable Security List estão `deferred`, pois a data da fotografia não substitui o corte por data interna do registro. Essas rotas não representam uma fonte de identidade disponível neste release; consulte `/capabilities`.

Quando fotografia cadastral for habilitada em versão futura, fixe `snapshot_id` ao paginar. Não use um cadastro atual como mapeamento automático de todas as cotações antigas.

`GET /instruments/{id}`, `/instruments/{id}/identifiers` e `/tradable-securities` permanecem no schema, mas não retornam observações produzidas pelo fluxo suportado em `0.0.1`. Cotações sem vínculo cadastral continuam disponíveis com estado `unresolved`.

## Administração e MCP

Operações pesadas são comandos em `POST /jobs`; 202 significa intenção persistida, não conclusão. Acompanhar `status_url`, tentativas e outputs; cancelamento é solicitação durável e preserva confirmações anteriores. Arquivos, inventário, pendências, revisões, conflitos e quarentena têm inspeção administrativa própria.

## Importação e retirada administrativa

Somente um administrador inicia ingestão de uma família/canal oficial já configurado ou registra explicitamente um candidato local para desenvolvimento interno. Não existe upload de arquivos por consumidores. Importação local preserva origem e proveniência, e sua elegibilidade não habilita entrega a terceiros.

Todo comando `ingest` requer `from` e `to`, inclusivos. No `0.0.1`, só as granularidades COTAHIST são aceitas: o worker decide quais detalhes pertencem ao import pela `DATA DO PREGÃO` dentro de cada registro, ainda que adquira um arquivo anual. Nome, ano, header ou data de geração do arquivo só ajudam a localizar/identificar a publicação. Importar o ano inteiro exige enviar o intervalo completo explicitamente. Uma seleção que exige vários arquivos COTAHIST é consolidada numa revisão. BVBG.028/TSL são rejeitadas até que seu corte possa se basear numa data interna adequada de cada registro.

Um import limpo publica automaticamente depois de validações estruturais e dos gates de rejeição/conflito/elegibilidade. Se encontrar registros rejeitados ou conflito sem precedência, mantém a candidata fora da visão ativa e termina `succeeded_with_issues`. `withdraw_window` retira logicamente um intervalo da visão ativa numa nova revisão; raw, fatos/versões, auditoria e revisões anteriores não são apagados, permitindo rollback ou reimportação idempotente.

Elegibilidade externa é avaliada por fonte/canal/finalidade e pode deixar dados indisponíveis mesmo com os endpoints e parsers no software. Consulte [elegibilidade de fontes](release_source_eligibility.md) antes de habilitar um canal real.

MCP usa stdio. O cliente realiza initialize, envia notifications/initialized e consulta tools/list. As ferramentas usam os mesmos serviços de REST, com identidade, revisão, precisão e proveniência equivalentes. Não há sync, replay, rebuild, SQL arbitrário ou transporte HTTP MCP.
