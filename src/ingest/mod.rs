//! Data ingestion module for tucano-b3.
//!
//! Provides standalone COTAHIST ingestion into Parquet + SQLite ledger.
//! Supports ZIP upload, folder scanning, incremental and idempotent ingestion.

pub mod cotahist_parser;
pub mod parquet_writer;
pub mod service;
pub mod zip_handler;

pub use service::IngestService;
