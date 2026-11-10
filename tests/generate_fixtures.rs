//! Fixture generator for tucano-b3 tests.
//!
//! Run with: cargo test --test generate_fixtures -- --nocapture
//! This creates minimal Parquet files and SQLite ledger for integration testing.

use arrow::array::*;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[test]
fn generate_all_fixtures() {
    generate_cotahist_fixture();
    generate_ibov_fixture();
    generate_sqlite_fixture();
}

fn generate_cotahist_fixture() {
    let dir = Path::new("tests/fixtures/processed/cotahist");
    fs::create_dir_all(dir).ok();

    let schema = tucano_b3::data::schema::cotahist_schema();

    // 5 rows
    let ticker = StringArray::from(vec!["VALE3", "VALE3", "PETR4", "PETR4", "ITUB4"]);
    let isin = StringArray::from(vec![
        Some("BRVALEACNOR0"),
        Some("BRVALEACNOR0"),
        Some("BRPETRACNPR6"),
        Some("BRPETRACNPR6"),
        Some("BRITUBACNPR1"),
    ]);
    let company_name = StringArray::from(vec![
        Some("VALE S.A."),
        Some("VALE S.A."),
        Some("PETROBRAS"),
        Some("PETROBRAS"),
        Some("ITAU UNIBANCO"),
    ]);
    let date = Date32Array::from(vec![20454, 20455, 20454, 20455, 20454]); // 2026-01-02, 2026-01-03
    let year = Int32Array::from(vec![2026, 2026, 2026, 2026, 2026]);
    let market_type = UInt8Array::from(vec![Some(10), Some(10), Some(10), Some(10), Some(10)]);
    let mod_ref = StringArray::from(vec![
        Some("R$"),
        Some("R$"),
        Some("R$"),
        Some("R$"),
        Some("R$"),
    ]);
    let pre_abr = Float64Array::from(vec![
        Some(62.00),
        Some(62.10),
        Some(38.50),
        Some(38.80),
        Some(34.00),
    ]);
    let pre_max = Float64Array::from(vec![
        Some(62.50),
        Some(62.60),
        Some(39.00),
        Some(39.20),
        Some(34.50),
    ]);
    let pre_min = Float64Array::from(vec![
        Some(61.80),
        Some(61.90),
        Some(38.20),
        Some(38.50),
        Some(33.80),
    ]);
    let pre_med = Float64Array::from(vec![
        Some(62.10),
        Some(62.25),
        Some(38.70),
        Some(38.95),
        Some(34.15),
    ]);
    let pre_ult = Float64Array::from(vec![
        Some(62.17),
        Some(62.30),
        Some(38.91),
        Some(39.02),
        Some(34.22),
    ]);
    let pre_ofe_compra = Float64Array::from(vec![
        Some(62.15),
        Some(62.28),
        Some(38.90),
        Some(39.00),
        Some(34.20),
    ]);
    let pre_ofe_venda = Float64Array::from(vec![
        Some(62.18),
        Some(62.31),
        Some(38.92),
        Some(39.03),
        Some(34.23),
    ]);
    let tot_neg = UInt32Array::from(vec![
        Some(25400),
        Some(22100),
        Some(35000),
        Some(31000),
        Some(18000),
    ]);
    let quat_tot = UInt64Array::from(vec![
        Some(29700000),
        Some(26000000),
        Some(30900000),
        Some(28200000),
        Some(28800000),
    ]);
    let vol_tot = Float64Array::from(vec![
        Some(1847293440.0),
        Some(1623104880.0),
        Some(1203847120.0),
        Some(1102993000.0),
        Some(987441003.0),
    ]);
    let schema_version = UInt8Array::from(vec![Some(1), Some(1), Some(1), Some(1), Some(1)]);
    let ingested_at = TimestampNanosecondArray::from(vec![
        Some(1767355200000000000),
        Some(1767355200000000000),
        Some(1767355200000000000),
        Some(1767355200000000000),
        Some(1767355200000000000),
    ]);
    let source_file = StringArray::from(vec![
        Some("COTAHIST_A2026.TXT"),
        Some("COTAHIST_A2026.TXT"),
        Some("COTAHIST_A2026.TXT"),
        Some("COTAHIST_A2026.TXT"),
        Some("COTAHIST_A2026.TXT"),
    ]);
    let source_hash = StringArray::from(vec![
        Some("hash1"),
        Some("hash1"),
        Some("hash1"),
        Some("hash1"),
        Some("hash1"),
    ]);

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ticker),
            Arc::new(isin),
            Arc::new(company_name),
            Arc::new(date),
            Arc::new(year),
            Arc::new(market_type),
            Arc::new(mod_ref),
            Arc::new(pre_abr),
            Arc::new(pre_max),
            Arc::new(pre_min),
            Arc::new(pre_med),
            Arc::new(pre_ult),
            Arc::new(pre_ofe_compra),
            Arc::new(pre_ofe_venda),
            Arc::new(tot_neg),
            Arc::new(quat_tot),
            Arc::new(vol_tot),
            Arc::new(schema_version),
            Arc::new(ingested_at),
            Arc::new(source_file),
            Arc::new(source_hash),
        ],
    )
    .expect("Failed to create batch");

    let file_path = dir.join("cotahist-2026.parquet");
    let file = fs::File::create(&file_path).expect("Failed to create file");

    let props = WriterProperties::builder().build();
    let mut writer =
        ArrowWriter::try_new(file, schema, Some(props)).expect("Failed to create writer");
    writer.write(&batch).expect("Failed to write batch");
    writer.close().expect("Failed to close writer");

    println!("Generated: {}", file_path.display());
}

fn generate_ibov_fixture() {
    let dir = Path::new("tests/fixtures/processed/ibov");
    fs::create_dir_all(dir).ok();

    let schema = tucano_b3::data::schema::ibov_schema();

    // 2 rows
    let date = Date32Array::from(vec![20454, 20455]);
    let codigo = StringArray::from(vec!["IBOV", "IBOV"]);
    let nome = StringArray::from(vec![Some("IBOVESPA"), Some("IBOVESPA")]);
    let tipo = StringArray::from(vec![Some("INDICE"), Some("INDICE")]);
    let pre_abr = Float64Array::from(vec![Some(128000.0), Some(128500.0)]);
    let pre_max = Float64Array::from(vec![Some(129200.0), Some(129800.0)]);
    let pre_min = Float64Array::from(vec![Some(127500.0), Some(128100.0)]);
    let pre_med = Float64Array::from(vec![Some(128600.0), Some(129000.0)]);
    let pre_ult = Float64Array::from(vec![Some(128900.0), Some(129500.0)]);
    let vol_tot = Float64Array::from(vec![Some(25000000000.0), Some(24000000000.0)]);
    let num_neg = UInt64Array::from(vec![Some(4200000), Some(4100000)]);
    let qtd_tot = Float64Array::from(vec![Some(500000000.0), Some(480000000.0)]);
    let fat_cotacao = UInt8Array::from(vec![Some(1), Some(1)]);
    let pre_ofe_compra: Float64Array = Float64Array::from(vec![None, None]);
    let pre_ofe_venda: Float64Array = Float64Array::from(vec![None, None]);
    let ingested_at =
        TimestampNanosecondArray::from(vec![Some(1767355200000000000), Some(1767355200000000000)]);
    let source_file = StringArray::from(vec![
        Some("IBOV_HISTORICAL.TXT"),
        Some("IBOV_HISTORICAL.TXT"),
    ]);
    let source_hash = StringArray::from(vec![Some("ibovhash"), Some("ibovhash")]);

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(date),
            Arc::new(codigo),
            Arc::new(nome),
            Arc::new(tipo),
            Arc::new(pre_abr),
            Arc::new(pre_max),
            Arc::new(pre_min),
            Arc::new(pre_med),
            Arc::new(pre_ult),
            Arc::new(vol_tot),
            Arc::new(num_neg),
            Arc::new(qtd_tot),
            Arc::new(fat_cotacao),
            Arc::new(pre_ofe_compra),
            Arc::new(pre_ofe_venda),
            Arc::new(ingested_at),
            Arc::new(source_file),
            Arc::new(source_hash),
        ],
    )
    .expect("Failed to create IBOV batch");

    let file_path = dir.join("ibov-historical.parquet");
    let file = fs::File::create(&file_path).expect("Failed to create file");

    let props = WriterProperties::builder().build();
    let mut writer =
        ArrowWriter::try_new(file, schema, Some(props)).expect("Failed to create writer");
    writer.write(&batch).expect("Failed to write batch");
    writer.close().expect("Failed to close writer");

    println!("Generated: {}", file_path.display());
}

fn generate_sqlite_fixture() {
    let dir = Path::new("tests/fixtures/ledger");
    fs::create_dir_all(dir).ok();

    let db_path = dir.join("tucano.db");
    if db_path.exists() {
        fs::remove_file(&db_path).ok();
    }

    let conn = Connection::open(&db_path).expect("Failed to open sqlite db");
    conn.execute(
        "CREATE TABLE tickers (
            ticker TEXT PRIMARY KEY,
            isin TEXT,
            company_name TEXT,
            first_seen TEXT,
            last_seen TEXT,
            is_active INTEGER
        )",
        [],
    )
    .expect("Failed to create tickers table");

    conn.execute(
        "INSERT INTO tickers (ticker, isin, company_name, first_seen, last_seen, is_active) VALUES
        ('VALE3', 'BRVALEACNOR0', 'VALE S.A.', '2020-01-02', '2026-11-10', 1),
        ('PETR4', 'BRPETRACNPR6', 'PETROLEO BRASILEIRO S.A.', '2020-01-02', '2026-11-10', 1),
        ('ITUB4', 'BRITUBACNPR1', 'ITAU UNIBANCO HOLDING S.A.', '2020-01-02', '2026-11-10', 1)",
        [],
    )
    .expect("Failed to insert tickers");

    println!("Generated: {}", db_path.display());
}
