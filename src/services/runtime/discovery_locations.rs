//! Configured, metadata-only discovery across local folders and official B3/S3 locations.
use crate::{
    error::{AppError, Result},
    services::artifacts::{ArtifactLimits, ArtifactService},
};
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{collections::HashSet, fs, path::PathBuf, time::SystemTime};
use url::Url;
use utoipa::ToSchema;

const MAX_LOCATIONS: usize = 32;
const MAX_ENTRIES_PER_LOCATION: usize = 10_000;
const MAX_DIRECTORY_DEPTH: usize = 8;
/// Upper bound for candidates returned by the manual REST response.
pub const MAX_RESPONSE_CANDIDATES: usize = 1_000;

/// COTAHIST file family inferred from a recognized file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscoveryFileType {
    /// Annual COTAHIST archive.
    CotahistYearly,
    /// Monthly COTAHIST archive.
    CotahistMonthly,
    /// Daily COTAHIST archive.
    CotahistDaily,
}

/// Supported source-location types for configured discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryLocationKind {
    /// A local filesystem directory.
    Folder,
    /// A direct URL to an official B3 COTAHIST artifact.
    B3Url,
    /// An AWS S3-compatible bucket and prefix.
    ObjectStorageBucket,
}

impl DiscoveryLocationKind {
    /// Stable API and persistence value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::B3Url => "b3_url",
            Self::ObjectStorageBucket => "object_storage_bucket",
        }
    }
}

/// Result states returned for one configured location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryLocationOutcome {
    /// At least one COTAHIST candidate was identified.
    Available,
    /// The location was checked and no candidate was identified within bounds.
    Absent,
    /// The location was only partially checked or could not be conclusive.
    Inconclusive,
    /// A source-specific failure prevented a reliable check.
    Failed,
}

impl DiscoveryLocationOutcome {
    /// Stable API and persistence value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Absent => "absent",
            Self::Inconclusive => "inconclusive",
            Self::Failed => "failed",
        }
    }
}

impl DiscoveryFileType {
    /// Stable source-family identifier used by persistence and the REST response.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CotahistYearly => "COTAHIST_YEARLY",
            Self::CotahistMonthly => "COTAHIST_MONTHLY",
            Self::CotahistDaily => "COTAHIST_DAILY",
        }
    }
}

/// Configured discovery source. Configuration uses a semicolon-separated list of
/// `folder:`, `b3_url:`/`host:`, and `object_storage_bucket:` entries.
#[derive(Debug, Clone)]
pub enum DiscoveryLocation {
    /// Filesystem directory scanned without following symlinks or reading file contents.
    Folder(PathBuf),
    /// Direct HTTPS URL for a COTAHIST file on an explicitly allowed B3 host.
    B3Url {
        /// Validated B3 artifact address.
        url: Url,
        /// Family inferred from the direct artifact filename.
        source_file_type: DiscoveryFileType,
    },
    /// S3 bucket and optional object key prefix, listed through ListObjectsV2.
    S3Bucket {
        /// Bucket name without an endpoint or credentials.
        bucket: String,
        /// Optional key prefix within the bucket.
        prefix: String,
    },
}

impl DiscoveryLocation {
    /// Returns the stable configuration kind used by reports and persistence.
    pub fn kind(&self) -> DiscoveryLocationKind {
        match self {
            Self::Folder(_) => DiscoveryLocationKind::Folder,
            Self::B3Url { .. } => DiscoveryLocationKind::B3Url,
            Self::S3Bucket { .. } => DiscoveryLocationKind::ObjectStorageBucket,
        }
    }

    /// Generates a path-free label from the location's configuration position.
    pub fn label(&self, index: usize) -> String {
        format!("{}-{index}", self.kind().as_str())
    }
}

/// One metadata observation for a COTAHIST artifact discovered at a configured location.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredCandidate {
    /// COTAHIST granularity inferred from the artifact filename.
    pub source_file_type: DiscoveryFileType,
    /// Sanitized source location; never contains AWS or URL credentials.
    pub source_locator: String,
    /// Source-reported artifact size in bytes, when present.
    pub byte_size: Option<i64>,
    /// Remote ETag, not a cryptographic content checksum.
    pub remote_etag: Option<String>,
    /// Last-modified metadata reported by the filesystem or remote provider.
    pub remote_last_modified: Option<String>,
}

/// Persistable result for one configured location.
#[derive(Debug, Clone)]
pub struct LocationScan {
    /// Source kind from the closed supported set.
    pub location_kind: DiscoveryLocationKind,
    /// Positional label that omits the private configured root.
    pub location_label: String,
    /// Outcome for this location.
    pub outcome: DiscoveryLocationOutcome,
    /// Candidate count found within the per-location inspection bound.
    pub candidate_count: usize,
    /// Whether the source listing reached a configured scan bound.
    pub truncated: bool,
    /// Candidates found, capped by the per-location entry bound.
    pub candidates: Vec<DiscoveredCandidate>,
}

/// Bounded aggregate returned to the manual discovery API.
#[derive(Debug, Clone)]
pub struct DiscoveryReport {
    /// Ordered results for configured locations.
    pub locations: Vec<LocationScan>,
    /// Total candidates found within per-location scan bounds.
    pub candidate_count: usize,
    /// Candidates included in the bounded API representation.
    pub candidates: Vec<DiscoveredCandidate>,
    /// Whether location inspection or API response candidate output was truncated.
    pub candidates_truncated: bool,
}

/// Parse and validate locations before runtime starts so unsupported sources are not ignored.
pub fn parse_locations(input: &str, allowed_hosts: &[String]) -> Result<Vec<DiscoveryLocation>> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let allowed_hosts: HashSet<String> = allowed_hosts
        .iter()
        .map(|host| host.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|host| !host.is_empty())
        .collect();
    let mut locations = Vec::new();
    for entry in input
        .split([';', ','])
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        if locations.len() >= MAX_LOCATIONS {
            return Err(invalid("B3_DISCOVERY_LOCATIONS excede 32 locais."));
        }
        let (kind, value) = entry
            .split_once(':')
            .ok_or_else(|| invalid("Cada local de descoberta exige tipo:valor."))?;
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid("Local de descoberta sem endereço."));
        }
        let location = match kind.trim().to_ascii_lowercase().as_str() {
            "folder" => {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err(invalid("Locais folder devem usar caminho absoluto."));
                }
                DiscoveryLocation::Folder(path)
            }
            "b3_url" | "host" => {
                let url = Url::parse(value).map_err(|_| invalid("URL B3 inválida."))?;
                let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
                let file_type = url
                    .path_segments()
                    .and_then(|mut parts| parts.next_back())
                    .and_then(cotahist_file_type)
                    .ok_or_else(|| {
                        invalid(
                            "URL B3 deve apontar diretamente para arquivo COTAHIST .TXT ou .ZIP.",
                        )
                    })?;
                if url.scheme() != "https"
                    || url.port_or_known_default() != Some(443)
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || !allowed_hosts.contains(host.trim_end_matches('.'))
                {
                    return Err(invalid("URL B3 deve usar HTTPS, host oficial permitido e não conter credenciais ou parâmetros."));
                }
                DiscoveryLocation::B3Url {
                    url,
                    source_file_type: file_type,
                }
            }
            "object_storage_bucket" => {
                let (bucket, prefix) = parse_s3_location(value)?;
                DiscoveryLocation::S3Bucket { bucket, prefix }
            }
            _ => {
                return Err(invalid(
                    "Tipo de local desconhecido em B3_DISCOVERY_LOCATIONS.",
                ))
            }
        };
        locations.push(location);
    }
    Ok(locations)
}

/// Finds COTAHIST filenames without opening file bodies. Symlinks are never followed.
pub fn scan_folder(location: &DiscoveryLocation, index: usize) -> Result<LocationScan> {
    let DiscoveryLocation::Folder(configured_path) = location else {
        return Err(invalid("Local não é uma pasta."));
    };
    let label = location.label(index);
    let root = match configured_path.canonicalize() {
        Ok(path) if path.is_dir() => path,
        Ok(_) => {
            return Ok(empty_location(
                location,
                index,
                DiscoveryLocationOutcome::Failed,
                false,
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(empty_location(
                location,
                index,
                DiscoveryLocationOutcome::Absent,
                false,
            ));
        }
        Err(_) => {
            return Ok(empty_location(
                location,
                index,
                DiscoveryLocationOutcome::Failed,
                false,
            ))
        }
    };

    let mut stack = vec![(root.clone(), PathBuf::new(), 0usize)];
    let mut candidates = Vec::new();
    let mut entries_seen = 0usize;
    let mut truncated = false;
    'walk: while let Some((directory, relative_directory, depth)) = stack.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                return Ok(empty_location(
                    location,
                    index,
                    DiscoveryLocationOutcome::Failed,
                    false,
                ))
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    return Ok(empty_location(
                        location,
                        index,
                        DiscoveryLocationOutcome::Failed,
                        false,
                    ))
                }
            };
            entries_seen += 1;
            if entries_seen > MAX_ENTRIES_PER_LOCATION {
                truncated = true;
                break 'walk;
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => {
                    return Ok(empty_location(
                        location,
                        index,
                        DiscoveryLocationOutcome::Failed,
                        false,
                    ))
                }
            };
            let relative = relative_directory.join(entry.file_name());
            if file_type.is_dir() {
                if depth < MAX_DIRECTORY_DEPTH {
                    stack.push((entry.path(), relative, depth + 1));
                } else {
                    truncated = true;
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Some(source_file_type) = entry.file_name().to_str().and_then(cotahist_file_type)
            else {
                continue;
            };
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    return Ok(empty_location(
                        location,
                        index,
                        DiscoveryLocationOutcome::Failed,
                        false,
                    ))
                }
            };
            if !entry.path().starts_with(&root) {
                return Ok(empty_location(
                    location,
                    index,
                    DiscoveryLocationOutcome::Failed,
                    false,
                ));
            }
            let relative_locator = relative.to_string_lossy().replace('\\', "/");
            candidates.push(DiscoveredCandidate {
                source_file_type,
                source_locator: format!("folder:{index}/{relative_locator}"),
                byte_size: i64::try_from(metadata.len()).ok(),
                remote_etag: None,
                remote_last_modified: metadata.modified().ok().map(system_time_string),
            });
        }
    }
    let candidate_count = candidates.len();
    candidates.sort_by(|left, right| left.source_locator.cmp(&right.source_locator));
    let outcome = if candidate_count > 0 {
        DiscoveryLocationOutcome::Available
    } else if truncated {
        DiscoveryLocationOutcome::Inconclusive
    } else {
        DiscoveryLocationOutcome::Absent
    };
    Ok(LocationScan {
        location_kind: location.kind(),
        location_label: label,
        outcome,
        candidate_count,
        truncated,
        candidates,
    })
}

/// Uses a metadata-only HEAD request through the existing B3 host/redirect protections.
pub async fn scan_b3_url(
    location: &DiscoveryLocation,
    index: usize,
    root: PathBuf,
    allowed_hosts: Vec<String>,
) -> Result<LocationScan> {
    let DiscoveryLocation::B3Url {
        url,
        source_file_type,
    } = location
    else {
        return Err(invalid("Local não é uma URL B3."));
    };
    let limits = ArtifactLimits {
        request_timeout: std::time::Duration::from_secs(8),
        ..ArtifactLimits::default()
    };
    let service = ArtifactService::new(root, allowed_hosts, limits)?;
    let observation = service.probe(url.as_str()).await?;
    let outcome = match observation.status {
        200..=299 => DiscoveryLocationOutcome::Available,
        404 => DiscoveryLocationOutcome::Absent,
        _ => DiscoveryLocationOutcome::Inconclusive,
    };
    let candidates = if outcome == DiscoveryLocationOutcome::Available {
        vec![DiscoveredCandidate {
            source_file_type: *source_file_type,
            source_locator: url.to_string(),
            byte_size: observation
                .content_length
                .and_then(|v| i64::try_from(v).ok()),
            remote_etag: observation.etag,
            remote_last_modified: observation.last_modified,
        }]
    } else {
        Vec::new()
    };
    Ok(LocationScan {
        location_kind: location.kind(),
        location_label: location.label(index),
        outcome,
        candidate_count: candidates.len(),
        truncated: false,
        candidates,
    })
}

/// Lists object metadata only. It never performs `GetObject` or reads object contents.
pub async fn scan_s3_bucket(
    location: &DiscoveryLocation,
    index: usize,
    client: &S3Client,
) -> Result<LocationScan> {
    let DiscoveryLocation::S3Bucket { bucket, prefix } = location else {
        return Err(invalid("Local não é um bucket S3."));
    };
    let mut paginator = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(prefix)
        .max_keys(1000)
        .into_paginator()
        .send();
    let mut candidates = Vec::new();
    let mut inspected = 0usize;
    let mut truncated = false;
    while let Some(page) = paginator
        .try_next()
        .await
        .map_err(|_| invalid("Falha ao listar metadados do bucket configurado."))?
    {
        for object in page.contents() {
            inspected += 1;
            if inspected > MAX_ENTRIES_PER_LOCATION {
                truncated = true;
                break;
            }
            let Some(key) = object.key() else { continue };
            let Some(source_file_type) = key.rsplit('/').next().and_then(cotahist_file_type) else {
                continue;
            };
            candidates.push(DiscoveredCandidate {
                source_file_type,
                source_locator: format!("s3://{bucket}/{key}"),
                byte_size: object.size(),
                remote_etag: object.e_tag().map(str::to_owned),
                remote_last_modified: object.last_modified().map(ToString::to_string),
            });
        }
        if truncated {
            break;
        }
        if inspected >= MAX_ENTRIES_PER_LOCATION && page.is_truncated().unwrap_or(false) {
            truncated = true;
            break;
        }
    }
    let candidate_count = candidates.len();
    candidates.sort_by(|left, right| left.source_locator.cmp(&right.source_locator));
    let outcome = if candidate_count > 0 {
        DiscoveryLocationOutcome::Available
    } else if truncated {
        DiscoveryLocationOutcome::Inconclusive
    } else {
        DiscoveryLocationOutcome::Absent
    };
    Ok(LocationScan {
        location_kind: location.kind(),
        location_label: location.label(index),
        outcome,
        candidate_count,
        truncated,
        candidates,
    })
}

pub async fn build_s3_client() -> Result<S3Client> {
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let timeout = aws_sdk_s3::config::timeout::TimeoutConfig::builder()
        .operation_timeout(std::time::Duration::from_secs(8))
        .build();
    let retries = aws_sdk_s3::config::retry::RetryConfig::standard().with_max_attempts(2);
    let config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .timeout_config(timeout)
        .retry_config(retries)
        .build();
    Ok(S3Client::from_conf(config))
}

fn parse_s3_location(value: &str) -> Result<(String, String)> {
    let url = Url::parse(value).map_err(|_| invalid("Bucket deve usar s3://bucket/prefixo."))?;
    if url.scheme() != "s3"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid(
            "Bucket deve usar s3://bucket/prefixo sem credenciais.",
        ));
    }
    let bucket = url.host_str().unwrap_or_default().to_string();
    if bucket.len() < 3 || bucket.len() > 255 || bucket.contains(['/', '\\']) {
        return Err(invalid("Nome de bucket inválido."));
    }
    let prefix = url.path().trim_start_matches('/').to_string();
    Ok((bucket, prefix))
}

fn cotahist_file_type(filename: &str) -> Option<DiscoveryFileType> {
    let (stem, extension) = filename.rsplit_once('.')?;
    if !matches!(extension.to_ascii_uppercase().as_str(), "ZIP" | "TXT") {
        return None;
    }
    let stem = stem.to_ascii_uppercase();
    let suffix = stem.strip_prefix("COTAHIST_")?;
    match suffix.as_bytes() {
        [b'A', a, b, c, d] if [a, b, c, d].iter().all(|value| value.is_ascii_digit()) => {
            Some(DiscoveryFileType::CotahistYearly)
        }
        [b'M', m1, m2, y1, y2, y3, y4]
            if m1.is_ascii_digit()
                && m2.is_ascii_digit()
                && (1..=12).contains(&((m1 - b'0') * 10 + (m2 - b'0')))
                && [y1, y2, y3, y4].iter().all(|value| value.is_ascii_digit()) =>
        {
            Some(DiscoveryFileType::CotahistMonthly)
        }
        [b'D', d1, d2, m1, m2, y1, y2, y3, y4]
            if [d1, d2, m1, m2, y1, y2, y3, y4]
                .iter()
                .all(|value| value.is_ascii_digit()) =>
        {
            let date = [*d1, *d2, *m1, *m2, *y1, *y2, *y3, *y4];
            let date = std::str::from_utf8(&date).ok()?;
            chrono::NaiveDate::parse_from_str(date, "%d%m%Y").ok()?;
            Some(DiscoveryFileType::CotahistDaily)
        }
        _ => None,
    }
}

fn empty_location(
    location: &DiscoveryLocation,
    index: usize,
    outcome: DiscoveryLocationOutcome,
    truncated: bool,
) -> LocationScan {
    LocationScan {
        location_kind: location.kind(),
        location_label: location.label(index),
        outcome,
        candidate_count: 0,
        truncated,
        candidates: Vec::new(),
    }
}

fn system_time_string(value: SystemTime) -> String {
    DateTime::<Utc>::from(value).to_rfc3339()
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed_hosts() -> Vec<String> {
        vec!["www.b3.com.br".to_string()]
    }

    #[test]
    fn parses_supported_sources_and_rejects_unknown_types() {
        let locations = parse_locations(
            "folder:/data/imports;b3_url:https://www.b3.com.br/files/COTAHIST_A2025.ZIP;object_storage_bucket:s3://market-data/cotahist/",
            &allowed_hosts(),
        )
        .unwrap();
        assert_eq!(locations.len(), 3);
        assert_eq!(
            parse_locations(
                "folder:/data/imports,host:https://www.b3.com.br/files/COTAHIST_A2025.ZIP",
                &allowed_hosts()
            )
            .unwrap()
            .len(),
            2
        );
        assert!(parse_locations("web_scrapper:https://www.b3.com.br", &allowed_hosts()).is_err());
    }

    #[test]
    fn rejects_non_official_urls_and_bucket_credentials() {
        assert!(parse_locations(
            "b3_url:http://www.b3.com.br/files/COTAHIST_A2025.ZIP",
            &allowed_hosts()
        )
        .is_err());
        assert!(parse_locations(
            "b3_url:https://example.com/COTAHIST_A2025.ZIP",
            &allowed_hosts()
        )
        .is_err());
        assert!(parse_locations(
            "object_storage_bucket:s3://user:pass@bucket/prefix",
            &allowed_hosts()
        )
        .is_err());
    }

    #[test]
    fn classifies_only_cotahist_file_families() {
        assert_eq!(
            cotahist_file_type("COTAHIST_A2025.ZIP"),
            Some(DiscoveryFileType::CotahistYearly)
        );
        assert_eq!(
            cotahist_file_type("COTAHIST_M012026.TXT"),
            Some(DiscoveryFileType::CotahistMonthly)
        );
        assert_eq!(
            cotahist_file_type("COTAHIST_D01032026.ZIP"),
            Some(DiscoveryFileType::CotahistDaily)
        );
        assert_eq!(cotahist_file_type("COTAHIST_M132026.ZIP"), None);
        assert_eq!(cotahist_file_type("other.zip"), None);
    }

    #[test]
    fn scans_nested_folder_metadata_without_opening_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("2025");
        fs::create_dir(&nested).unwrap();
        fs::write(
            nested.join("COTAHIST_A2025.ZIP"),
            b"synthetic metadata-only fixture",
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(temp.path(), nested.join("loop")).unwrap();
        let location = DiscoveryLocation::Folder(temp.path().to_path_buf());
        let report = scan_folder(&location, 1).unwrap();
        assert_eq!(report.outcome, DiscoveryLocationOutcome::Available);
        assert_eq!(report.candidate_count, 1);
        assert_eq!(
            report.candidates[0].source_locator,
            "folder:1/2025/COTAHIST_A2025.ZIP"
        );
        assert_eq!(report.candidates[0].byte_size, Some(31));
    }

    #[test]
    fn s3_uris_require_bucket_and_return_prefix_without_leaking_credentials() {
        let (bucket, prefix) = parse_s3_location("s3://data-bucket/cotahist/2025").unwrap();
        assert_eq!(bucket, "data-bucket");
        assert_eq!(prefix, "cotahist/2025");
        assert!(parse_s3_location("s3:///prefix").is_err());
    }

    #[tokio::test]
    async fn lists_s3_object_metadata_through_a_local_compatible_endpoint() {
        use aws_credential_types::Credentials;
        use axum::{routing::get, Router};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = Router::new().fallback({
            let calls = calls.clone();
            get(move || {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    let body = if call == 0 {
                        r#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>data-bucket</Name><Prefix>cotahist/</Prefix><KeyCount>1</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>true</IsTruncated><NextContinuationToken>page-two</NextContinuationToken><Contents><Key>cotahist/COTAHIST_A2025.ZIP</Key><LastModified>2025-01-01T00:00:00.000Z</LastModified><ETag>&quot;opaque-tag&quot;</ETag><Size>123</Size><StorageClass>STANDARD</StorageClass></Contents></ListBucketResult>"#
                    } else {
                        r#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>data-bucket</Name><Prefix>cotahist/</Prefix><KeyCount>1</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated><Contents><Key>cotahist/COTAHIST_D01032026.TXT</Key><LastModified>2026-03-01T00:00:00.000Z</LastModified><ETag>&quot;another-tag&quot;</ETag><Size>456</Size><StorageClass>STANDARD</StorageClass></Contents></ListBucketResult>"#
                    };
                    ([(axum::http::header::CONTENT_TYPE, "application/xml")], body)
                }
            })
        });
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "unit-test",
            ))
            .endpoint_url(format!("http://{address}"))
            .load()
            .await;
        let config = aws_sdk_s3::config::Builder::from(&shared)
            .force_path_style(true)
            .build();
        let client = S3Client::from_conf(config);
        let location = DiscoveryLocation::S3Bucket {
            bucket: "data-bucket".into(),
            prefix: "cotahist/".into(),
        };
        let report = scan_s3_bucket(&location, 1, &client).await.unwrap();
        assert_eq!(report.outcome, DiscoveryLocationOutcome::Available);
        assert_eq!(report.candidate_count, 2);
        assert_eq!(
            report.candidates[0].source_file_type,
            DiscoveryFileType::CotahistYearly
        );
        assert_eq!(
            report.candidates[0].source_locator,
            "s3://data-bucket/cotahist/COTAHIST_A2025.ZIP"
        );
        assert_eq!(report.candidates[0].byte_size, Some(123));
        assert_eq!(
            report.candidates[0].remote_etag.as_deref(),
            Some("\"opaque-tag\"")
        );
        assert_eq!(
            report.candidates[1].source_file_type,
            DiscoveryFileType::CotahistDaily
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
