use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

use crate::config::B3Settings;
use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct DownloadedFile {
    pub content: Vec<u8>,
    pub sha256: String,
    pub source_url: String,
}

pub struct B3DownloadService {
    allowed_hosts: HashSet<String>,
    raw_dir: PathBuf,
}

impl B3DownloadService {
    pub fn new(settings: &B3Settings) -> Self {
        let allowed_hosts = settings
            .b3_allowed_hosts
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let raw_dir = PathBuf::from(&settings.b3_raw_data_dir);
        Self {
            allowed_hosts,
            raw_dir,
        }
    }

    pub fn validate_url(&self, url_str: &str) -> Result<()> {
        if let Ok(parsed) = Url::parse(url_str) {
            if parsed.scheme() == "file" {
                let path = parsed.to_file_path().map_err(|_| {
                    AppError::Validation(format!("Invalid file URL path: {url_str}"))
                })?;
                return self.validate_local_path(&path);
            }

            if let Some(host) = parsed.host_str() {
                let host_lower = host.to_lowercase();
                if !self.allowed_hosts.contains(&host_lower) {
                    return Err(AppError::HostNotAllowed(host_lower));
                }
                return Ok(());
            }
        }

        // Plain path
        let path = PathBuf::from(url_str);
        self.validate_local_path(&path)
    }

    fn validate_local_path(&self, path: &Path) -> Result<()> {
        let canonical_raw = self.raw_dir.canonicalize().unwrap_or_else(|_| self.raw_dir.clone());
        let target = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.raw_dir.join(path)
        };

        let canonical_target = target.canonicalize().map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found or invalid: {target:?} ({e})"),
            ))
        })?;

        if !canonical_target.starts_with(&canonical_raw) && canonical_target != canonical_raw {
            return Err(AppError::Validation(format!(
                "Path traversal detected: {path:?} is outside {canonical_raw:?}"
            )));
        }

        Ok(())
    }

    pub async fn download(&self, url_str: &str) -> Result<DownloadedFile> {
        self.validate_url(url_str)?;

        if let Ok(parsed) = Url::parse(url_str) {
            if parsed.scheme() == "http" || parsed.scheme() == "https" {
                let client = reqwest::Client::builder().build()?;
                let resp = client.get(url_str).send().await?.error_for_status()?;
                let content = resp.bytes().await?.to_vec();
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let sha256 = hex::encode(hasher.finalize());
                return Ok(DownloadedFile {
                    content,
                    sha256,
                    source_url: url_str.to_string(),
                });
            } else if parsed.scheme() == "file" {
                let path = parsed.to_file_path().map_err(|_| {
                    AppError::Validation(format!("Invalid file URL path: {url_str}"))
                })?;
                let content = fs::read(&path)?;
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let sha256 = hex::encode(hasher.finalize());
                return Ok(DownloadedFile {
                    content,
                    sha256,
                    source_url: url_str.to_string(),
                });
            }
        }

        let target = if Path::new(url_str).is_absolute() {
            PathBuf::from(url_str)
        } else {
            self.raw_dir.join(url_str)
        };
        let content = fs::read(&target)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let sha256 = hex::encode(hasher.finalize());
        Ok(DownloadedFile {
            content,
            sha256,
            source_url: url_str.to_string(),
        })
    }
}

