//! Release execution and scheduled discovery share PostgreSQL coordination.
pub mod discovery;
pub mod discovery_locations;
pub mod executor;
pub mod local_channels;
mod registry;

use std::path::PathBuf;

/// Resource caps are explicit configuration, not performance claims.
#[derive(Clone)]
pub struct RuntimeSettings {
    pub raw_root: PathBuf,
    pub allowed_hosts: Vec<String>,
    pub discovery_locations: String,
    pub batch_records: usize,
    pub batch_bytes: usize,
    pub lease_seconds: i32,
}

impl RuntimeSettings {
    pub fn from_settings(settings: &crate::config::B3Settings) -> Self {
        Self {
            raw_root: PathBuf::from(&settings.b3_raw_data_dir),
            allowed_hosts: settings
                .b3_allowed_hosts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            discovery_locations: settings.b3_discovery_locations.clone(),
            batch_records: configured("B3_BATCH_RECORDS", 500),
            batch_bytes: configured("B3_BATCH_BYTES", 4 * 1024 * 1024),
            lease_seconds: configured("B3_LEASE_SECONDS", 120).try_into().unwrap_or(0),
        }
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        discovery_locations::parse_locations(&self.discovery_locations, &self.allowed_hosts)?;
        if !(1..=1000).contains(&self.batch_records)
            || !(1024..=64 * 1024 * 1024).contains(&self.batch_bytes)
            || !(3..=3600).contains(&self.lease_seconds)
        {
            return Err(crate::error::AppError::Validation(
                "Limites de lote/lease inválidos na configuração.".into(),
            ));
        }
        Ok(())
    }
}

fn configured(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|v| v.parse().unwrap_or(0))
        .unwrap_or(default)
}
