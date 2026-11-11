//! Metrics module for tucano-b3.
//!
//! Provides Prometheus metrics collection and exposition.

pub mod registry;

pub use registry::AppMetrics;
