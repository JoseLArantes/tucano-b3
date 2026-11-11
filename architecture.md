# Tucano B3 Architecture

`tucano-b3` is a high-performance REST API service designed to serve official market data from B3 (Brasil, Bolsa, Balcão), specifically historical quotes (COTAHIST) and IBOVESPA series.

## Architecture Overview

```
                                  +-------------------+
                                  |    HTTP Client    |
                                  +---------+---------+
                                            |
                                            v
                                  +-------------------+
                                  |   Axum Router     |
                                  +---------+---------+
                                            |
                         +------------------+------------------+
                         |           Middleware Pipeline       |
                         |  - MetricsLayer (Prometheus)        |
                         |  - RequestIdLayer (UUIDv4)          |
                         |  - RequestLoggingLayer (JSON/Trace) |
                         |  - RateLimitLayer (Token Bucket)    |
                         |  - CacheLayer (DashMap + LRU)       |
                         |  - CompressionLayer (Gzip/Brotli)   |
                         |  - CorsLayer                        |
                         |  - TimeoutLayer                     |
                         +------------------+------------------+
                                            |
                         +------------------+------------------+
                         |            REST Handlers            |
                         |  - /api/v1/health & /api/v1/info    |
                         |  - /api/v1/cotahist/...             |
                         |  - /api/v1/ibov/...                 |
                         |  - /metrics                         |
                         |  - /api/docs/ (Swagger UI)          |
                         +------------------+------------------+
                                            |
                         +------------------+------------------+
                         |          Domain Services            |
                         |  - CotahistService                  |
                         |  - IbovService                      |
                         +------------------+------------------+
                                            |
                         +------------------+------------------+
                         |            Data Layer               |
                         |  - DataLoader & Auto-reload         |
                         |  - QueryService (Apache DataFusion) |
                         |  - TickerRegistry (SQLite ledger)   |
                         |  - Parquet Files (/data/processed)  |
                         +-------------------------------------+
```

## Core Components

### 1. HTTP Layer (`src/server/`, `src/api/`)
- Powered by `axum` 0.7 and `tokio` 1.0.
- Serves typed JSON responses wrapped in standard API response envelopes (`ApiResponseEnvelope`).
- Exposes OpenAPI 3.0 schema and Swagger UI documentation via `utoipa`.

### 2. Middleware Pipeline (`src/middleware/`)
- **Metrics**: Records in-flight requests, request durations (histogram), and total requests with normalized path labels.
- **Request ID**: Guarantees each request has a unique `x-request-id` header.
- **Structured Logging**: Outputs structured JSON or pretty traces via `tracing`.
- **Rate Limiting**: Per-IP and per-API-key token bucket algorithm with header feedback (`x-ratelimit-*`).
- **Cache**: Fast in-memory cache using `DashMap` and LRU eviction, keyed deterministically via BLAKE3 hashes.

### 3. Query Engine (`src/data/`, `src/services/`)
- Uses **Apache DataFusion** to execute vectorized queries directly on partitioned Parquet datasets.
- Ticker metadata and ledger state are cached in memory from SQLite for sub-millisecond ticker lookups.
- Dynamic data reload monitors and refreshes Parquet datasets without downtime.

### 4. Cache System (`src/cache/`)
- High-concurrency `DashMap` cache with configurable TTL and LRU max capacity.
- Automatic expiration cleanup and cache invalidation on data reload.
- Comprehensive cache statistics: hits, misses, evictions, hit rate, and memory estimates.

### 5. Observability (`src/metrics/`)
- Prometheus metrics registry exposed at `/metrics` and `/api/v1/metrics`.
- Gauges and counters tracking data volume, loaded tickers, query durations, cache statistics, and rate limiting.

