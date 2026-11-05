use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B3Settings {
    pub database_url: String,
    pub b3_allowed_hosts: String,
    pub b3_raw_data_dir: String,
    pub b3_processed_data_dir: String,
    pub b3_sync_enabled: bool,
    pub b3_manual_sync_enabled: bool,
    pub b3_default_timezone: String,
    pub b3_parser_version: String,
    pub tucano_b3_token: String,
    pub b3_source_url_bvbg_028: Option<String>,
    pub b3_source_url_tradable_security_list: Option<String>,
    pub b3_source_url_bvbg_086: Option<String>,
    pub b3_source_url_bvbg_186: Option<String>,
    pub b3_source_url_bvbg_087: Option<String>,
    pub b3_source_url_index_composition: Option<String>,
    pub b3_source_url_cotahist: Option<String>,
    pub log_level: String,
}

impl Default for B3Settings {
    fn default() -> Self {
        if env::var("DATABASE_URL").is_err() {
            if let Ok(content) = std::fs::read_to_string(".env") {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.starts_with('#') && !trimmed.is_empty() {
                        if let Some((k, v)) = trimmed.split_once('=') {
                            let k = k.trim();
                            let v = v.trim();
                            if env::var(k).is_err() {
                                env::set_var(k, v);
                            }
                        }
                    }
                }
            }
        }

        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string()),
            b3_allowed_hosts: env::var("B3_ALLOWED_HOSTS").unwrap_or_else(|_| {
                "www.b3.com.br,b3.com.br,bvmf.bmfbovespa.com.br,arquivos.b3.com.br,sistemaswebb3-listados.b3.com.br".to_string()
            }),
            b3_raw_data_dir: env::var("B3_RAW_DATA_DIR").unwrap_or_else(|_| "/data/raw".to_string()),
            b3_processed_data_dir: env::var("B3_PROCESSED_DATA_DIR").unwrap_or_else(|_| "/data/processed".to_string()),
            b3_sync_enabled: env::var("B3_SYNC_ENABLED").map(|v| v.to_lowercase() == "true" || v == "1").unwrap_or(true),
            b3_manual_sync_enabled: env::var("B3_MANUAL_SYNC_ENABLED").map(|v| v.to_lowercase() == "true" || v == "1").unwrap_or(true),
            b3_default_timezone: env::var("B3_DEFAULT_TIMEZONE").unwrap_or_else(|_| "America/Sao_Paulo".to_string()),
            b3_parser_version: env::var("B3_PARSER_VERSION").unwrap_or_else(|_| "v1".to_string()),
            tucano_b3_token: env::var("TUCANO_B3_TOKEN").unwrap_or_else(|_| "dev-token".to_string()),
            b3_source_url_bvbg_028: env::var("B3_SOURCE_URL_BVBG_028").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_tradable_security_list: env::var("B3_SOURCE_URL_TRADABLE_SECURITY_LIST").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_bvbg_086: env::var("B3_SOURCE_URL_BVBG_086").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_bvbg_186: env::var("B3_SOURCE_URL_BVBG_186").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_bvbg_087: env::var("B3_SOURCE_URL_BVBG_087").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_index_composition: env::var("B3_SOURCE_URL_INDEX_COMPOSITION").ok().filter(|s| !s.trim().is_empty()),
            b3_source_url_cotahist: env::var("B3_SOURCE_URL_COTAHIST").ok().filter(|s| !s.trim().is_empty()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()),
        }
    }
}
