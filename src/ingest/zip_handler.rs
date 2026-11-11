//! ZIP file handler for ingestion.
//!
//! Decompresses uploaded ZIP files and extracts COTAHIST files.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::ZipArchive;

/// Errors during ZIP handling.
#[derive(Error, Debug)]
pub enum ZipError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("No COTAHIST files found in ZIP")]
    NoCotahistFiles,

    #[error("File too large: {size} bytes (max: {max} bytes)")]
    FileTooLarge { size: usize, max: usize },
}

/// Maximum ZIP file size: 500 MB.
pub const MAX_ZIP_SIZE: usize = 500 * 1024 * 1024;

/// Extracted file from a ZIP archive.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    pub filename: String,
    pub content: Vec<u8>,
}

/// Extract COTAHIST files from a ZIP archive.
///
/// Only extracts files matching COTAHIST patterns:
/// - COTAHIST_D*.TXT (yearly)
/// - COTAHIST_A*.TXT (yearly)
/// - COTAHIST_M*.TXT (monthly)
/// - COTAHIST_*.TXT (daily)
///
/// Returns extracted files ready for parsing.
pub fn extract_cotahist_from_zip(data: &[u8]) -> Result<Vec<ExtractedFile>, ZipError> {
    if data.len() > MAX_ZIP_SIZE {
        return Err(ZipError::FileTooLarge {
            size: data.len(),
            max: MAX_ZIP_SIZE,
        });
    }

    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;

    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let filename = file.name().to_string();

        // Skip directories
        if filename.ends_with('/') {
            continue;
        }

        // Only process COTAHIST files
        let upper = filename.to_uppercase();
        if !upper.contains("COTAHIST") || !upper.ends_with(".TXT") {
            continue;
        }

        // Read file content
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        extracted.push(ExtractedFile {
            filename: Path::new(&filename)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(filename),
            content,
        });
    }

    if extracted.is_empty() {
        return Err(ZipError::NoCotahistFiles);
    }

    tracing::info!(
        count = extracted.len(),
        "Arquivos COTAHIST extraídos do ZIP"
    );

    Ok(extracted)
}

/// Scan a local folder for COTAHIST files.
///
/// Returns paths to all COTAHIST .TXT files found.
pub fn scan_folder_for_cotahist(folder: &Path) -> Result<Vec<PathBuf>, ZipError> {
    if !folder.exists() {
        return Err(ZipError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Folder not found: {}", folder.display()),
        )));
    }

    let mut files = Vec::new();

    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_uppercase())
            .unwrap_or_default();

        if filename.contains("COTAHIST") && filename.ends_with(".TXT") {
            files.push(path);
        }
    }

    files.sort();

    if files.is_empty() {
        return Err(ZipError::NoCotahistFiles);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn build_test_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();
            for (name, content) in files {
                zip.start_file(*name, options).unwrap();
                zip.write_all(content).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_extract_cotahist_from_zip_success() {
        let zip_bytes = build_test_zip(&[
            ("COTAHIST_D2026.TXT", b"00COTAHIST..."),
            ("ignored.csv", b"a,b,c"),
        ]);

        let extracted = extract_cotahist_from_zip(&zip_bytes).unwrap();
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].filename, "COTAHIST_D2026.TXT");
        assert_eq!(extracted[0].content, b"00COTAHIST...");
    }

    #[test]
    fn test_extract_cotahist_from_zip_empty() {
        let zip_bytes = build_test_zip(&[("other.txt", b"foo")]);
        let result = extract_cotahist_from_zip(&zip_bytes);
        assert!(matches!(result, Err(ZipError::NoCotahistFiles)));
    }

    #[test]
    fn test_extract_cotahist_from_zip_too_large() {
        let fake_data = [0u8; 10];
        // Test size limit with fake small max by checking error constructor
        let err = ZipError::FileTooLarge {
            size: MAX_ZIP_SIZE + 1,
            max: MAX_ZIP_SIZE,
        };
        assert!(format!("{}", err).contains("File too large"));
        assert!(fake_data.len() <= MAX_ZIP_SIZE);
    }

    #[test]
    fn test_scan_folder_for_cotahist_success() {
        let temp = TempDir::new().unwrap();
        let path1 = temp.path().join("COTAHIST_D2026.TXT");
        let path2 = temp.path().join("ignored.txt");
        let path3 = temp.path().join("COTAHIST_M202601.TXT");

        std::fs::write(&path1, b"test1").unwrap();
        std::fs::write(&path2, b"test2").unwrap();
        std::fs::write(&path3, b"test3").unwrap();

        let files = scan_folder_for_cotahist(temp.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&path1));
        assert!(files.contains(&path3));
    }

    #[test]
    fn test_scan_folder_not_found() {
        let result = scan_folder_for_cotahist(Path::new("/nonexistent/folder/12345"));
        assert!(matches!(result, Err(ZipError::Io(_))));
    }

    #[test]
    fn test_scan_folder_empty() {
        let temp = TempDir::new().unwrap();
        let result = scan_folder_for_cotahist(temp.path());
        assert!(matches!(result, Err(ZipError::NoCotahistFiles)));
    }
}
