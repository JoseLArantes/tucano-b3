# Tucano B3

Serviço Rust para preservar e consultar histórico oficial da B3, com proveniência, precisão decimal, identidade temporal e publicação por revisões.

O primeiro lançamento **0.0.1** entrega a plataforma de ingestão histórica, sem depender de um corpus pré-carregado. Neste corte, importação é limitada a COTAHIST anual, mensal ou diário, por datas de negociação dentro dos registros; BVBG.028 e Tradable Security List ficam adiadas até oferecerem uma referência interna adequada para o mesmo contrato de cortes. O [PRD](docs/b3_prd.md) define os requisitos; o [plano de execução](docs/0.0.1-plan.md) registra o estado do release.

**Canais de dados começam desativados.** Nenhuma fonte é habilitada por seed. A [elegibilidade por fonte, canal e finalidade](docs/release_source_eligibility.md) governa a ativação e o uso externo dos dados; ela não exige carregar um corpus para declarar a plataforma `0.0.1` completa. Fixtures sintéticas validam comportamento do software, não autenticidade nem direitos de redistribuição de dados reais.

## Contratos do primeiro lançamento

- REST em `/api/v1/b3/...`, com OpenAPI gerado em `/openapi.json` e Swagger UI interativa em `/docs`.
- Consumidores e administradores usam tokens Bearer distintos. Credencial de leitura não autoriza ingestão, replay, rebuild, publicação ou rollback.
- Operações administrativas persistem jobs e retornam `202 Accepted` com localização de acompanhamento. O aceite HTTP não significa conclusão da ingestão.
- Listas usam `limit=100`, máximo `1000`, `offset=0` e ordem determinística. Revisão informada no payload pode ser fixada nas páginas seguintes.
- Cotações têm proveniência até arquivo/member/registro, estado da identidade e limitações de cobertura. Valores financeiros são strings decimais.
- `GET /api/v1/b3/cotahist/quotes/export` transmite o histórico integral de um ticker em NDJSON finito, com revisão fixada, checkpoints retomáveis e artefatos cacheados reconstruíveis em `B3_PROCESSED_DATA_DIR`.
- MCP inicial usa stdio, os mesmos serviços de consulta e credencial PostgreSQL somente leitura. Logs vão para stderr; stdout contém apenas o protocolo.
- BVBG.028 e Tradable Security List ficam adiadas até que a seleção dos registros por `from`/`to` possa usar uma data de negócio interna. BVBG.086/186, BVBG.087 e carteiras permanecem nos marcos G6/G7, sem fallback silencioso para COTAHIST.

> COTAHIST representa preços históricos nominais brutos, na moeda e base de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. `PREULT` não é renomeado como um fechamento oficial genérico.

## Modelo de execução

A descoberta leve registra checagens de canais e percorre os locais definidos em `B3_DISCOVERY_LOCATIONS` (`folder:`, `b3_url:` e `object_storage_bucket:`), retornando metadados COTAHIST sem baixar arquivos. No Compose, a pasta local `./data/imports` é montada somente leitura em `/data/raw/imports`. Para S3 compatível, use o provedor de credenciais AWS e `AWS_ENDPOINT_URL_S3` opcional com permissão restrita a listar objetos. Um candidato encontrado não vira canal elegível nem inicia ingestão. Administradores solicitam aquisição/ingestão com cortes `from`/`to`; um import limpo publica automaticamente após os gates de qualidade. O worker consome jobs duráveis PostgreSQL, com lanes de manutenção/trabalho pesado, tentativas, lease, heartbeat, fencing e cancelamento.

A aquisição preserva raw por SHA-256 e inventaria ZIPs aninhados em disco. O processamento produz registros tipados em lotes limitados e quarentena localizada. A revisão anterior continua consultável até a promoção atômica da candidata completa. Divergências sem precedência comprovada não são resolvidas pela ordem de download.

Identificadores de instrumento são temporais e qualificados. Ticker isolado não é identidade permanente; cadastro atual não cria vínculos retroativos com todo o histórico. Preços válidos sem cadastro continuam preservados.

## Stack

Rust/Tokio, Axum, SQLx/PostgreSQL 16, quick-xml, rust_decimal e Utoipa/OpenAPI 3.1. REST, CLI, worker e MCP compartilham serviços. Raw fica em filesystem persistente. Não há Redis ou Celery.

## Desenvolvimento com Compose

1. Copiar `.env.example` para `.env` e definir dois tokens Bearer distintos e não vazios.
2. Executar `docker compose up -d`. Compose sobe PostgreSQL, executa `b3_migrate` como serviço de tarefa única e só inicia API/worker depois que a migration termina com sucesso.
3. API, worker e MCP verificam o schema na inicialização; os processos de runtime não executam DDL. O container separado `b3_migrate` usa `B3_MIGRATION_DATABASE_URL` para aplicar migrations.
4. Configurar fontes/canais elegíveis antes de habilitar aquisição; a stack inicia com discovery e fontes externas desativadas.

```bash
docker compose config --quiet
docker compose up -d
docker compose ps --all
curl --fail http://localhost:8010/healthz
curl --fail http://localhost:8010/api/v1/health
```

Compose constrói imagens ausentes durante `up`. Depois de alterações de código, execute `docker compose up --build -d`. Para aplicar migrations sem iniciar a stack, rode `docker compose run --rm b3_migrate`; esse é o mesmo comando de migração isolado usado por automações de release.

A API usa a porta host `8010` e PostgreSQL `5433`; na rede Compose, `8000` e `5432`. O exemplo PostgreSQL é local e não define credenciais de produção. Não usar `docker compose down -v` como rotina: remove evidência persistente.

O volume `b3_processed_files` guarda apenas artefatos derivados de exportação, com expiração de 30 dias; eles podem ser reconstruídos a partir das revisões PostgreSQL. Em uma implantação com várias instâncias de API, configure `B3_PROCESSED_DATA_DIR` para um filesystem persistente compartilhado por elas. A composição atual não fornece armazenamento de objetos multi-host.

```bash
docker compose --profile tools run --rm cargo cargo fmt --all -- --check
docker compose --profile tools run --rm cargo cargo clippy --all-targets --all-features -- -D warnings
docker compose --profile tools run --rm cargo cargo test --all-targets --all-features
```

Testes de persistência/concorrência exigem PostgreSQL 16 real em base isolada. A ausência de banco não pode produzir falso verde. Testes HTTP determinísticos usam mocks; aquisição real é separada, limitada e depende de elegibilidade.

O serviço `cargo` usa `tucano_b3_test`, separado do banco de operação. Na primeira preparação local, crie a base e aplique nela as migrations antes de executar testes:

```bash
docker compose exec -T b3_postgres createdb -U postgres tucano_b3_test
docker compose --profile tools run --rm cargo cargo run --bin tucano-b3 -- migrate
```

Não aponte `B3_TEST_DATABASE_URL` para `tucano_b3` ou outra base com dados operacionais. Testes de integração criam e removem apenas bases de nomes aleatórios dentro do servidor indicado.

## CLI

```bash
tucano-b3 migrate
tucano-b3 enqueue --command-file comando.json --idempotency-key chave-unica
tucano-b3 worker --once --lane maintenance
tucano-b3 worker --once --lane heavy
tucano-b3 discover
tucano-b3 mcp
```

O arquivo de comando é tipado; não contém SQL arbitrário. Configurar `DATABASE_URL` com o papel adequado antes de executar. O MCP exige papel somente leitura, inclusive leitura dos metadados de versão do schema.

## Operação e evidências

O [guia operacional](docs/release_operations.md) descreve backups consistentes SQL/raw, restore em banco novo, limites e evidências de medição. O [guia detalhado de consumo da API](docs/consumers_guides.md) organiza rotas, filtros e fluxos para interfaces. O [changelog de contratos](docs/frontend_api_changelog.md) acompanha mudanças voltadas a consumidores.

## Documentação

O site para consumidores e operadores está configurado para publicação em [Tucano B3 Docs](https://joselarantes.github.io/b3-tucano/); o workflow de GitHub Pages publica alterações em docusaurus/ na branch main. O código fonte fica em [docusaurus/](docusaurus/). Consulte os guias de conceitos, fontes B3, endpoints, schemas e operações; eles acompanham o contrato gerado em /openapi.json.

Para servir e validar o site localmente:

    npm --prefix docusaurus ci
    npm --prefix docusaurus start
    npm --prefix docusaurus run build

O servidor Docusaurus usa http://localhost:3000. Ele é separado da API em execução: /docs redireciona para a Swagger UI em /docs/, ligada ao contrato do binário, e /openapi.json entrega o documento legível por máquina; o site Docusaurus contém guias completos para pessoas e documentação operacional. A UI Swagger permite executar chamadas reais, mas as rotas protegidas continuam exigindo o Bearer e o nível de autorização adequados.

Referências: [marcos](docs/prd_dev_milestones.md), [backlog](docs/issues/README.md), [pesquisa de fontes](docs/RESEARCH.md), [guia B3](docs/B3.md), [referência técnica das famílias](docs/ref_b3.md), [contextos](CONTEXT-MAP.md) e [revisão inicial](docs/19-plan.md). Documentação e aprovação de PRD não equivalem a uma publicação externa.
