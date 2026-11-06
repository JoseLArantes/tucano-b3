# Tucano B3 API (Rust Edition)

[![API](https://img.shields.io/badge/API-Production-0A66C2)](https://b3.tucano.beakcloud.com)
[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-black?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8-000000?logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
[![SQLx](https://img.shields.io/badge/SQLx-0.8-336791?logo=postgresql&logoColor=white)](https://github.com/launchbadge/sqlx)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-336791?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Advisory Locks](https://img.shields.io/badge/Queue%20%26%20Locks-Zero%20Redis%20%2F%20Zero%20Celery-brightgreen)](https://www.postgresql.org/docs/16/explicit-locking.html#ADVISORY-LOCKS)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.1-6BA539?logo=openapiinitiative&logoColor=white)](https://swagger.io/specification/)
[![Docker Compose](https://img.shields.io/badge/Docker%20Compose-v2-2496ED?logo=docker&logoColor=white)](https://docs.docker.com/compose/)

Motor de normalização e exposição de dados públicos oficiais da **B3 (Brasil, Bolsa, Balcão)**, reconstruído inteiramente em **Rust**, com arquitetura assíncrona baseada em Tokio/Axum, persistência nativa em PostgreSQL 16 (sem dependência de Redis ou Celery) com evolução orientada pelo [PRD de produto](docs/b3_prd.md).

Documentação de referência: [plano de revisão e migração](docs/19-plan.md), [pesquisa de fontes](docs/RESEARCH.md), [mapa de contextos](CONTEXT-MAP.md) e [backlog de desenvolvimento](docs/issues/README.md).

---

## Índice

- [Objetivo e Filosofia](#objetivo-e-filosofia)
- [Escopo Estrito de Dados](#escopo-estrito-de-dados)
- [Arquitetura e Destaques Técnicos](#arquitetura-e-destaques-tícnicos)
- [Aviso Regulatório Obrigatório: Dados de Preço COTAHIST](#aviso-regulatório-obrigatório-dados-de-preço-cotahist)
- [Tecnologias e Bibliotecas](#tecnologias-e-bibliotecas)
- [Autenticação](#autenticação)
- [Documentação Interativa (Swagger & ReDoc)](#documentação-interativa-swagger--redoc)
- [Endpoints REST](#endpoints-rest)
- [Model Context Protocol (MCP Server)](#model-context-protocol-mcp-server)
- [Interface CLI](#interface-cli)
- [Execução com Docker Compose](#execução-com-docker-compose)
- [Comandos Cargo via Docker Compose](#comandos-cargo-via-docker-compose)
- [Testes Automatizados](#testes-automatizados)
- [Variáveis de Ambiente](#variáveis-de-ambiente)
- [Licença](#licença)

---

## Objetivo e Filosofia

Disponibilizar dados oficiais de mercado da B3 via API REST de alta performance, garantindo:
1. **Proveniência auditável**: rastreabilidade completa por arquivo fonte (`b3_source_files`), checksum SHA-256 e tamanho exato em bytes.
2. **Idempotência absoluta e auditoria granular**: reprocessar o mesmo arquivo com o mesmo payload gera zero escritas redundantes. Alterações parciais são detectadas campo a campo e auditadas com histórico temporal em `b3_field_changes`.
3. **Isolamento entre famílias de arquivos**: sem merges implícitos ou confusão semântica entre `BVBG.086` (PriceReport abrangente) e `BVBG.186` (Simplified Equities PriceReport).
4. **Resolução temporal de relacionamentos**: mapeamento determinístico de instrumentos e identificadores baseado em janelas de validade (`valid_from` / `valid_to`), precedência regulatória e isolamento de colisões ambíguas.
5. **Zero Redis / Zero Celery**: eliminação de complexidade operacional através do uso nativo de travas consultivas (`pg_try_advisory_xact_lock` / `pg_try_advisory_lock`) e filas ACID do PostgreSQL 16.

---

## Escopo Estrito de Dados

O escopo deste projeto é restrito aos arquivos de dados públicos distribuídos oficialmente pela B3:
- **`BVBG.028.02`**: Cadastro oficial de instrumentos financeiros negociados (ações, opções, futuros, ETFs, BDRs, etc.).
- **`Tradable Security List`**: Listagem de segurança negociável (CSV oficial da B3).
- **`BVBG.086.01`**: Boletim PriceReport oficial contendo cotações de fechamento, médias, máximas, mínimas e volumes.
- **`BVBG.186.01`**: Simplified Equities PriceReport para o mercado de ações.
- **`BVBG.087.01`**: IndexReport com cotações e fechamentos dos índices oficiais da B3.
- **`Composição de Índices`**: Carteiras teóricas oficiais dos índices (IBOV, IBrX-100, SMLL, etc.).
- **`COTAHIST`**: Séries históricas oficiais em arquivo de largura fixa de 245 bytes.

> [!IMPORTANT]
> **Fronteira Operacional**: Nenhuma fonte externa (CVM, Yahoo Finance, corretoras ou APIs de cotações em tempo real com atraso) faz parte desta base. A ingestão de fontes CVM pertence ao repositório irmão `tucano-cvm`.

---

## Arquitetura e Destaques Técnicos

```mermaid
flowchart TD
    B3[Portal de Dados Públicos B3] -->|Downloads Seguros| SyncEngine[Rust Sync Engine / Streaming Parsers]
    SyncEngine -->|quick-xml / csv / slice parser| Normalizer[Canonical Normalization & Hasher]
    Normalizer -->|pg_advisory_lock & pg_advisory_xact_lock| Upsert[Changed-Field Upsert & Audit Trail]
    Upsert --> Postgres[(PostgreSQL 16\nb3_* tables & JSONB)]
    Postgres --> Resolver[Temporal Relationship Resolver]
    Postgres --> AxumAPI[Axum 0.8 REST API\nBearer Auth & OpenAPI 3.1]
    Postgres --> MCPServer[MCP stdio JSON-RPC Server]
    AxumAPI --> Client[Swagger UI / ReDoc / Clients]
    MCPServer --> AIAgents[AI Coding Agents & Claude Desktop]
```

### 1. Parsers Streaming de Baixa Alocação
- **XML**: Leitura sem alocação do arquivo inteiro em memória via `quick-xml` com decodificação streaming e buffer circular.
- **COTAHIST**: Decodificador de fatias de bytes (`&[u8]`) para registros fixos de 245 bytes com conversão de ponto decimal implícito (`V99`) e parsing direto de datas sem regex.
- **ZIP Recursivo**: Descompactador automático streaming com sanitização contra ataques de path traversal (*Zip Slip*).

### 2. Transações ACID e Travas Consultivas
- Controle de concorrência global e por data de referência via travas consultivas nativas do PostgreSQL (`pg_try_advisory_xact_lock` e `pg_try_advisory_lock`).
- Previne corridas entre o daemon agendado (`worker`), solicitações de ingestão manual da API (`POST /api/v1/b3/sync/run`) e execuções via CLI (`tucano-b3 sync`).

---

## Aviso Regulatório Obrigatório: Dados de Preço COTAHIST

> [!WARNING]
> **AVISO IMPORTANTE SOBRE DADOS DE PREÇO (COTAHIST)**:
> Os preços fornecidos pelo arquivo oficial COTAHIST da B3 representam **preços nominais históricos brutos (unadjusted)** praticados na data da negociação.
> Eles **NÃO sofrem ajuste retroativo** por eventos corporativos (como dividendos, juros sobre capital próprio, grupamentos, desdobramentos ou bonificações). Séries ajustadas requerem cálculo ex-dividendo externo cruzado com a base de eventos corporativos da CVM.
>
> Este aviso é exibido de forma permanente no OpenAPI 3.1, na interface Swagger/ReDoc e no cabeçalho do envelope JSON da API REST:
> ```json
> {
>   "disclaimer": "Precos historicos brutos nominais oficiais da B3 (unadjusted). Nao refletem eventos corporativos retroativos.",
>   "data": [ ... ]
> }
> ```

---

## Tecnologias e Bibliotecas

| Componente | Tecnologia | Versão | Função |
| :--- | :--- | :--- | :--- |
| **Linguagem** | Rust | 1.85+ (Edition 2021) | Concorrência segura e alta performance |
| **Framework Web** | Axum | 0.8 | Servidor HTTP assíncrono sobre Tokio |
| **Driver de Banco** | SQLx | 0.8 | Acesso assíncrono ao PostgreSQL com pool nativo |
| **OpenAPI & Docs**| Utoipa & Swagger UI | 5.3 / 5.2 | Especificação OpenAPI 3.1 com Swagger UI e ReDoc |
| **Parsers** | quick-xml, csv | 0.37 / 1.3 | Parsing streaming de arquivos XML e CSV |
| **Banco de Dados**| PostgreSQL | 16 | Banco relacional com travas consultivas e partições |
| **CLI** | Clap | 4.5 | Interface de linha de comando tipada |

---

## Autenticação

Todos os endpoints da API (exceto `/api/v1/health`, `/openapi.json`, `/docs` e `/redoc`) exigem autenticação via cabeçalho HTTP Bearer:

```http
Authorization: Bearer <TUCANO_B3_TOKEN>
```

Em ambiente local e Docker Compose de desenvolvimento, o token padrão configurado é `dev-token`.

---

## Documentação Interativa (Swagger & ReDoc)

Com os serviços em execução, acesse pelo navegador:
- **Swagger UI com suporte a Bearer Auth**: [http://localhost:8010/docs](http://localhost:8010/docs) (ou `/swagger-ui`)
- **ReDoc Interativo**: [http://localhost:8010/redoc](http://localhost:8010/redoc)
- **Especificação OpenAPI 3.1 JSON**: [http://localhost:8010/openapi.json](http://localhost:8010/openapi.json)

> [!TIP]
> No Swagger UI, clique no botão verde **Authorize** no canto superior direito e insira o valor `dev-token` para executar chamadas de teste diretamente na interface.

---

## Endpoints REST

### 1. Sistema e Saúde
- `GET /api/v1/health`: Diagnóstico de conectividade com PostgreSQL e status do sistema.
- `GET /openapi.json`: Especificação completa OpenAPI 3.1.
- `GET /docs` ou `/swagger-ui`: Interface Swagger UI.
- `GET /redoc`: Interface ReDoc.

### 2. Instrumentos (`BVBG.028.02`)
- `GET /api/v1/b3/instruments`: Listagem paginada de instrumentos com filtros (`ticker_symbol`, `asset_code`, `instrument_type`, `limit`, `offset`).
- `GET /api/v1/b3/instruments/{id}`: Detalhes por ID interno ou `security_id`.
- `GET /api/v1/b3/instruments/by-ticker/{ticker_symbol}`: Busca por símbolo de negociação.
- `GET /api/v1/b3/instruments/by-isin/{isin}`: Busca por código ISIN oficial.
- `GET /api/v1/b3/instruments/{id}/identifiers`: Histórico de identificadores com janelas de validade temporal (`valid_from` a `valid_to`).

### 3. Tradable Security List
- `GET /api/v1/b3/tradable-securities`: Listagem com filtros por ticker, data de referência e paginação.
- `GET /api/v1/b3/tradable-securities/by-date/{reference_date}`: Dados de instrumentos negociáveis em uma data específica.
- `GET /api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}`: Histórico de listagem negociável do ativo.

### 4. Boletins PriceReport (`BVBG.086` e `BVBG.186`)
- `GET /api/v1/b3/price-reports/bvbg086`: Cotações oficiais completas do PriceReport.
- `GET /api/v1/b3/price-reports/bvbg186`: Cotações do Simplified Equities PriceReport.

### 5. Índices e Composições de Carteira (`BVBG.087`)
- `GET /api/v1/b3/indices`: Séries de cotações e pontuações dos índices oficiais.
- `GET /api/v1/b3/indices/compositions`: Carteiras teóricas oficiais e participações percentuais dos ativos nos índices.

### 6. Cotações Históricas (`COTAHIST`)
- `GET /api/v1/b3/cotahist`: Cotações diárias com o aviso de preços nominais unadjusted obrigatório.

### 7. Sincronização e Operação
- `POST /api/v1/b3/sync/run`: Disparo de sincronização de ingestão com controle de travas consultivas.
- `POST /api/v1/b3/sync/resolve-relationships`: Reexecução de resolução de relacionamentos entre identificadores e instrumentos.
- `GET /api/v1/b3/sync/runs`: Histórico de execuções com status (`completed`, `failed`), estatísticas de registros inseridos, modificados e inalterados.
- `GET /api/v1/b3/sync/coverage`: Relatório de cobertura e disponibilidade temporal por família de arquivos.

---

## Model Context Protocol (MCP Server)

O binário `tucano-b3` inclui um servidor nativo **Model Context Protocol (MCP)** em modo `stdio` com protocolo JSON-RPC, permitindo que agentes de inteligência artificial (Claude Desktop, Cursor, Antigravity) interajam diretamente com os dados de mercado:

Ferramentas MCP disponíveis:
- `b3_healthcheck`: Verificação de saúde operacional e integridade da base.
- `b3_get_instrument`: Consulta detalhada de instrumentos por ticker ou código ISIN.
- `b3_get_daily_summary`: Resumo diário de cotações e volumes para um ativo.
- `b3_get_index_composition`: Composição teórica e peso dos ativos em índices (ex: `IBOV`).
- `b3_run_sync`: Disparo de ingestão e sincronização de dados sob demanda.

Para iniciar o servidor MCP:
```bash
tucano-b3 mcp
```

---

## Interface CLI

O binário disponibiliza subcomandos via `clap`:

```bash
# Iniciar o servidor HTTP REST
tucano-b3 serve

# Iniciar o daemon de sincronização em segundo plano (respeita fins de semana)
tucano-b3 worker

# Executar sincronização manual de um arquivo ou intervalo de datas
tucano-b3 sync --source-type bvbg028_instruments_file --start-date 2026-03-01 --end-date 2026-03-01

# Iniciar o servidor MCP sobre stdio
tucano-b3 mcp
```

---

## Execução com Docker Compose

A infraestrutura completa sobe via Docker Compose:
1. `b3_postgres`: PostgreSQL 16 Alpine na porta interna `5432` (mapeada para `5433` no host).
2. `b3_api`: Servidor REST Axum exposto na porta `8010` (porta interna `8000`).
3. `b3_worker`: Daemon em segundo plano executando o loop de atualização diária.

```bash
# Iniciar todos os serviços
docker compose up -d

# Verificar o status dos containers
docker compose ps

# Visualizar logs em tempo real
docker compose logs -f b3_api b3_worker
```

---

## Comandos Cargo via Docker Compose

Conforme a diretriz do ambiente de desenvolvimento, comandos de verificação do compilador Rust (`cargo fmt`, `cargo clippy`, `cargo test`) devem ser executados através do serviço `cargo` do Docker Compose:

```bash
# Verificar formatação do código
docker compose run --rm cargo cargo fmt --check

# Aplicar formatação automática
docker compose run --rm cargo cargo fmt

# Executar linter Clippy com zero tolerância a warnings
docker compose run --rm cargo cargo clippy --all-targets --all-features -- -D warnings

# Executar a suíte completa de testes unitários e de integração
docker compose run --rm cargo cargo test
```

---

## Testes Automatizados

A suíte de testes cobre todas as camadas do sistema:

- `tests/canonical_and_hash_tests.rs`: Normalização canônica de strings, tickers, ISINs, decimais com ponto implícito e hashing determinístico.
- `tests/parser_tests.rs`: Validação de streaming parsers para `BVBG.028`, `BVBG.086`, `BVBG.186`, `BVBG.087`, `Tradable Security List` e `COTAHIST`.
- `tests/download_and_lock_tests.rs`: Proteção contra domínios não permitidos (*SSRF protection*) e travas consultivas concorrentes.
- `tests/sync_service_integration_tests.rs`: Pipeline completo de ponta a ponta com persistência em PostgreSQL.
- `tests/upsert_and_audit_tests.rs`: Idempotência, isolamento JSONB e auditoria granular de campos alterados em `b3_field_changes`.
- `tests/resolver_and_rebuild_tests.rs`: Janelas de validade temporal, precedência de resolução e isolamento de colisões ambíguas.
- `tests/api_contract_tests.rs`: Autenticação Bearer, OpenAPI 3.1, endpoints REST e envelope de disclaimer regulatório do COTAHIST.
- `tests/worker_scheduler_tests.rs`: Ciclo do daemon de background e regras de calendário de negociação (pular fins de semana).

---

## Variáveis de Ambiente

| Variável | Padrão | Descrição |
| :--- | :--- | :--- |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5433/tucano_b3` | Conexão com PostgreSQL |
| `TUCANO_B3_TOKEN` | `dev-token` | Token Bearer de autorização da API |
| `B3_RAW_DATA_DIR` | `/data/raw` | Diretório base para armazenamento de arquivos brutos |
| `B3_ALLOWED_HOSTS` | `www.b3.com.br,b3.com.br` | Lista de hosts autorizados para downloads |
| `B3_SYNC_ENABLED` | `true` | Ativação do agendador automático no worker |
| `B3_MANUAL_SYNC_ENABLED`| `true` | Habilitação do endpoint de disparo manual |
| `B3_DEFAULT_TIMEZONE`| `America/Sao_Paulo` | Fuso horário operacional da B3 |
| `LOG_LEVEL` | `INFO` | Nível de detalhamento dos logs de observabilidade |

---

## Licença

Distribuído sob a licença MIT. Consulte o arquivo `LICENSE` para mais detalhes.
