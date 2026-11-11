//! API performance benchmarks.
//!
//! Measures throughput and latency for all endpoint types.
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Benchmark: QueryService SQL execution (cold path).
fn bench_query_service_sql(c: &mut Criterion) {
    let fixture_dir = std::path::PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        eprintln!("Skipping: fixtures not found");
        return;
    }

    let config = tucano_b3::config::DataConfig {
        dir: fixture_dir,
        reload_interval_seconds: 0,
    };

    let cache = Arc::new(tucano_b3::cache::CacheStore::new(
        &tucano_b3::config::CacheConfig {
            enabled: false,
            ttl_seconds: 0,
            max_entries: 0,
        },
    ));

    let loader = match tucano_b3::data::reload::DataLoader::new(&config, cache) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Skipping: failed to load data: {}", e);
            return;
        }
    };

    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("query_service");
    group.throughput(Throughput::Elements(1));

    // Benchmark: simple ticker query
    group.bench_function("ticker_latest", |b| {
        b.iter(|| {
            rt.block_on(async {
                let service = loader.get_query_service().await;
                if service.has_table("cotahist") {
                    let _ = service
                        .execute_sql(
                            "SELECT * FROM cotahist WHERE ticker = 'VALE3' ORDER BY date DESC LIMIT 1",
                        )
                        .await;
                }
            });
        });
    });

    // Benchmark: aggregation query
    group.bench_function("ticker_aggregation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let service = loader.get_query_service().await;
                if service.has_table("cotahist") {
                    let _ = service
                        .execute_sql(
                            "SELECT ticker, AVG(pre_ult), SUM(vol_tot) FROM cotahist GROUP BY ticker",
                        )
                        .await;
                }
            });
        });
    });

    // Benchmark: date range query
    group.bench_function("date_range_query", |b| {
        b.iter(|| {
            rt.block_on(async {
                let service = loader.get_query_service().await;
                if service.has_table("cotahist") {
                    let _ = service
                        .execute_sql(
                            "SELECT * FROM cotahist WHERE date >= '2026-01-01' AND date <= '2026-01-31' ORDER BY date",
                        )
                        .await;
                }
            });
        });
    });

    group.finish();
}

/// Benchmark: JSON serialization.
fn bench_serialization(c: &mut Criterion) {
    let quote = tucano_b3::models::cotahist::QuoteResponse {
        ticker: "VALE3".to_string(),
        data: "2026-11-07".to_string(),
        pre_abr: Some(62.05),
        pre_max: Some(62.89),
        pre_min: Some(61.72),
        pre_med: Some(62.31),
        pre_ult: Some(62.17),
        pre_ofe_compra: Some(62.15),
        pre_ofe_venda: Some(62.19),
        tot_neg: Some(245891),
        quat_tot: Some(29847362),
        vol_tot: Some(1847293440.0),
        var_pct: Some(0.45),
        var_abs: Some(0.28),
    };

    let mut group = c.benchmark_group("serialization");

    group.bench_function("single_quote_json", |b| {
        b.iter(|| serde_json::to_string(&quote));
    });

    group.bench_function("envelope_json", |b| {
        b.iter(|| {
            let envelope = tucano_b3::models::envelope::ApiResponse::new(vec![quote.clone()], 0.42);
            serde_json::to_string(&envelope)
        });
    });

    group.finish();
}

/// Benchmark: Cache operations.
fn bench_cache(c: &mut Criterion) {
    let config = tucano_b3::config::CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 10_000,
    };
    let store = tucano_b3::cache::CacheStore::new(&config);

    // Pre-populate
    for i in 0..1000 {
        store.insert(
            format!("key_{}", i),
            b"cached_response_body".to_vec(),
            200,
            "application/json".to_string(),
        );
    }

    let mut group = c.benchmark_group("cache");

    group.bench_function("cache_hit", |b| {
        b.iter(|| store.get("key_500"));
    });

    group.bench_function("cache_miss", |b| {
        b.iter(|| store.get("nonexistent_key"));
    });

    let counter = AtomicU32::new(1000);
    group.bench_function("cache_insert", |b| {
        b.iter(|| {
            let id = counter.fetch_add(1, Ordering::Relaxed);
            store.insert(
                format!("key_{}", id),
                b"new_data".to_vec(),
                200,
                "application/json".to_string(),
            );
        });
    });

    group.finish();
}

/// Benchmark: DataLoader startup.
fn bench_data_loader_startup(c: &mut Criterion) {
    let fixture_dir = std::path::PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        eprintln!("Skipping: fixtures not found");
        return;
    }

    let config = tucano_b3::config::DataConfig {
        dir: fixture_dir,
        reload_interval_seconds: 0,
    };

    let cache = Arc::new(tucano_b3::cache::CacheStore::new(
        &tucano_b3::config::CacheConfig {
            enabled: false,
            ttl_seconds: 0,
            max_entries: 0,
        },
    ));

    c.bench_function("data_loader_startup", |b| {
        b.iter(|| {
            let _ = tucano_b3::data::reload::DataLoader::new(&config, cache.clone());
        });
    });
}

criterion_group!(
    benches,
    bench_query_service_sql,
    bench_serialization,
    bench_cache,
    bench_data_loader_startup,
);
criterion_main!(benches);
