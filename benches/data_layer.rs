//! Performance benchmarks for the data layer.

use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use tucano_b3::config::DataConfig;
use tucano_b3::data::reload::DataLoader;

fn bench_data_loader_startup(c: &mut Criterion) {
    let fixture_dir = PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        eprintln!("Skipping benchmark: fixtures not found");
        return;
    }

    let config = DataConfig {
        dir: fixture_dir,
        reload_interval_seconds: 0,
    };

    c.bench_function("data_loader_startup", |b| {
        b.iter(|| {
            let _loader = DataLoader::new_standalone(&config);
        });
    });
}

fn bench_cotahist_query(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let fixture_dir = PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        eprintln!("Skipping benchmark: fixtures not found");
        return;
    }

    let config = DataConfig {
        dir: fixture_dir,
        reload_interval_seconds: 0,
    };

    let loader = DataLoader::new_standalone(&config).unwrap();

    c.bench_function("cotahist_ticker_query", |b| {
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
}

criterion_group!(benches, bench_data_loader_startup, bench_cotahist_query);
criterion_main!(benches);
