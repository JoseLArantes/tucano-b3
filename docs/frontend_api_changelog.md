# Alterações da API para consumidores

## 08/11/2026 — Descoberta configurável de fontes COTAHIST

- Amplia `POST /api/v1/b3/discovery/scan` para percorrer `B3_DISCOVERY_LOCATIONS`: pastas, URLs HTTPS diretas de arquivos em hosts oficiais B3 e buckets AWS S3 compatíveis com prefixo.
- A resposta `DiscoveryResult` agora inclui totais e truncamento, estado por local e até 1.000 candidatos com família inferida, locador sem segredo e metadados. Cada checagem e candidato fica persistido em tabelas próprias.
- A descoberta permanece metadata-only: pasta sem seguir symlink, HEAD para URL B3 e ListObjectsV2 para S3. Não baixa, cria arquivo bruto, cria pendência de canal nem inicia ingestão. Um candidato requer canal configurado e elegibilidade antes de uso.
- Configuração: `AWS_REGION`, `AWS_ENDPOINT_URL_S3` e credenciais do provedor AWS para listar buckets; Compose monta `./data/imports` somente leitura em `/data/raw/imports`. Limites: 32 locais, 10.000 entradas por local e 1.000 candidatos na resposta.
- Compatibilidade: mudança aditiva na resposta do endpoint existente. Clientes que validam objetos fechados devem aceitar os novos campos; nenhum parâmetro de requisição foi adicionado.

## 08/11/2026 — Swagger UI vinculada ao OpenAPI em execução

- `GET /docs` redireciona com `303` para `GET /docs/`, que serve Swagger UI com “Try it out” e carrega o documento gerado e publicado pelo mesmo binário em `GET /openapi.json`.
- Os assets GET sob `/docs/` são públicos para permitir carregar a interface. As chamadas a operações continuam sujeitas às regras Bearer e aos papéis de leitura/administração já publicados; a UI não concede acesso.
- Compatibilidade: nenhuma operação, campo ou semântica de payload REST mudou. A documentação narrativa detalhada continua no Docusaurus.

## 08/11/2026 — Exportação integral COTAHIST em NDJSON

- Adiciona `GET /api/v1/b3/cotahist/quotes/export`, protegido por Bearer de leitura, com `ticker_symbol` obrigatório e `data_revision`, `cursor` e `include_internal` opcionais.
- Responde com linhas `meta`, `quote`, `checkpoint`, `complete` ou `error`; consumidores devem validar `complete` e `content_sha256`, e podem retomar depois do checkpoint mais recente na mesma revisão.
- O formato `HistoricalQuote` mantém precisão decimal textual, identidade, proveniência e o aviso de preços nominais sem ajuste. A lista paginada existente não muda.
- Cache derivada é privada, revalidada por elegibilidade e expira após 30 dias. ETag e `304` aplicam-se somente a artefatos prontos. Compatibilidade: aditiva; nenhum campo ou semântica da rota `/quotes` existente mudou.
- Runtime local: configurar/compartilhar `B3_PROCESSED_DATA_DIR` para que instâncias da API usem o mesmo volume. Não se anuncia ainda a meta de primeira cotação inferior a um segundo sob 50 consumidores; ela depende de benchmark reproduzível.

## 06/11/2026 — COTAHIST como única fonte de import no 0.0.1

- ``GET /api/v1/b3/capabilities`` marca BVBG.028 e Tradable Security List como ``deferred``; a data de uma fotografia ou arquivo não satisfaz corte por data interna de cada registro.
- ``POST /api/v1/b3/jobs`` aceita ``ingest`` somente para COTAHIST diário, mensal ou anual. Solicitações de import cadastral retornam ``400`` antes de criar um job.
- As rotas de consulta cadastral permanecem no schema técnico, mas não recebem dados pelo fluxo suportado do release. Cotações COTAHIST seguem consultáveis com identidade ``unresolved`` quando não há evidência habilitada.
- Compatibilidade: decisão de escopo anterior ao lançamento estável `0.0.1`; clientes de preview não devem depender de ingestão cadastral.

## 06/11/2026 — Cortes por data de negócio e retirada lógica (contrato 0.0.1)

- `POST /api/v1/b3/jobs`: `ingest.from` e `ingest.to` têm semântica inclusiva sobre a data interna de negócio. Para COTAHIST, os detalhes fora do corte são contados como `out_of_scope`; a data/nome do arquivo apenas ajuda a localizar publicações.
- `POST /api/v1/b3/jobs`: adiciona `withdraw_window { from, to }`, que publica uma visão sem os fatos do intervalo solicitado, preservando raw, versões e revisões anteriores para rollback/reimportação.
- Promoção: import limpo publica automaticamente. Rejeições ou conflitos deixam a candidata fora da visão ativa e produzem `succeeded_with_issues` até decisão administrativa.
- GET `/api/v1/b3/data-revisions/{id}`: acrescenta `revision_kind`, `source_file_ids` e `out_of_scope` para explicar revisões agregadas, retiradas e filtros de data.
- Vários artefatos COTAHIST selecionados para um mesmo job pertencem a uma única revisão candidata e troca atômica do ponteiro ativo.
- Import é reservado a administradores sobre fontes oficiais configuradas; não há upload de arquivo por consumidor final. Requisitos de autorização continuam por canal/finalidade.

Compatibilidade: contrato inicial `0.0.1` é definido antes do primeiro lançamento; não há obrigação de compatibilidade com payloads ou IDs anteriores. Consumers que já estejam seguindo o preview devem alinhar DTOs de revisão e estados de job a esta definição antes do release.

## 06/11/2026 — Contrato inicial 0.0.1 qualificado; ativação externa por canal

Primeiro contrato suportado, sem obrigação de compatibilidade com a implementação anterior ao lançamento. A plataforma 0.0.1 não depende de corpus oficial pré-carregado. Elegibilidade externa é um gate por fonte/canal/finalidade para servir dados reais; fixtures sintéticas qualificam o comportamento do contrato, não autorizam a distribuição de dados.

- Namespace REST: `/api/v1/b3`; versão de software independente da versão do namespace.
- Consulta COTAHIST: `/cotahist/quotes` e `/cotahist/series`, com valores financeiros em strings, `data_revision`, cobertura, estado de identidade e proveniência até member/registro. Série ambígua retorna 409, sem escolher uma linha arbitrária.
- Cadastro: `/instruments` retorna evidências de fotografias explicitamente publicadas, com `snapshot_id`, namespace do código e origem. Não transforma fotografia atual em validade histórica.
- Cadastro detalhado: `/instruments/{id}` mantém as evidências de snapshots distintos em itens separados; `/instruments/{id}/identifiers` retorna valores, qualificadores, validade demonstrada e proveniência; `/tradable-securities` força a família TSL sem fallback a BVBG.028.
- Administração: `POST /jobs` retorna 202 com `job_id`, `status_url` e `Location`. `GET /jobs`, detalhe e cancelamento usam os mesmos estados duráveis da CLI e do worker. Nenhuma aquisição é executada durante consultas analíticas.
- Inspeção: arquivos/members, revisões/conflitos/quarentena, pendências e catálogo de capacidades. Famílias posteriores a G5 ficam explicitamente adiadas no catálogo.
- Autorização: credenciais Bearer distintas de leitura/administração; 401 para credencial inválida e 403 para leitura tentando administrar. Variável antiga `TUCANO_B3_TOKEN` não concede acesso ao contrato novo.
- Erros REST: `code`, `message`, `request_id`, sem SQL, credenciais ou caminhos privados.
- MCP inicial: stdio, ferramentas analíticas somente leitura, com os mesmos serviços de consulta; índices/carteiras não são expostos antes de sua homologação.

Compatibilidade: consumidores pré-lançamento devem adotar o contrato inicial e suas credenciais novas. Reprocessamento, republicação e rebuild posteriores ao lançamento devem preservar as garantias de identidade, versão e auditoria do PRD.
