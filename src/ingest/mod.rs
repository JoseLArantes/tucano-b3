//! Data ingestion module for tucano-b3.
//!
//! Provides standalone COTAHIST ingestion without requiring tucano-b3-cli.
//! Supports ZIP upload, folder scanning, incremental and idempotent ingestion.

pub mod cotahist_parser;
