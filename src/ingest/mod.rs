//! Data ingestion module for tucano-b3.
//!
//! Provides standalone COTAHIST ingestion into Parquet + SQLite ledger.
//! Supports ZIP upload, folder scanning, S3-compatible bucket ingest,
//! and incremental idempotent ingestion.

pub mod cotahist_parser;
pub mod object_storage;
pub mod parquet_writer;
pub mod service;
pub mod zip_handler;

pub use service::IngestService;
