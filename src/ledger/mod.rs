//! SQLite ledger for tucano-b3.
//!
//! Tracks ticker metadata and ingested dates.

pub mod connection;

pub use connection::{Ledger, LedgerError};
