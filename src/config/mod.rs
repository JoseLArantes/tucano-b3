//! Configuration module for tucano-b3.
//!
//! Configuration is loaded in the following priority order (highest first):
//! 1. Environment variables (TUCANO_*)
//! 2. Config file (config.toml)
//! 3. Built-in defaults
//!
//! All configuration values are immutable after startup.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Top-level configuration for the tucano-b3 service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub data: DataConfig,
    pub cache: CacheConfig,
    pub rate_limit: RateLimitConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub auth: AuthConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address (e.g., "0.0.0.0")
    pub host: String,
    /// Port number (e.g., 8080)
    pub port: u16,
    /// Number of tokio worker threads (0 = auto-detect)
    pub workers: usize,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
}

/// Data directory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Path to the data/ directory (contains processed/ and ledger/)
    pub dir: PathBuf,
    /// Auto-reload interval in seconds (0 = disabled)
    pub reload_interval_seconds: u64,
}

/// In-memory cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Time-to-live for cache entries in seconds
    pub ttl_seconds: u64,
    /// Maximum number of cache entries before LRU eviction
    pub max_entries: usize,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Default requests per minute per IP
    pub default_requests_per_minute: u32,
    /// Requests per minute per API key
    pub api_key_requests_per_minute: u32,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    pub level: String,
    /// Log format: json or pretty
    pub format: String,
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether Prometheus metrics are enabled
    pub enabled: bool,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// Token principal de autenticação (TUCANO_B3_TOKEN)
    pub token: Option<String>,
    /// Tokens adicionais permitidos (ex: TUCANO_B3_READ_TOKEN, TUCANO_B3_ADMIN_TOKEN)
    #[serde(default)]
    pub allowed_tokens: Vec<String>,
}

impl AuthConfig {
    /// Returns true if authentication is enabled (at least one token configured)
    pub fn is_enabled(&self) -> bool {
        self.token.is_some() || !self.allowed_tokens.is_empty()
    }

    /// Validate a candidate token against configured tokens
    pub fn validate_token(&self, candidate: &str) -> bool {
        if let Some(ref primary) = self.token {
            if primary == candidate {
                return true;
            }
        }
        for allowed in &self.allowed_tokens {
            if allowed == candidate {
                return true;
            }
        }
        false
    }
}

/// Load key-value pairs from .env file into std::env if not already set.
pub fn load_dotenv() {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if !key.is_empty() && std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
}

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Config file not found: {path}")]
    NotFound { path: String },

    #[error("Invalid configuration: {message}")]
    Invalid { message: String },
}

impl AppConfig {
    /// Load configuration from file and environment variables.
    ///
    /// Priority order (highest first):
    /// 1. Environment variables (TUCANO_SERVER_PORT, etc.)
    /// 2. Config file (config.toml)
    /// 3. Built-in defaults
    pub fn load(config_path: Option<&Path>) -> Result<Self, ConfigError> {
        load_dotenv();
        let mut config = Self::default();

        // Layer 1: Load from file if it exists
        if let Some(path) = config_path {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let file_config: AppConfig = toml::from_str(&content)?;
                config = file_config;
            } else {
                return Err(ConfigError::NotFound {
                    path: path.display().to_string(),
                });
            }
        }

        // Layer 2: Override with environment variables
        config.apply_env_overrides();

        // Validate
        config.validate()?;

        Ok(config)
    }

    /// Apply environment variable overrides.
    ///
    /// Environment variables use the pattern: TUCANO_{SECTION}_{FIELD}
    /// Examples:
    ///   TUCANO_SERVER_PORT=9090
    ///   TUCANO_DATA_DIR=/mnt/data
    ///   TUCANO_CACHE_TTL=600
    ///   TUCANO_LOG_LEVEL=debug
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("TUCANO_SERVER_HOST") {
            self.server.host = val;
        }
        if let Ok(val) = std::env::var("TUCANO_SERVER_PORT") {
            if let Ok(port) = val.parse() {
                self.server.port = port;
            }
        }
        if let Ok(val) = std::env::var("TUCANO_SERVER_WORKERS") {
            if let Ok(workers) = val.parse() {
                self.server.workers = workers;
            }
        }
        if let Ok(val) = std::env::var("TUCANO_DATA_DIR") {
            self.data.dir = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("TUCANO_CACHE_ENABLED") {
            self.cache.enabled = val == "true" || val == "1";
        }
        if let Ok(val) = std::env::var("TUCANO_CACHE_TTL") {
            if let Ok(ttl) = val.parse() {
                self.cache.ttl_seconds = ttl;
            }
        }
        if let Ok(val) = std::env::var("TUCANO_CACHE_MAX_ENTRIES") {
            if let Ok(max) = val.parse() {
                self.cache.max_entries = max;
            }
        }
        if let Ok(val) = std::env::var("TUCANO_LOG_LEVEL") {
            self.logging.level = val;
        }
        if let Ok(val) = std::env::var("TUCANO_LOG_FORMAT") {
            self.logging.format = val;
        }
        if let Ok(val) = std::env::var("TUCANO_B3_TOKEN") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.auth.token = Some(trimmed.to_string());
            }
        }
        if let Ok(val) = std::env::var("TUCANO_B3_READ_TOKEN") {
            let trimmed = val.trim();
            if !trimmed.is_empty() && !self.auth.allowed_tokens.contains(&trimmed.to_string()) {
                self.auth.allowed_tokens.push(trimmed.to_string());
            }
        }
        if let Ok(val) = std::env::var("TUCANO_B3_ADMIN_TOKEN") {
            let trimmed = val.trim();
            if !trimmed.is_empty() && !self.auth.allowed_tokens.contains(&trimmed.to_string()) {
                self.auth.allowed_tokens.push(trimmed.to_string());
            }
        }
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::Invalid {
                message: "server.port cannot be 0".to_string(),
            });
        }
        if self.cache.ttl_seconds == 0 && self.cache.enabled {
            return Err(ConfigError::Invalid {
                message: "cache.ttl_seconds cannot be 0 when cache is enabled".to_string(),
            });
        }
        if self.cache.max_entries == 0 && self.cache.enabled {
            return Err(ConfigError::Invalid {
                message: "cache.max_entries cannot be 0 when cache is enabled".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 0, // auto-detect
                request_timeout_seconds: 30,
            },
            data: DataConfig {
                dir: PathBuf::from("./data"),
                reload_interval_seconds: 0,
            },
            cache: CacheConfig {
                enabled: true,
                ttl_seconds: 300,
                max_entries: 10_000,
            },
            rate_limit: RateLimitConfig {
                enabled: true,
                default_requests_per_minute: 100,
                api_key_requests_per_minute: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            metrics: MetricsConfig { enabled: true },
            auth: AuthConfig::default(),
        }
    }
}
