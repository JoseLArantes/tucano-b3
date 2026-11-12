//! End-to-end ingestion test.

use std::io::Write;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Create a minimal COTAHIST ZIP for testing.
fn create_test_cotahist_zip() -> Vec<u8> {
    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default();

        // Create a minimal COTAHIST file (exact byte width and layout)
        let header = format!("00COTAHIST.2026{}", " ".repeat(230));
        let mut line = String::new();
        line.push_str("01"); // TIPOREG (0..2)
        line.push_str("20260102"); // DATA_PREGAO (2..10)
        line.push_str("  "); // CODBDI (10..12)
        line.push_str("VALE3       "); // CODNEG (12..24)
        line.push_str("010"); // TPMERC (24..27)
        line.push_str("VALE S.A.   "); // NOMRES (27..39)
        line.push_str("ON        "); // ESPECI (39..49)
        line.push_str("   "); // PRAZOT (49..52)
        line.push_str(" 2  "); // FAT_COTACAO (52..56)
        line.push_str("0000000006205"); // PREABR (56..69)
        line.push_str("0000000006289"); // PREMAX (69..82)
        line.push_str("0000000006172"); // PREMIN (82..95)
        line.push_str("0000000006231"); // PREMED (95..108)
        line.push_str("0000000006217"); // PREULT (108..121)
        line.push_str("0000000006215"); // PREOFC (121..134)
        line.push_str("0000000006219"); // PREOFV (134..147)
        line.push_str("00245"); // TOTNEG (147..152)
        line.push_str("000000000029847362"); // QUATOT (152..170)
        line.push_str("000000184729344000"); // VOLTOT (170..188)
        line.push_str("   "); // FILLER (188..191)
        line.push_str("BRVALEACNPA3"); // CODISIN (191..203)
        while line.len() < 249 {
            line.push(' ');
        }
        let trailer = format!("02{}", " ".repeat(245));

        let content = format!("{}\n{}\n{}\n", header, line, trailer);

        zip.start_file("COTAHIST_D2026.TXT", options).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    zip_buffer
}

#[tokio::test]
async fn test_e2e_zip_ingestion() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().to_path_buf();

    // Create data directory structure
    std::fs::create_dir_all(data_dir.join("processed/cotahist")).unwrap();
    std::fs::create_dir_all(data_dir.join("ledger")).unwrap();

    // Create test ZIP
    let zip_data = create_test_cotahist_zip();

    // Run ingestion directly (bypass HTTP for unit test)
    let cache = std::sync::Arc::new(tucano_b3::cache::store::CacheStore::new(
        &tucano_b3::config::CacheConfig {
            enabled: false,
            ttl_seconds: 0,
            max_entries: 0,
        },
    ));

    let config = tucano_b3::config::DataConfig {
        dir: data_dir.clone(),
        reload_interval_seconds: 0,
    };

    let data_loader =
        std::sync::Arc::new(tucano_b3::data::reload::DataLoader::new(&config, cache).unwrap());

    let result =
        tucano_b3::ingest::service::IngestService::ingest_zip(zip_data.clone(), &data_dir, &data_loader);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.files_processed, 1);
    assert!(result.total_records_parsed > 0);
    assert!(result.total_records_new > 0);
    assert!(result.success);

    // Verify Parquet file was created
    let parquet_path = data_dir.join("processed/cotahist/cotahist-2026.parquet");
    assert!(parquet_path.exists());

    // Verify idempotency: re-ingest same ZIP
    let result2 =
        tucano_b3::ingest::service::IngestService::ingest_zip(zip_data.clone(), &data_dir, &data_loader);

    assert!(result2.is_ok());
    let result2 = result2.unwrap();

    // Second ingestion should skip all records
    assert_eq!(result2.total_records_new, 0);
    assert_eq!(result2.total_records_skipped, result.total_records_parsed);
}
