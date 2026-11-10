//! Data layer module for tucano-b3.
//!
//! Provides Parquet reading, DataFusion querying, and schema validation.

pub mod query_service;
pub mod reader;
pub mod schema;
pub mod ticker_registry;
