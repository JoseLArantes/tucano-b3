//! Bounded, content-addressed acquisition and deterministic archive inventory.
use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    net::{IpAddr, SocketAddr},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ArtifactLimits {
    pub download_bytes: u64,
    pub member_bytes: u64,
    pub total_uncompressed_bytes: u64,
    pub max_members: usize,
    pub max_depth: usize,
    pub max_expansion_ratio: u64,
    pub max_inventory_bytes: u64,
    pub request_timeout: Duration,
}

impl Default for ArtifactLimits {
    fn default() -> Self {
        Self {
            download_bytes: 1024 * 1024 * 1024,
            member_bytes: 2 * 1024 * 1024 * 1024,
            total_uncompressed_bytes: 4 * 1024 * 1024 * 1024,
            max_members: 10_000,
            max_depth: 4,
            max_expansion_ratio: 1000,
            max_inventory_bytes: 16 * 1024 * 1024,
            request_timeout: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawArtifact {
    pub sha256: String,
    pub byte_size: u64,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RemoteObservation {
    pub status: u16,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_length: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ArtifactMember {
    pub member_index: usize,
    pub parent_index: Option<usize>,
    /// Full nesting locator; physical content path is separately content-addressed.
    pub name: String,
    pub path: PathBuf,
    pub sha256: String,
    pub byte_size: u64,
    pub selected: bool,
    pub exclusion_reason: Option<String>,
}

#[derive(Clone)]
pub struct ArtifactService {
    root: PathBuf,
    allowed_hosts: HashSet<String>,
    limits: ArtifactLimits,
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

impl ArtifactService {
    pub fn new(root: PathBuf, allowed_hosts: Vec<String>, limits: ArtifactLimits) -> Result<Self> {
        if limits.download_bytes == 0
            || limits.member_bytes == 0
            || limits.total_uncompressed_bytes == 0
            || limits.max_members == 0
            || limits.max_expansion_ratio == 0
            || limits.max_inventory_bytes == 0
            || limits.request_timeout.is_zero()
        {
            return Err(invalid("Limites de artefato inválidos."));
        }
        fs::create_dir_all(root.join("raw"))?;
        fs::create_dir_all(root.join("imports"))?;
        fs::create_dir_all(root.join("tmp"))?;
        Ok(Self {
            root: root.canonicalize()?,
            allowed_hosts: allowed_hosts
                .into_iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            limits,
        })
    }

    /// Imports are relative to root/imports; no external filesystem paths are accepted.
    pub async fn import_local(&self, relative: &Path) -> Result<RawArtifact> {
        validate_relative(relative)?;
        let base = self.root.join("imports").canonicalize()?;
        let path = base.join(relative).canonicalize()?;
        if !path.starts_with(&base) || !path.is_file() {
            return Err(invalid("Importação fora do diretório permitido."));
        }
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            service.persist_reader(File::open(path)?, service.limits.download_bytes, true)
        })
        .await
        .map_err(|_| invalid("Falha na tarefa de importação."))?
    }

    /// Every redirect is independently validated and DNS addresses are pinned to the connection.
    pub async fn download(&self, input: &str) -> Result<RawArtifact> {
        let mut url = Url::parse(input).map_err(|_| invalid("URL de aquisição inválida."))?;
        for _ in 0..=5 {
            let addresses = self.resolve_destination(&url).await?;
            let host = url.host_str().ok_or_else(|| invalid("Host ausente."))?;
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(self.limits.request_timeout)
                .no_gzip()
                .resolve_to_addrs(host, &addresses)
                .build()
                .map_err(|_| invalid("Falha ao criar transporte HTTPS."))?;
            let response = client
                .get(url.clone())
                .send()
                .await
                .map_err(|_| invalid("Falha no transporte da fonte oficial."))?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| invalid("Redirecionamento sem destino válido."))?;
                url = url
                    .join(location)
                    .map_err(|_| invalid("Destino de redirecionamento inválido."))?;
                continue;
            }
            return self.persist_response(response).await;
        }
        Err(invalid("Quantidade de redirecionamentos excedida."))
    }

    async fn persist_response(&self, response: reqwest::Response) -> Result<RawArtifact> {
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.to_ascii_lowercase().starts_with("text/html"))
        {
            return Err(invalid("Fonte retornou HTML em vez de arquivo oficial."));
        }
        if !response.status().is_success() {
            return Err(invalid("Fonte retornou status HTTP sem conteúdo válido."));
        }
        if response
            .content_length()
            .is_some_and(|n| n > self.limits.download_bytes)
        {
            return Err(invalid("Artefato excede limite de download."));
        }
        let declared = response.content_length();
        let temp = Temporary(
            self.root
                .join("tmp")
                .join(format!("{}.part", Uuid::new_v4())),
        );
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp.0)
            .await?;
        let mut response = response;
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut prefix = Vec::with_capacity(512);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| invalid("Download interrompido."))?
        {
            size = size
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| invalid("Tamanho de download inválido."))?;
            if size > self.limits.download_bytes {
                return Err(invalid("Artefato excede limite de download."));
            }
            prefix.extend_from_slice(&chunk[..chunk.len().min(512 - prefix.len())]);
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.sync_all().await?;
        drop(file);
        if size == 0 || declared.is_some_and(|n| n != size) || looks_like_html(&prefix) {
            return Err(invalid("Resposta vazia, incompleta ou HTML de erro."));
        }
        self.finalize(&temp, hex::encode(hasher.finalize()), size)
    }

    pub fn two_step_supported(&self) -> bool {
        false
    }

    /// Lightweight discovery never falls back to GET when HEAD is unsupported.
    pub async fn probe(&self, input: &str) -> Result<RemoteObservation> {
        let mut url = Url::parse(input).map_err(|_| invalid("URL de descoberta inválida."))?;
        for _ in 0..=5 {
            let addresses = self.resolve_destination(&url).await?;
            let host = url.host_str().ok_or_else(|| invalid("Host ausente."))?;
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(self.limits.request_timeout)
                .no_gzip()
                .resolve_to_addrs(host, &addresses)
                .build()
                .map_err(|_| invalid("Falha ao criar transporte HTTPS."))?;
            let response =
                client.head(url.clone()).send().await.map_err(|_| {
                    invalid("Checagem remota inconclusiva por falha de transporte.")
                })?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| invalid("Redirecionamento sem destino válido."))?;
                url = url
                    .join(location)
                    .map_err(|_| invalid("Destino de redirecionamento inválido."))?;
                continue;
            }
            let status = response.status().as_u16();
            let header = |name| {
                response
                    .headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .filter(|v| v.len() <= 1024)
                    .map(str::to_owned)
            };
            return Ok(RemoteObservation {
                status,
                etag: header(reqwest::header::ETAG),
                last_modified: header(reqwest::header::LAST_MODIFIED),
                content_length: response
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok()),
            });
        }
        Err(invalid("Quantidade de redirecionamentos excedida."))
    }

    async fn resolve_destination(&self, url: &Url) -> Result<Vec<SocketAddr>> {
        let host = url.host_str().ok_or_else(|| invalid("Host ausente."))?;
        if url.scheme() != "https"
            || url.port_or_known_default() != Some(443)
            || !url.username().is_empty()
            || url.password().is_some()
            || !self.allowed_hosts.contains(&host.to_ascii_lowercase())
        {
            return Err(invalid("Destino HTTPS não permitido."));
        }
        let addresses: Vec<_> = tokio::net::lookup_host((host, 443))
            .await
            .map_err(|_| invalid("Falha ao resolver fonte oficial."))?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|a| !public_address(a.ip())) {
            return Err(invalid("Destino de rede não público."));
        }
        Ok(addresses)
    }

    fn persist_reader<R: Read>(
        &self,
        mut reader: R,
        limit: u64,
        validate_document: bool,
    ) -> Result<RawArtifact> {
        let temp = Temporary(
            self.root
                .join("tmp")
                .join(format!("{}.part", Uuid::new_v4())),
        );
        let mut out = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp.0)?;
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        let mut prefix = Vec::with_capacity(512);
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            size = size
                .checked_add(n as u64)
                .ok_or_else(|| invalid("Tamanho inválido."))?;
            if size > limit {
                return Err(invalid("Artefato excede limite de bytes."));
            }
            prefix.extend_from_slice(&buffer[..n.min(512 - prefix.len())]);
            hasher.update(&buffer[..n]);
            out.write_all(&buffer[..n])?;
        }
        out.sync_all()?;
        drop(out);
        if validate_document && (size == 0 || looks_like_html(&prefix)) {
            return Err(invalid("Conteúdo vazio ou HTML de erro."));
        }
        self.finalize(&temp, hex::encode(hasher.finalize()), size)
    }

    fn finalize(&self, temp: &Temporary, sha256: String, byte_size: u64) -> Result<RawArtifact> {
        let path = self.root.join("raw").join(&sha256);
        match fs::hard_link(&temp.0, &path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let (digest, size) = hash_file(&path)?;
                if digest != sha256 || size != byte_size {
                    return Err(invalid("Conteúdo raw existente foi alterado."));
                }
            }
            Err(e) => return Err(e.into()),
        }
        Ok(RawArtifact {
            sha256,
            byte_size,
            path,
        })
    }

    /// Blocking disk work: callers must use bounded spawn_blocking capacity.
    pub fn inventory(&self, artifact: &RawArtifact) -> Result<Vec<ArtifactMember>> {
        let canonical = artifact.path.canonicalize()?;
        if !canonical.starts_with(self.root.join("raw")) {
            return Err(invalid("Raw fora do armazenamento permitido."));
        }
        let (hash, size) = hash_file(&canonical)?;
        if hash != artifact.sha256 || size != artifact.byte_size {
            return Err(invalid("Integridade raw inválida."));
        }
        let mut state = InventoryState::default();
        self.walk(&artifact.path, "", None, 0, &mut state)?;
        Ok(state.members)
    }

    fn walk(
        &self,
        path: &Path,
        prefix: &str,
        parent: Option<usize>,
        depth: usize,
        state: &mut InventoryState,
    ) -> Result<()> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        let n = file.read(&mut magic)?;
        if n != 4 || &magic[..2] != b"PK" {
            if parent.is_none() {
                let (sha256, byte_size) = hash_file(path)?;
                push_inventory_member(
                    state,
                    &self.limits,
                    ArtifactMember {
                        member_index: 0,
                        parent_index: None,
                        name: "payload".into(),
                        path: path.into(),
                        sha256,
                        byte_size,
                        selected: true,
                        exclusion_reason: None,
                    },
                )?;
            }
            return Ok(());
        }
        if depth > self.limits.max_depth {
            return Err(invalid("Profundidade máxima de ZIP excedida."));
        }
        archive_bounds(
            path,
            &self.limits,
            self.limits.max_members.saturating_sub(state.members.len()),
        )?;
        let mut archive = zip::ZipArchive::new(File::open(path)?)?;
        let mut names = HashSet::new();
        // Archive entry order is source evidence; preserve it rather than guessing authority.
        for index in 0..archive.len() {
            if state.members.len() >= self.limits.max_members {
                return Err(invalid("Quantidade máxima de members excedida."));
            }
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            if name.len() > 1024 {
                return Err(invalid("Nome de member excede limite."));
            }
            validate_relative(Path::new(&name))?;
            if !names.insert(name.clone())
                || entry
                    .unix_mode()
                    .is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000 | 0o040000))
            {
                return Err(invalid("Member duplicado ou link simbólico não permitido."));
            }
            let locator = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}!/{name}")
            };
            let current = state.members.len();
            if locator.len() > 4096 || state.members.iter().any(|m| m.name == locator) {
                return Err(invalid("Localizador de member ambíguo ou excessivo."));
            }
            if entry.is_dir() {
                push_inventory_member(
                    state,
                    &self.limits,
                    ArtifactMember {
                        member_index: current,
                        parent_index: parent,
                        name: locator,
                        path: PathBuf::new(),
                        sha256: hex::encode(Sha256::digest([])),
                        byte_size: 0,
                        selected: false,
                        exclusion_reason: Some("DIRECTORY".into()),
                    },
                )?;
                continue;
            }
            if entry.size() > self.limits.member_bytes
                || entry.size()
                    > entry
                        .compressed_size()
                        .max(1)
                        .saturating_mul(self.limits.max_expansion_ratio)
            {
                return Err(invalid("Limite de expansão ou tamanho de member excedido."));
            }
            let remaining = self
                .limits
                .total_uncompressed_bytes
                .checked_sub(state.expanded_bytes)
                .ok_or_else(|| invalid("Limite total de expansão excedido."))?;
            let raw =
                self.persist_reader(&mut entry, self.limits.member_bytes.min(remaining), false)?;
            state.expanded_bytes = state
                .expanded_bytes
                .checked_add(raw.byte_size)
                .ok_or_else(|| invalid("Tamanho expandido inválido."))?;
            if raw.byte_size != entry.size() {
                return Err(invalid("Member truncado."));
            }
            let is_zip = {
                let mut f = File::open(&raw.path)?;
                let mut b = [0u8; 2];
                f.read_exact(&mut b).is_ok() && b == *b"PK"
            };
            let selected = !is_zip
                && [".txt", ".xml", ".csv", ".json"]
                    .iter()
                    .any(|ext| locator.to_ascii_lowercase().ends_with(ext));
            push_inventory_member(
                state,
                &self.limits,
                ArtifactMember {
                    member_index: current,
                    parent_index: parent,
                    name: locator.clone(),
                    path: raw.path.clone(),
                    sha256: raw.sha256,
                    byte_size: raw.byte_size,
                    selected,
                    exclusion_reason: if is_zip {
                        Some("CONTAINER".into())
                    } else if !selected {
                        Some("UNSUPPORTED_MEMBER".into())
                    } else {
                        None
                    },
                },
            )?;
            if is_zip {
                self.walk(&raw.path, &locator, Some(current), depth + 1, state)?;
            }
        }
        Ok(())
    }
}

fn push_inventory_member(
    state: &mut InventoryState,
    limits: &ArtifactLimits,
    member: ArtifactMember,
) -> Result<()> {
    let size = std::mem::size_of::<ArtifactMember>() as u64
        + member.name.len() as u64
        + member.path.as_os_str().len() as u64
        + member.sha256.len() as u64
        + member
            .exclusion_reason
            .as_ref()
            .map_or(0, |reason| reason.len() as u64);
    state.estimated_metadata_bytes = state
        .estimated_metadata_bytes
        .checked_add(size)
        .ok_or_else(|| invalid("Tamanho do inventário excedido."))?;
    if state.estimated_metadata_bytes > limits.max_inventory_bytes {
        return Err(invalid("Metadados do inventário excedem limite."));
    }
    state.members.push(member);
    Ok(())
}

#[derive(Default)]
struct InventoryState {
    expanded_bytes: u64,
    estimated_metadata_bytes: u64,
    members: Vec<ArtifactMember>,
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}
fn validate_relative(path: &Path) -> Result<()> {
    let text = path.to_string_lossy();
    if path.as_os_str().is_empty()
        || path.to_string_lossy().contains(['\\', '\0', '!'])
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || text
            .trim_end_matches('/')
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains(':'))
    {
        return Err(invalid("Caminho de member/importação inválido."));
    }
    Ok(())
}
fn looks_like_html(prefix: &[u8]) -> bool {
    let s = String::from_utf8_lossy(prefix)
        .trim_start()
        .to_ascii_lowercase();
    s.starts_with("<!doctype html") || s.starts_with("<html")
}
fn hash_file(path: &Path) -> Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        hash.update(&buf[..n]);
    }
    Ok((hex::encode(hash.finalize()), size))
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let o = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && !ip.is_multicast()
                && !ip.is_unspecified()
                && o[0] != 0
                && o[0] < 240
                && !(o[0] == 100 && (64..=127).contains(&o[1]))
                && !(o[0] == 198 && (o[1] == 18 || o[1] == 19))
                && !(o[0] == 192 && o[1] == 0 && o[2] == 0)
                && !(o[0] == 192 && o[1] == 88 && o[2] == 99)
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public_address(IpAddr::V4(v4));
            }
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
                && (s[0] & 0xfff0) != 0x3ff0
        }
    }
}

/// Check archive metadata bounds before zip allocates its central-directory inventory.
fn archive_bounds(path: &Path, limits: &ArtifactLimits, remaining_members: usize) -> Result<()> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let tail_size = size.min(65_557) as usize;
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    let mut tail = vec![0u8; tail_size];
    file.read_exact(&mut tail)?;
    let ends: Vec<_> = tail
        .windows(4)
        .enumerate()
        .filter_map(|(i, b)| {
            if b == b"PK\x05\x06"
                && i + 22 <= tail.len()
                && i + 22 + u16::from_le_bytes(tail[i + 20..i + 22].try_into().unwrap()) as usize
                    == tail.len()
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    if ends.len() != 1 {
        return Err(invalid("Diretório final ZIP ambíguo."));
    }
    let end = ends[0];
    let u16_at = |at| u16::from_le_bytes(tail[at..at + 2].try_into().unwrap());
    let u32_at = |at| u32::from_le_bytes(tail[at..at + 4].try_into().unwrap());
    if end + 22 + u16_at(end + 20) as usize != tail.len()
        || u16_at(end + 4) != 0
        || u16_at(end + 6) != 0
        || u16_at(end + 8) != u16_at(end + 10)
    {
        return Err(invalid("ZIP multipartes ou estrutura final não suportada."));
    }
    let mut count = u16_at(end + 10) as u64;
    let mut central_bytes = u32_at(end + 12) as u64;
    if count == u16::MAX as u64 || central_bytes == u32::MAX as u64 {
        if end < 20 || &tail[end - 20..end - 16] != b"PK\x06\x07" {
            return Err(invalid("Localizador ZIP64 inválido."));
        }
        let offset = u64::from_le_bytes(tail[end - 12..end - 4].try_into().unwrap());
        if offset.checked_add(56).is_none_or(|end| end > size) {
            return Err(invalid("Diretório ZIP64 fora do arquivo."));
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut header = [0u8; 56];
        file.read_exact(&mut header)?;
        if &header[..4] != b"PK\x06\x06" {
            return Err(invalid("Diretório ZIP64 inválido."));
        }
        if header[16..24] != [0u8; 8]
            || header[24..32] != header[32..40]
            || tail[end - 4..end] != 1u32.to_le_bytes()
        {
            return Err(invalid("ZIP64 multipartes não suportado."));
        }
        count = u64::from_le_bytes(header[32..40].try_into().unwrap());
        central_bytes = u64::from_le_bytes(header[40..48].try_into().unwrap());
    }
    if count > remaining_members as u64 || central_bytes > limits.max_inventory_bytes {
        return Err(invalid("Inventário ZIP excede limite de memória/entradas."));
    }
    Ok(())
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn nonpublic_and_transition_networks_are_rejected() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "192.0.0.1",
            "198.18.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
            "2001::1",
            "2002:7f00:1::1",
            "3fff::1",
        ] {
            assert!(!public_address(address.parse().unwrap()), "{address}");
        }
        assert!(public_address("8.8.8.8".parse().unwrap()));
        assert!(public_address("2606:4700::1111".parse().unwrap()));
    }

    async fn response(bytes: &'static [u8]) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).await;
            stream.write_all(bytes).await.unwrap();
            stream.shutdown().await.unwrap();
        });
        reqwest::Client::new()
            .get(format!("http://{address}/test"))
            .send()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn mocked_http_validates_body_limits_truncation_and_html() {
        let root = tempfile::tempdir().unwrap();
        let service = ArtifactService::new(
            root.path().into(),
            vec![],
            ArtifactLimits {
                download_bytes: 32,
                ..ArtifactLimits::default()
            },
        )
        .unwrap();
        let raw = service
            .persist_response(
                response(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nDATA")
                    .await,
            )
            .await
            .unwrap();
        assert_eq!(fs::read(raw.path).unwrap(), b"DATA");
        for bytes in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nDATA".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nDATA",
            b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\n<html>",
        ] {
            assert!(service
                .persist_response(response(bytes).await)
                .await
                .is_err());
        }
        assert_eq!(fs::read_dir(root.path().join("tmp")).unwrap().count(), 0);
    }
}
