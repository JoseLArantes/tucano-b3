# tucano-b3 — Performance Guide

## Performance Targets

| Metric | Target | Status |
|---|---|---|
| Throughput (cached) | > 10,000 req/s | ✓ |
| Latency p50 (cached) | < 5 ms | ✓ |
| Latency p99 (cached) | < 50 ms | ✓ |
| Latency p99 (cold) | < 100 ms | ✓ |
| Memory (steady state) | < 512 MB | ✓ |
| Startup time | < 2 seconds | ✓ |
| Data load (3 years) | < 5 seconds | ✓ |
| Binary size | < 30 MB | ✓ |

## Architecture Decisions for Performance

### 1. Embedded DataFusion
DataFusion is embedded in the binary. No external database server.
Queries execute in-process with zero network overhead.

### 2. Parquet Columnar Storage
Data is stored in Parquet format with zstd compression.
Columnar layout enables predicate pushdown and column projection.

### 3. In-Memory Cache
Hot queries are cached in a DashMap with TTL and LRU eviction.
Cache hits serve responses in < 1ms.

### 4. Streaming Response
Large result sets are streamed, not buffered entirely in memory.

### 5. Connection-Free Design
No database connections. Parquet files are memory-mapped or streamed.
Zero connection pool overhead.

## Benchmarking

### Run benchmarks
```bash
cargo bench
```

### Run load tests
```bash
./scripts/load_test.sh http://localhost:8080
./scripts/vegeta_test.sh http://localhost:8080
```

### Profile memory
```bash
./scripts/memory_profile.sh
```

## Optimization Techniques Applied

1. **DataFusion configuration tuning**: batch_size=8192, target_partitions=4
2. **Predicate pushdown**: date filters pushed into Parquet reader
3. **Column projection**: only needed columns selected
4. **LIMIT optimization**: latest-quote queries fetch only 2 rows
5. **Cache body size limit**: responses > 1 MB not cached
6. **Path normalization**: prevents metric label explosion
7. **Atomic counters**: metrics use lock-free atomic operations

## Profiling

### CPU profiling
```bash
cargo flamegraph --bin tucano-b3 -- --config config.toml
```

### Memory profiling
```bash
valgrind --tool=massif ./target/release/tucano-b3 --config config.toml
ms_print massif.out.<pid>
```

### Async profiling
```bash
tokio-console  # Requires tokio_unstable feature
```

## CI Performance Gate

Every PR runs performance checks:
- Binary size < 30 MB
- Startup time < 100 ms
- Benchmark results uploaded for comparison

## Tuning for Production

### Increase cache size
```toml
[cache]
max_entries = 50000
ttl_seconds = 600
```

### Increase worker threads
```toml
[server]
workers = 8  # Match CPU cores
```

### Behind a reverse proxy
Use nginx or caddy for TLS termination.
Set `X-Forwarded-For` header for accurate IP-based rate limiting.

