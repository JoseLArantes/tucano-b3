//! API key management.
//!
//! Loads API keys from a static TOML file.
//! Each key has a name and an optional custom rate limit.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use thiserror::Error;

/// A single API key entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// The API key string (sent in X-API-Key header)
    pub key: String,
    /// Human-readable name for logging
    pub name: String,
    /// Custom rate limit (requests per minute). Overrides default.
    #[serde(default)]
    pub rate_limit: Option<u32>,
}

/// API key file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyFile {
    #[serde(default)]
    pub keys: Vec<ApiKeyEntry>,
}

/// Errors during API key loading.
#[derive(Error, Debug)]
pub enum ApiKeyError {
    #[error("API key file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to read API key file: {0}")]
    Read(#[from] std::io::Error),

    #[error("Failed to parse API key file: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Thread-safe API key registry.
pub struct ApiKeyRegistry {
    keys: RwLock<HashMap<String, ApiKeyEntry>>,
}

impl ApiKeyRegistry {
    /// Load API keys from a TOML file.
    ///
    /// If the file doesn't exist, returns an empty registry (no keys).
    pub fn load(path: &Path) -> Result<Self, ApiKeyError> {
        if !path.exists() {
            tracing::warn!(
                path = %path.display(),
                "Arquivo de API keys não encontrado, registry vazio"
            );
            return Ok(Self {
                keys: RwLock::new(HashMap::new()),
            });
        }

        let content = std::fs::read_to_string(path)?;
        let file: ApiKeyFile = toml::from_str(&content)?;

        let mut keys = HashMap::new();
        for entry in file.keys {
            keys.insert(entry.key.clone(), entry);
        }

        tracing::info!(count = keys.len(), "API keys carregadas");

        Ok(Self {
            keys: RwLock::new(keys),
        })
    }

    /// Create an empty registry (no API keys).
    pub fn empty() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Helper to acquire read lock safely.
    fn read_lock(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, ApiKeyEntry>> {
        match self.keys.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Validate an API key and return its entry if valid.
    pub fn validate(&self, key: &str) -> Option<ApiKeyEntry> {
        let keys = self.read_lock();
        keys.get(key).cloned()
    }

    /// Check if a key exists.
    pub fn exists(&self, key: &str) -> bool {
        let keys = self.read_lock();
        keys.contains_key(key)
    }

    /// Get the rate limit for a key (custom or default).
    pub fn rate_limit_for(&self, key: &str, default: u32) -> u32 {
        let keys = self.read_lock();
        keys.get(key)
            .and_then(|entry| entry.rate_limit)
            .unwrap_or(default)
    }

    /// Get the total number of registered keys.
    pub fn count(&self) -> usize {
        let keys = self.read_lock();
        keys.len()
    }
}
