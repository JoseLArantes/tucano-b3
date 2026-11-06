pub mod bvbg_028;
pub mod bvbg_086;
pub mod bvbg_087;
pub mod bvbg_186;
pub mod canonical;
pub mod cotahist;
pub mod cotahist_stream;
pub mod index_composition;
pub mod registry_stream;
pub mod security_definition;
pub mod tradable_list;
pub mod xml_utils;

use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use zip::ZipArchive;

use crate::db::enums::SourceFileType;
use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct ExtractedMember {
    pub name: String,
    pub content: Vec<u8>,
    pub sha256: String,
    pub byte_size: i64,
}

pub fn extract_archive_members(
    source_file_type: SourceFileType,
    raw_content: &[u8],
) -> Result<Vec<ExtractedMember>> {
    let mut members = Vec::new();
    let preferred_extensions = match source_file_type {
        SourceFileType::TradableSecurityList => vec![".csv", ".txt"],
        SourceFileType::CotahistYearly
        | SourceFileType::CotahistMonthly
        | SourceFileType::CotahistDaily => vec![".txt"],
        SourceFileType::B3IndexPortfolioComposition => vec![".csv", ".txt"],
        _ => vec![".xml", ".txt", ".csv"],
    };

    if raw_content.len() >= 4 && &raw_content[..2] == b"PK" {
        extract_zip_recursive(raw_content, &preferred_extensions, &mut members, 0)?;
    } else {
        let mut hasher = Sha256::new();
        hasher.update(raw_content);
        let sha256 = hex::encode(hasher.finalize());
        members.push(ExtractedMember {
            name: source_file_type.as_str().to_string(),
            content: raw_content.to_vec(),
            sha256,
            byte_size: raw_content.len() as i64,
        });
    }

    if members.is_empty() {
        return Err(AppError::Layout(
            "Nenhum arquivo valido encontrado no arquivo B3.".to_string(),
        ));
    }

    members.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(members)
}

fn extract_zip_recursive(
    bytes: &[u8],
    preferred_extensions: &[&str],
    members: &mut Vec<ExtractedMember>,
    depth: usize,
) -> Result<()> {
    if depth > 4 {
        return Err(AppError::Layout(
            "Profundidade máxima de descompactação recursiva de ZIP (4) excedida.".to_string(),
        ));
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if file.is_dir() || name.ends_with('/') {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        if buf.len() >= 4 && &buf[..2] == b"PK" {
            extract_zip_recursive(&buf, preferred_extensions, members, depth + 1)?;
            continue;
        }

        let lower_name = name.to_lowercase();
        let matches = preferred_extensions
            .iter()
            .any(|ext| lower_name.ends_with(ext));

        if matches {
            let mut hasher = Sha256::new();
            hasher.update(&buf);
            let sha256 = hex::encode(hasher.finalize());
            let byte_size = buf.len() as i64;

            members.push(ExtractedMember {
                name,
                content: buf,
                sha256,
                byte_size,
            });
        }
    }

    Ok(())
}
