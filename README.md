# Tucano-B3

[![Rust](https://img.shields.io/badge/Rust-1.82%2B-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-1.x-000000?style=for-the-badge&logo=rust&logoColor=white)](https://tokio.rs/)
[![Axum](https://img.shields.io/badge/Axum-0.7-000000?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
[![Apache DataFusion](https://img.shields.io/badge/DataFusion-42.0-D22128?style=for-the-badge&logo=apache&logoColor=white)](https://datafusion.apache.org/)
[![Apache Parquet](https://img.shields.io/badge/Arrow%20%2F%20Parquet-53.0-D22128?style=for-the-badge&logo=apache&logoColor=white)](https://parquet.apache.org/)
[![SQLite](https://img.shields.io/badge/SQLite-0.32%20(Bundled)-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.0%20(Utoipa%204)-6BA539?style=for-the-badge&logo=openapiinitiative&logoColor=white)](https://www.openapis.org/)
[![Prometheus](https://img.shields.io/badge/Prometheus-0.13-E6522C?style=for-the-badge&logo=prometheus&logoColor=white)](https://prometheus.io/)
[![Docker](https://img.shields.io/badge/Docker-GHCR-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://github.com/tucano-b3/tucano-b3/pkgs/container/tucano-b3)

Motor de ingestão colunar e API REST de alta performance para séries históricas oficiais de cotações da **B3 (Brasil, Bolsa, Balcão)**.

---

## B3 e COTAHIST

A **B3 (Brasil, Bolsa, Balcão)** é a bolsa de valores oficial do Brasil, resultante da fusão entre a BM&FBOVESPA e a CETIP. Ela concentra a negociação de ações, fundos imobiliários (FIIs), ETFs, BDRs, derivativos e índices do mercado financeiro brasileiro.

Para disponibilizar o registro histórico oficial de todos os pregões, a B3 publica gratuitamente os arquivos **COTAHIST** (*Cotações Históricas*). Esses arquivos contêm o resumo diário de negociação de cada ativo listado (preços de abertura, mínimo, máximo, médio, fechamento, melhores ofertas de compra e venda, número de negócios, quantidade de títulos e volume financeiro), abrangendo séries anuais desde 1986 até os pregões diários atuais.

Embora os dados do COTAHIST sejam públicos e oficiais, consumi-los diretamente em aplicações modernas apresenta barreiras técnicas severas:

1. **Formato Posicional Legado (Mainframe):** Os arquivos são distribuídos em pacotes `.ZIP` contendo arquivos `.TXT` de largura fixa (**245 bytes por linha**), sem delimitadores (como vírgula ou tabulação) e codificados em **ISO-8859-1 (Latin-1)**.
2. **Casas Decimais Implícitas e Fator de Cotação (`FATCOT`):** Os preços não possuem ponto decimal explícito (ex.: `0000000003850` representa `38,50`) e dependem do fator de cotação do ativo para conversão correta.
3. **Volume Massivo de Dados:** Um único arquivo anual recente contém mais de **3,5 milhões de linhas** (~900 MB descompactado por ano). Ler arquivos `.TXT` sequencialmente a cada consulta é inviável para APIs em tempo real.
4. **Controle de Idempotência e Reprocessamento:** A B3 publica arquivos anuais (`COTAHIST_A`), mensais (`COTAHIST_M`) e diários (`COTAHIST_D`) que frequentemente se sobrepõem em datas. Sem um controle transacional de datas já processadas, a ingestão gera duplicatas e corrompe séries históricas.

O **Tucano-B3** resolve esse problema ponta a ponta em um **único binário escrito em Rust**:

- **Ingestão Streaming e Idempotente:** Descompacta arquivos `.ZIP` ou lê diretórios `.TXT` em streaming (buffer de 1 MB, sem estourar memória RAM), converte codificação Latin-1/UTF-8, aplica a escala decimal e verifica em um ledger **SQLite** quais pregões já foram processados.
- **Armazenamento Colunar (Apache Parquet + ZSTD):** Converte os registros brutos em arquivos **Parquet** particionados por ano com compressão **ZSTD**, reduzindo ~900 MB de texto posicional para ~60 MB em formato colunar tipado.
- **Motor Analítico Vetorizado (Apache DataFusion):** Executa consultas SQL analíticas diretamente sobre os arquivos Parquet usando **Apache Arrow**, entregando agregações e séries temporais em poucos milissegundos.
- **Cache em Memória de Sub-milissegundo:** Respostas frequentes são cacheadas em memória (`DashMap` + `LRU`) com chaves determinísticas **BLAKE3**.

---

## Estrutura e Layout do Arquivo COTAHIST

### Nomenclatura Oficial dos Arquivos

| Periodicidade | Padrão de Nome | Exemplo | Conteúdo |
| :--- | :--- | :--- | :--- |
| **Anual** | `COTAHIST_A<AAAA>.ZIP` | `COTAHIST_A2025.ZIP` | Todos os pregões do ano especificado |
| **Mensal** | `COTAHIST_M<MMAAAA>.ZIP` | `COTAHIST_M112026.ZIP` | Todos os pregões do mês especificado |
| **Diário** | `COTAHIST_D<DDMMAAAA>.ZIP` | `COTAHIST_D10112026.ZIP` | Pregão único da data especificada |

### Tipos de Registro (`TIPOREG`)

Cada linha possui exatamente **245 caracteres** (bytes) e começa com um identificador de 2 dígitos:

- **`00` (Header):** Cabeçalho do arquivo (nome do arquivo, código da origem `BOVESPA` e data de geração).
- **`01` (Cotação / Detalhe):** Registro de negociação de um ativo em uma data de pregão.
- **`02` (Trailer):** Rodapé de controle contendo o totalizador de linhas do arquivo.

### Mapa Posicional de Campos (Registro Tipo `01`)

A tabela abaixo detalha os campos do layout oficial da B3 (índices base-1 do manual da B3 e fatias base-0 utilizadas pelo parser em [`src/ingest/cotahist_parser.rs`](file:///Users/joseluiz/workspace/projects/tucano-b3/src/ingest/cotahist_parser.rs)):

| Posição (1-based) | Slice (0-based) | Campo B3 | Tamanho | Tipo Bruto | Campo no Tucano-B3 | Descrição |
| :---: | :---: | :--- | :---: | :---: | :--- | :--- |
| 01–02 | `0..2` | `TIPREG` | 2 | `N(02)` | — | Tipo de registro (`"01"` para cotações) |
| 03–10 | `2..10` | `DATAPREG` | 8 | `N(08)` | `data` (`YYYY-MM-DD`) | Data do pregão no formato `AAAAMMDD` |
| 11–12 | `10..12` | `CODBDI` | 2 | `X(02)` | — | Código BDI (lote padrão, fracionário, opções etc.) |
| 13–24 | `12..24` | `CODNEG` | 12 | `X(12)` | `ticker` | Código de negociação do ativo (ex.: `PETR4`, `VALE3`) |
| 25–27 | `24..27` | `TPMERC` | 3 | `N(03)` | `market_type` | Tipo de mercado (`010` Vista, `020` Fracionário, `070` Opções etc.) |
| 28–39 | `27..39` | `NOMRES` | 12 | `X(12)` | `company_name` | Nome resumido da empresa emissora |
| 40–49 | `39..49` | `ESPECI` | 10 | `X(10)` | — | Especificação do papel (ex.: `ON`, `PN`, `UNT`) |
| 50–52 | `49..52` | `PRAZOT` | 3 | `X(03)` | — | Prazo em dias do mercado a termo |
| 53–56 | `52..56` | `MODREF` / `FATCOT` | 4 | `X(04)` | *(escala decimal)* | Moeda de referência e fator de escala decimal |
| 57–69 | `56..69` | `PREABE` | 13 | `N(11)V99` | `pre_abr` | Preço de abertura do papel no pregão |
| 70–82 | `69..82` | `PREMAX` | 13 | `N(11)V99` | `pre_max` | Preço máximo do papel no pregão |
| 83–95 | `82..95` | `PREMIN` | 13 | `N(11)V99` | `pre_min` | Preço mínimo do papel no pregão |
| 96–108 | `95..108` | `PREMED` | 13 | `N(11)V99` | `pre_med` | Preço médio do papel no pregão |
| 109–121 | `108..121` | `PREULT` | 13 | `N(11)V99` | `pre_ult` | Preço de fechamento (último negócio) do pregão |
| 122–134 | `121..134` | `PREOFC` | 13 | `N(11)V99` | `pre_ofe_compra` | Preço da melhor oferta de compra |
| 135–147 | `134..147` | `PREOFV` | 13 | `N(11)V99` | `pre_ofe_venda` | Preço da melhor oferta de venda |
| 148–152 | `147..152` | `TOTNEG` | 5 | `N(05)` | `tot_neg` | Número de negócios realizados no pregão |
| 153–170 | `152..170` | `QUATOT` | 18 | `N(18)` | `quat_tot` | Quantidade total de títulos negociados |
| 171–188 | `170..188` | `VOLTOT` | 18 | `N(16)V99` | `vol_tot` | Volume financeiro total negociado |
| 189–201 | `188..201` | `PREEXE` | 13 | `N(11)V99` | — | Preço de exercício para opções / termo |
| 202–202 | `201..202` | `INDOPC` | 1 | `N(01)` | — | Indicador de correção de preços de exercício |
| 203–210 | `202..210` | `DATVEN` | 8 | `N(08)` | — | Data de vencimento para opções / termo |
| 211–217 | `210..217` | `FATCOT` | 7 | `N(07)` | — | Fator de cotação do título (`1`, `100`, `1000`) |
| 218–230 | `217..230` | `PTOEXE` | 13 | `N(07)V06` | — | Preço de exercício em pontos |
| 231–242 | `191..203`* | `CODISI` | 12 | `X(12)` | `isin` | Código ISIN (*International Securities Identification Number*) |
| 243–245 | `242..245` | `DISMES` | 3 | `N(03)` | — | Número de distribuição do papel |

> **⚠️ Aviso Importante sobre Preços Nominais:**
> Os dados do arquivo COTAHIST publicados pela B3 representam **preços históricos brutos nominais** da época em que cada pregão ocorreu. Eles **não possuem ajuste retroativo** para eventos corporativos subsequentes, tais como grupamentos (*inplits*), desdobramentos (*splits*), bonificações ou distribuição de dividendos e JCP.

---

## Arquitetura de Integração

O Tucano-B3 foi desenhado sob a filosofia *Zero External Dependencies* em tempo de execução, permitindo que a mesma base de código escale desde o notebook de um analista até ambientes corporativos de missão crítica:

### 🟢 Pequeno Porte (Pesquisa Quantitativa, Scripts Pessoais e Protótipos)
- **Implantação em Arquivo Único:** Roda como um único executável nativo (`tucano-b3`) sem exigir PostgreSQL, Redis, Kafka ou motores pesados.
- **Baixíssimo Consumo de Recursos:** Utiliza SQLite embarcado para o registro de tickers e arquivos Parquet locais, operando confortavelmente com menos de **128 MB de RAM**.
- **Ideal para:** Notebooks Python/R/Julia (via requisições HTTP simples), backtesting local de estratégias, carteiras pessoais e automações domésticas.

### 🟡 Médio Porte (Fintechs, Portais Financeiros e Dashboards Analíticos)
- **Endpoints Especializados para Gráficos:** Oferece rotas como `/series` (que retorna apenas `date_ms`, `pre_ult`, `vol_tot` e `tot_neg` ordenados cronologicamente sem overhead de paginação) e `/pre-ult/min` e `/pre-ult/max` para alimentar gráficos *candlestick* ou *line charts* (TradingView, ECharts, Recharts) instantaneamente.
- **Segurança e Controle de Tráfego Nativos:** Possui autenticação via **Bearer Token** / **API Key** e *Rate Limiting* (*Token Bucket*) configurável por IP e por chave de API.
- **Cache Concorrente:** O cache `DashMap` + `LRU` absorve picos de leitura em ativos populares (`PETR4`, `VALE3`, `ITUB4`) respondendo em frações de milissegundo.

### 🔴 Grande Porte (Plataformas Corporativas, Corretoras e Clusters Cloud)
- **Escalabilidade Horizontal Stateless:** Como os dados processados residem em arquivos Parquet imutáveis por partição anual e o ledger é atualizado apenas na ingestão, múltiplas réplicas de leitura do Tucano-B3 podem ser escaladas atrás de um Load Balancer (Kubernetes / AWS ECS) lendo de um volume compartilhado (EFS / Ceph / S3 mount).
- **Observabilidade Nativa:** Exporta métricas detalhadas no padrão **Prometheus** (`/metrics`), incluindo histogramas de latência por rota normalizada, taxa de acerto de cache (*hit rate*), contagem de registros carregados e rejeições de rate limit.
- **Rastreabilidade Distribuída:** Logs estruturados em JSON com propagação e geração automática do cabeçalho `x-request-id` (UUID v4) em todas as respostas e envelopes de erro.

---

## Instalação, Configuração e Execução

### 5.1. Pré-requisitos

- **Para execução via binário ou código-fonte:** Rust `1.82+` e Cargo.
- **Para execução via container:** Docker `24+` e Docker Compose `v2+`.

### 5.2. Configuração do Ambiente (`.env` e `config.toml`)

1. **Configure o Token de Autenticação no arquivo `.env`** na raiz do projeto:

```dotenv
# Token principal exigido em todas as rotas protegidas
TUCANO_B3_TOKEN=seu_token_secreto_aqui

# Opcional: Tokens adicionais permitidos (leitura / administração)
TUCANO_B3_READ_TOKEN=token_secundario_leitura
TUCANO_B3_ADMIN_TOKEN=token_secundario_admin
```

2. **Revise o arquivo `config.toml`** (valores padrão já otimizados):

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 0 # 0 = detecta automaticamente os núcleos de CPU
request_timeout_seconds = 30

[data]
dir = "./data"
reload_interval_seconds = 0

[cache]
enabled = true
ttl_seconds = 300
max_entries = 10000

[rate_limit]
enabled = true
default_requests_per_minute = 100
api_key_requests_per_minute = 1000

[logging]
level = "info"
format = "json" # "json" ou "pretty"

[metrics]
enabled = true
```

> **Sobrescrita via Variáveis de Ambiente:** Qualquer chave do `config.toml` pode ser sobrescrita via variáveis de ambiente: `TUCANO_SERVER_HOST`, `TUCANO_SERVER_PORT`, `TUCANO_SERVER_WORKERS`, `TUCANO_DATA_DIR`, `TUCANO_CACHE_ENABLED`, `TUCANO_CACHE_TTL`, `TUCANO_CACHE_MAX_ENTRIES`, `TUCANO_LOG_LEVEL` e `TUCANO_LOG_FORMAT`.

### Compilação e Execução Nativa

```bash
# 1. Compilar o binário otimizado de produção (Thin LTO)
cargo build --release

# 2. Garantir que o diretório de dados existe
mkdir -p data

# 3. Iniciar o servidor
./target/release/tucano-b3 --config config.toml
```

### Execução via Docker e Docker Compose

**Opção A — Imagem Oficial via Docker Compose:**
```bash
docker compose up -d api
```

**Opção B — Empacotamento Instantâneo do Binário Local (`Dockerfile.local`):**
Se você já compilou `./target/release/tucano-b3` na máquina host (Linux), pode gerar a imagem em 2 segundos:
```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml up -d --build api
```

**Opção C — Build Completo Multi-Stage via Docker:**
```bash
docker build -t tucano-b3:latest .
docker run -d \
  --name tucano_b3_api \
  -p 8080:8080 \
  --env-file .env \
  -v $(pwd)/data:/data \
  tucano-b3:latest
```

---

## Ingestão de Dados

O Tucano-B3 possui dois métodos integrados via API para ingerir arquivos `COTAHIST` da B3 (tanto arquivos `.ZIP` quanto `.TXT` descompactados).

### Ingestão em Lote a partir de uma Pasta Local (`POST /api/v1/ingest/folder`)

Coloque seus arquivos oficiais da B3 (ex.: `COTAHIST_A2025.ZIP`, `COTAHIST_A2026.ZIP`) dentro da pasta `./data` e execute:

```bash
# Se o servidor estiver rodando nativamente no host:
curl -X POST "http://localhost:8080/api/v1/ingest/folder" \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"caminho": "./data"}'

# Se o servidor estiver rodando dentro do container Docker (volume montado em /data):
curl -X POST "http://localhost:8080/api/v1/ingest/folder" \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"caminho": "/data"}'
```

**Exemplo de Resposta:**
```json
{
  "data": {
    "task_id": "2c84ef59-97a7-46c7-b6cf-51b2a7d9d102",
    "files_processed": 6,
    "total_records_parsed": 7384908,
    "total_records_new": 6588617,
    "total_records_skipped": 796291,
    "success": true,
    "message": "Ingestão concluída: 6 arquivos processados, 6588617 novos registros."
  },
  "meta": {
    "request_id": "7a82d1b2-3e4f-4a12-9c01-88a72109bcde",
    "timestamp": "2026-11-12T12:55:10.412Z",
    "cached": false,
    "query_time_ms": 14520.31
  }
}
```

### Ingestão via Upload de Arquivo (`POST /api/v1/ingest/upload`)

Você também pode enviar um arquivo `.ZIP` diretamente via HTTP `multipart/form-data` (pelo terminal ou pelo botão **Upload** no Swagger UI):

```bash
curl -X POST "http://localhost:8080/api/v1/ingest/upload" \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -F "file=@./data/COTAHIST_A2025.ZIP"
```

### Idempotência na Ingestão

- Durante a ingestão, o serviço consulta a tabela `ingested_dates` no SQLite (`data/ledger/tucano.db`).
- Qualquer registro cuja data de pregão (`YYYY-MM-DD`) já tenha sido ingerida anteriormente é automaticamente contabilizado em `total_records_skipped` e ignorado.
- Apenas pregões inéditos são gravados nos arquivos Parquet (`data/processed/cotahist/cotahist-<ANO>.parquet`).
- Ao final da gravação, o `DataLoader` recarrega a tabela do DataFusion a quente (*hot reload*) e limpa o cache em memória sem interromper as requisições em andamento.

---

## Endpoints e Exemplos de Uso

### Autenticação

Todas as rotas da API — **exceto** `/health`, `/api/v1/health`, `/api/docs/*` e `/redoc` — exigem o envio do token configurado em `TUCANO_B3_TOKEN` através de um dos cabeçalhos abaixo:

- **Bearer Token (Recomendado):** `Authorization: Bearer <TUCANO_B3_TOKEN>`
- **API Key Header:** `X-API-Key: <TUCANO_B3_TOKEN>`

### Tabela de Endpoints

| Método | Rota | Autenticação | Descrição |
| :---: | :--- | :---: | :--- |
| `GET` | `/health` ou `/api/v1/health` | Pública | Status de saúde do serviço, arquivos carregados e total de tickers |
| `GET` | `/api/v1/info` | Protegida | Informações detalhadas sobre o dataset, anos cobertos e cache |
| `GET` | `/api/v1/cotahist/tickers` | Protegida | Lista paginada de tickers com busca por prefixo (`busca`) e filtro `ativo` |
| `GET` | `/api/v1/cotahist/tickers/{ticker}` | Protegida | Metadados de um ticker, primeira/última data vista e último preço |
| `GET` | `/api/v1/cotahist/tickers/{ticker}/quotes` | Protegida | Histórico completo paginado de cotações por período ou últimos `dias` |
| `GET` | `/api/v1/cotahist/tickers/{ticker}/latest` | Protegida | Cotação do pregão mais recente do ticker com variação percentual |
| `GET` | `/api/v1/cotahist/tickers/{ticker}/series` | Protegida | Série enxuta (`date_ms`, `pre_ult`, `vol_tot`, `tot_neg`) para gráficos |
| `GET` | `/api/v1/cotahist/tickers/{ticker}/pre-ult/min` | Protegida | Menor preço de fechamento (`pre_ult`) do ticker no período |
| `GET` | `/api/v1/cotahist/tickers/{ticker}/pre-ult/max` | Protegida | Maior preço de fechamento (`pre_ult`) do ticker no período |
| `GET` | `/api/v1/cotahist/dates/{date}/quotes` | Protegida | Todas as cotações de todos os ativos negociados em uma data (`AAAA-MM-DD`) |
| `GET` | `/api/v1/ibov/history` | Protegida | Série histórica paginada do índice IBOVESPA |
| `GET` | `/api/v1/ibov/latest` | Protegida | Ponto mais recente disponível do índice IBOVESPA |
| `POST` | `/api/v1/ingest/upload` | Protegida | Upload e ingestão incremental de arquivo `.ZIP` do COTAHIST |
| `POST` | `/api/v1/ingest/folder` | Protegida | Varredura e ingestão incremental de pasta local no servidor |
| `GET` | `/metrics` ou `/api/v1/metrics` | Protegida | Métricas no formato texto do Prometheus |

### Exemplos Práticos com `curl`

```bash
export TOKEN="TUCAN0K9hf*74hJ948aaFagndjehH74h3jhheg39983jfhnnhj5hkjHg"

# 1. Health Check (Público — não requer token)
curl -s http://localhost:8080/health | jq

# 2. Buscar tickers que começam com "PETR"
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers?busca=PETR&por_pagina=5" | jq

# 3. Obter detalhes e resumo do ativo PETR4
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/PETR4" | jq

# 4. Consultar a última cotação disponível de VALE3
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/VALE3/latest" | jq

# 5. Consultar os últimos 5 pregões de PETR4
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/PETR4/quotes?dias=5" | jq

# 6. Obter série compacta para renderização de gráfico em 2025
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/PETR4/series?desde=2025-01-01&ate=2025-12-31" | jq

# 7. Obter mínima e máxima de fechamento no período
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/PETR4/pre-ult/min?desde=2025-01-01&ate=2025-12-31" | jq
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/cotahist/tickers/PETR4/pre-ult/max?desde=2025-01-01&ate=2025-12-31" | jq
```

---

## Swagger UI e ReDoc

Com o servidor em execução, a especificação **OpenAPI 3.0** e duas interfaces visuais estão disponíveis publicamente:

- **Swagger UI (Playground Interativo):** [http://localhost:8080/api/docs/](http://localhost:8080/api/docs/)
  - Clique no botão **Authorize** no topo da página, cole seu `TUCANO_B3_TOKEN` em `BearerAuth` (ou `ApiKeyAuth`) e clique em **Authorize**. A sessão ficará salva automaticamente no navegador (`persistAuthorization`).
- **ReDoc (Documentação de Referência):** [http://localhost:8080/redoc](http://localhost:8080/redoc) (ou `/api/redoc`)
- **Especificação OpenAPI JSON Bruta:** [http://localhost:8080/api/docs/openapi.json](http://localhost:8080/api/docs/openapi.json)

---

## Para Desenvolvedores

Para detalhes profundos sobre o desenho interno dos componentes, consulte o documento [**ARCHITECTURE.md**](file:///Users/joseluiz/workspace/projects/tucano-b3/ARCHITECTURE.md).

### Comandos de Validação e Qualidade

```bash
# Verificar formatação do código
cargo fmt --all -- --check

# Executar análise estática rigorosa (Clippy sem warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Executar toda a suíte de testes (unitários, integração, API, cache, ingestão e OpenAPI)
cargo test --all-targets --all-features

# Executar benchmarks de performance (Criterion)
cargo bench
```

---

## Licença

Distribuído sob dupla licença **MIT**.
