# Tucano B3 API

[![API](https://img.shields.io/badge/API-Production-0A66C2)](https://b3.tucano.beakcloud.com)
[![Python](https://img.shields.io/badge/Python-3.11-3776AB?logo=python&logoColor=white)](https://www.python.org/)
[![FastAPI](https://img.shields.io/badge/FastAPI-0.115.0-009688?logo=fastapi&logoColor=white)](https://fastapi.tiangolo.com/)
[![SQLAlchemy](https://img.shields.io/badge/SQLAlchemy-2.0.35-D71F00?logo=sqlalchemy&logoColor=white)](https://www.sqlalchemy.org/)
[![Alembic](https://img.shields.io/badge/Alembic-1.13.2-4B5563)](https://alembic.sqlalchemy.org/)
[![Celery](https://img.shields.io/badge/Celery-5.4.0-37814A?logo=celery&logoColor=white)](https://docs.celeryq.dev/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-336791?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Redis](https://img.shields.io/badge/Redis-7-DC382D?logo=redis&logoColor=white)](https://redis.io/)
[![Docker Compose](https://img.shields.io/badge/Docker%20Compose-v2-2496ED?logo=docker&logoColor=white)](https://docs.docker.com/compose/)

API de normalização de dados públicos da B3 com escopo **B3-only**, endpoints por família de arquivo oficial e pipeline de sincronização com detecção de mudanças por campo.

## Production

- URL base de produção: [https://b3.tucano.beakcloud.com](https://b3.tucano.beakcloud.com)

## Índice

- [Objetivo](#objetivo)
- [Escopo de dados](#escopo-de-dados)
- [Arquitetura](#arquitetura)
- [Tecnologias e versões](#tecnologias-e-versões)
- [Autenticação](#autenticação)
- [Documentação da API](#documentação-da-api)
- [Endpoints](#endpoints)
- [Modelo de dados](#modelo-de-dados)
- [Pipeline de sincronização](#pipeline-de-sincronização)
- [Executando localmente](#executando-localmente)
- [Docker Compose](#docker-compose)
- [Migrações](#migrações)
- [Testes](#testes)
- [Operação e observabilidade](#operação-e-observabilidade)
- [Variáveis de ambiente](#variáveis-de-ambiente)
- [Estrutura do projeto](#estrutura-do-projeto)
- [Checklist de conformidade com PRD](#checklist-de-conformidade-com-prd)
- [Licença](#licença)

## Objetivo

Disponibilizar dados oficiais públicos da B3 por API REST, preservando:

- proveniência por arquivo de origem;
- contratos de dados por família de arquivo;
- sincronização idempotente;
- atualização apenas de campos alterados;
- separação explícita entre fontes (sem merge implícito entre `BVBG.086.01` e `BVBG.186.01`).

## Escopo de dados

Fontes suportadas:

- `BVBG.028.02` (cadastro de instrumentos)
- `Tradable Security List`
- `BVBG.086.01` (PriceReport)
- `BVBG.186.01` (Simplified Equities PriceReport)
- `BVBG.087.01` (IndexReport)
- Composição de índices B3
- `COTAHIST.AAAA.TXT`

Não há integração com fontes externas como Yahoo, CVM, corretoras, portais ou APIs pagas.

## Arquitetura

Componentes:

- **API FastAPI**: exposição de endpoints e OpenAPI.
- **Worker Celery**: execução de sincronizações manuais e agendadas.
- **Scheduler (Celery Beat embutido no worker)**: agendamento diário.
- **PostgreSQL**: persistência relacional.
- **Redis**: broker/result backend do Celery.
- **Alembic**: migrações de schema.

Fluxo simplificado:

1. Download validado por host permitido.
2. Parse por tipo de fonte.
3. Resolução de relacionamento para instrumentos/índices.
4. Upsert changed-field-only.
5. Atualização de metadados de arquivo e execução.

## Tecnologias e versões

- Python `3.11`
- FastAPI `0.115.0`
- SQLAlchemy `2.0.35`
- Alembic `1.13.2`
- Celery `5.4.0`
- PostgreSQL `16`
- Redis `7`
- Uvicorn `0.30.6`

## Autenticação

Todos os endpoints de API exigem token Bearer.

- Variável: `TUCANO_B3_TOKEN`
- Header:

```http
Authorization: Bearer <TUCANO_B3_TOKEN>
```

Sem token ou com token inválido retorna `401`.

## Documentação da API

- Swagger UI: `/docs`
- ReDoc: `/redoc`
- OpenAPI JSON: `/openapi.json`

Exemplos (local):

- [http://localhost:8001/docs](http://localhost:8001/docs)
- [http://localhost:8001/redoc](http://localhost:8001/redoc)

## Endpoints

Base local: `http://localhost:8001`

Base produção: `https://b3.tucano.beakcloud.com`

### Infra

- `GET /api/v1/health`
- `GET /api/v1/b3/worker/health`

### Instrumentos (`BVBG.028.02`)

- `GET /api/v1/b3/instruments`
- `GET /api/v1/b3/instruments/{instrument_id}`
- `GET /api/v1/b3/instruments/by-security-id/{security_id}`
- `GET /api/v1/b3/instruments/by-ticker/{ticker_symbol}`
- `GET /api/v1/b3/instruments/by-isin/{isin}`
- `GET /api/v1/b3/instruments/{instrument_id}/identifiers`

### Tradable Security List

- `GET /api/v1/b3/tradable-securities`
- `GET /api/v1/b3/tradable-securities/by-date/{reference_date}`
- `GET /api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}`

Observação: por padrão, o endpoint público retorna apenas registros resolvidos. Para incluir pendências, use `include_unresolved=true`.

### PriceReport (`BVBG.086.01`)

- `GET /api/v1/b3/price-report/quotes`
- `GET /api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}`
- `GET /api/v1/b3/price-report/quotes/by-date/{trade_date}`
- `GET /api/v1/b3/price-report/quotes/{quote_id}`

### Simplified Equities PriceReport (`BVBG.186.01`)

- `GET /api/v1/b3/equities-simplified-price-report/quotes`
- `GET /api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}`
- `GET /api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}`
- `GET /api/v1/b3/equities-simplified-price-report/quotes/{quote_id}`

### IndexReport (`BVBG.087.01`)

- `GET /api/v1/b3/index-report/indices`
- `GET /api/v1/b3/index-report/values`
- `GET /api/v1/b3/index-report/values/by-index/{index_code}`
- `GET /api/v1/b3/index-report/values/by-date/{trade_date}`

### Composição de índices

- `GET /api/v1/b3/index-composition/portfolios`
- `GET /api/v1/b3/index-composition/portfolios/{index_code}`
- `GET /api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}`
- `GET /api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}`

### COTAHIST

- `GET /api/v1/b3/cotahist/quotes`
- `GET /api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}`
- `GET /api/v1/b3/cotahist/quotes/by-isin/{isin}`
- `GET /api/v1/b3/cotahist/quotes/by-date/{trade_date}`
- `GET /api/v1/b3/cotahist/quotes/{quote_id}`

### Metadados de sincronização

- `GET /api/v1/b3/source-files`
- `GET /api/v1/b3/source-files/{source_file_id}`
- `GET /api/v1/b3/sync-runs`
- `GET /api/v1/b3/sync-runs/{sync_run_id}`
- `POST /api/v1/b3/sync-runs`

## Modelo de dados

Tabelas principais:

- `b3_source_files`
- `b3_sync_runs`
- `b3_instruments`
- `b3_instrument_identifiers`
- `b3_tradable_securities`
- `b3_tradable_securities_staging`
- `b3_price_report_quotes`
- `b3_equities_simplified_price_quotes`
- `b3_indices`
- `b3_index_report_values`
- `b3_index_portfolio_compositions`
- `b3_cotahist_quotes`
- `b3_field_changes`

Contratos de mudança interna:

- `source_row_hash`
- `data_changed_at`
- `field_changed_at`

## Pipeline de sincronização

### Regras implementadas

- Ordem de ingestão protegida: primeiro `BVBG.028.02`.
- Detecção de duplicidade por `source_file_type + reference_date + sha256`.
- Lock por fonte/data usando advisory lock no PostgreSQL.
- Retry no Celery para erros transitórios.
- Resolução por `security_id` e fallbacks de identificadores.
- Linhas unresolved de tradable armazenadas em staging.
- `force_reparse` suportado no POST de sync manual.

### Agendamento

Task diária:

- `sync.scheduled.daily` (Celery Beat)

A task calcula a data no momento da execução (timezone configurada em `B3_DEFAULT_TIMEZONE`) e dispara sincronizações para as fontes com URL configurada via ambiente.

## Executando localmente

Pré-requisitos:

- Docker e Docker Compose v2

Passos:

1. Ajuste `.env` (pode partir de `.env.example`).
2. Suba stack:
   - `docker compose up --build -d`
3. Acesse docs:
   - `http://localhost:8001/docs`
   - `http://localhost:8001/redoc`

Exemplo de healthcheck com auth:

```bash
curl -H "Authorization: Bearer dev-token" http://localhost:8001/api/v1/health
```

## Docker Compose

Serviços:

- `api`
- `worker`
- `postgres`
- `redis`

Volumes:

- `postgres_data`
- `b3_raw_files`
- `b3_processed_files`

## Migrações

As migrações são aplicadas automaticamente na inicialização do `b3_api`:

```bash
alembic upgrade head && uvicorn app.main:app --host 0.0.0.0 --port 8000
```

Execução manual (opcional):

```bash
alembic upgrade head
```

## Testes

Local:

```bash
pytest -q
```

Container:

```bash
docker compose run --rm api pytest -q
```

Cobertura atual inclui:

- parsers (incluindo COTAHIST)
- canonicalização
- resolução de relacionamento
- upsert por campo alterado
- contratos de API/OpenAPI
- autenticação por token
- sync service (duplicidade, lock, status)

## Operação e observabilidade

Metadados operacionais:

- `b3_source_files` guarda hash, status, contagens e erros.
- `b3_sync_runs` guarda status, range de datas, estatísticas e warnings.

Endpoints de consulta:

- `/api/v1/b3/source-files`
- `/api/v1/b3/sync-runs`

## Variáveis de ambiente

Obrigatórias/mais relevantes:

- `DATABASE_URL`
- `REDIS_URL`
- `TUCANO_B3_TOKEN`
- `B3_ALLOWED_HOSTS`
- `B3_DEFAULT_TIMEZONE`
- `B3_SYNC_ENABLED`
- `B3_MANUAL_SYNC_ENABLED`
- `B3_PARSER_VERSION`

URLs por fonte para agendamento:

- `B3_SOURCE_URL_BVBG_028`
- `B3_SOURCE_URL_TRADABLE_SECURITY_LIST`
- `B3_SOURCE_URL_BVBG_086`
- `B3_SOURCE_URL_BVBG_186`
- `B3_SOURCE_URL_BVBG_087`
- `B3_SOURCE_URL_INDEX_COMPOSITION`
- `B3_SOURCE_URL_COTAHIST`

## Estrutura do projeto

```text
app/
  api/
    routers/
    schemas.py
  core/
    auth.py
    config.py
  db/
    models.py
    session.py
  parsers/
  services/
    sync_service.py
    download_service.py
    resolver.py
    upsert.py
  worker/
    celery_app.py
alembic/
docs/
tests/
```

## Checklist de conformidade com PRD

- Endpoints por família de fonte B3.
- Proveniência por linha de resposta.
- COTAHIST documentado como dado bruto não ajustado.
- `BVBG.086.01` e `BVBG.186.01` separados.
- Sync com deduplicação por hash.
- Lock distribuído (PostgreSQL advisory lock).
- Testes unitários e OpenAPI.
- Stack Docker Compose completa.

## Licença

Uso interno Tucano/BeakCloud, salvo definição diferente no repositório principal.
