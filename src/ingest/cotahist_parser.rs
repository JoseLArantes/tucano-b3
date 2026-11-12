//! COTAHIST fixed-width parser.
//!
//! Parses B3 COTAHIST files (LATIN1, fixed-width, 245-249 bytes per line).
//! Supports all 3 known schema versions.
//!
//! Reference: B3 "Layout do Arquivo COTAHIST" documentation.

use chrono::{Datelike, NaiveDate};
use std::io::{BufRead, BufReader, Read};
use thiserror::Error;

/// COTAHIST record types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    Header,  // TIPOREG = "00"
    Quote,   // TIPOREG = "01"
    Trailer, // TIPOREG = "02"
}

/// A parsed COTAHIST quote record.
#[derive(Debug, Clone, PartialEq)]
pub struct CotahistRecord {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub date: NaiveDate,
    pub year: i32,
    pub market_type: Option<u8>,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: Option<u32>,
    pub quat_tot: Option<u64>,
    pub vol_tot: Option<f64>,
}

/// Errors during COTAHIST parsing.
#[derive(Error, Debug)]
pub enum CotahistParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid line length at line {line}: expected 245-249, got {actual}")]
    WrongLineLength { line: usize, actual: usize },

    #[error("Unknown record type at line {line}: {tipo}")]
    UnknownRecordType { line: usize, tipo: String },

    #[error("Invalid date at line {line}: {value}")]
    InvalidDate { line: usize, value: String },

    #[error("Invalid decimal at line {line}, field {field}: {value}")]
    InvalidDecimal {
        line: usize,
        field: String,
        value: String,
    },

    #[error("Encoding error at line {line}: {message}")]
    EncodingError { line: usize, message: String },
}

/// Statistics from a parse operation.
#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub lines_total: usize,
    pub lines_parsed: usize,
    pub lines_skipped: usize,
    pub lines_errored: usize,
    pub tickers_seen: std::collections::HashSet<String>,
    pub date_min: Option<NaiveDate>,
    pub date_max: Option<NaiveDate>,
}

/// COTAHIST parser.
///
/// Streaming: reads line-by-line, never loads entire file into memory.
pub struct CotahistParser;

impl CotahistParser {
    /// Parse a COTAHIST file from any Read source into a `Vec`.
    ///
    /// Lines with errors are skipped (logged) rather than aborting.
    pub fn parse<R: Read>(
        reader: R,
    ) -> Result<(Vec<CotahistRecord>, ParseStats), CotahistParseError> {
        let mut records = Vec::new();
        let stats = Self::parse_stream(reader, |record| -> Result<(), CotahistParseError> {
            records.push(record);
            Ok(())
        })?;
        Ok((records, stats))
    }

    /// Stream-parse a COTAHIST file from any `Read` source, invoking `on_record`
    /// for each valid quote record without buffering the entire file in memory.
    pub fn parse_stream<R, F, E>(reader: R, mut on_record: F) -> Result<ParseStats, E>
    where
        R: Read,
        F: FnMut(CotahistRecord) -> Result<(), E>,
        E: From<CotahistParseError>,
    {
        let mut buf_reader = BufReader::with_capacity(256 * 1024, reader); // 256 KB buffer
        let mut stats = ParseStats::default();
        let mut raw_bytes = Vec::with_capacity(512);
        let mut line_num = 0;

        loop {
            raw_bytes.clear();
            let bytes_read = match buf_reader.read_until(b'\n', &mut raw_bytes) {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!(line = line_num + 1, error = %e, "Erro ao ler linha");
                    stats.lines_errored += 1;
                    break;
                }
            };

            if bytes_read == 0 {
                break;
            }

            line_num += 1;
            stats.lines_total += 1;

            // Safe conversion supporting UTF-8 and ISO-8859-1 (Latin-1) without allocating for UTF-8
            let line: std::borrow::Cow<'_, str> = match std::str::from_utf8(&raw_bytes) {
                Ok(s) => std::borrow::Cow::Borrowed(s.trim_end_matches(['\r', '\n'])),
                Err(_) => {
                    let s: String = raw_bytes.iter().map(|&b| b as char).collect();
                    std::borrow::Cow::Owned(s.trim_end_matches(['\r', '\n']).to_string())
                }
            };

            // Skip empty lines
            if line.trim().is_empty() {
                stats.lines_skipped += 1;
                continue;
            }

            // Parse the line
            match Self::parse_line(&line, line_num) {
                Ok(Some(record)) => {
                    if !stats.tickers_seen.contains(&record.ticker) {
                        stats.tickers_seen.insert(record.ticker.clone());
                    }
                    stats.date_min =
                        Some(stats.date_min.map_or(record.date, |d| d.min(record.date)));
                    stats.date_max =
                        Some(stats.date_max.map_or(record.date, |d| d.max(record.date)));
                    stats.lines_parsed += 1;
                    on_record(record)?;
                }
                Ok(None) => {
                    // Header or trailer — skip
                    stats.lines_skipped += 1;
                }
                Err(e) => {
                    tracing::warn!(line = line_num, error = %e, "Erro ao parsear linha");
                    stats.lines_errored += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Parse a single COTAHIST line.
    ///
    /// Returns:
    /// - Ok(Some(record)) for quote lines (TIPOREG = "01")
    /// - Ok(None) for header/trailer lines
    /// - Err for malformed lines
    pub fn parse_line(
        line: &str,
        line_num: usize,
    ) -> Result<Option<CotahistRecord>, CotahistParseError> {
        // Minimum length check
        if line.len() < 11 {
            return Err(CotahistParseError::WrongLineLength {
                line: line_num,
                actual: line.len(),
            });
        }

        // Record type (bytes 0-1)
        let tipo_reg = &line[0..2];
        // B3 trailers are commonly "99"; some layouts also document "02".
        let record_type = match tipo_reg {
            "00" => RecordType::Header,
            "01" => RecordType::Quote,
            "02" | "99" => RecordType::Trailer,
            _ => {
                return Err(CotahistParseError::UnknownRecordType {
                    line: line_num,
                    tipo: tipo_reg.to_string(),
                });
            }
        };

        // Only parse quote records
        if record_type != RecordType::Quote {
            return Ok(None);
        }

        // Full line length validation (at least 245 bytes for COTAHIST layout)
        if line.len() < 245 {
            return Err(CotahistParseError::WrongLineLength {
                line: line_num,
                actual: line.len(),
            });
        }

        // === Parse fields by position ===
        // Reference: B3 COTAHIST layout (V3, 2005+)

        // DATA_PREGAO (bytes 2-9, YYYYMMDD)
        let date_str = line[2..10].trim();
        let date = Self::parse_date(date_str, line_num)?;

        // CODBDI (bytes 10-12) — skip

        // CODNEG (bytes 12-24) — ticker
        let raw_ticker = line[12..24].trim();
        let ticker = Self::normalize_ticker(raw_ticker);

        // TPMERC (bytes 24-27) — market type
        let market_type = line[24..27].trim().parse::<u8>().ok();

        // NOMRES (bytes 27-39) — company name (LATIN1)
        let company_name = {
            let raw = &line[27..39];
            let name = raw.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        };

        // ESPECI (bytes 39-49) — specification (ON/PN/UNT)

        // FAT_COTACAO (bytes 52-53) — decimal factor
        let fat_cotacao: u8 = line
            .get(52..53)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(2);

        // PREABR (bytes 56-69) — opening price
        let pre_abr = Self::parse_implicit_decimal(&line[56..69], fat_cotacao, line_num, "PREABR")?;

        // PREMAX (bytes 69-82) — max price
        let pre_max = Self::parse_implicit_decimal(&line[69..82], fat_cotacao, line_num, "PREMAX")?;

        // PREMIN (bytes 82-95) — min price
        let pre_min = Self::parse_implicit_decimal(&line[82..95], fat_cotacao, line_num, "PREMIN")?;

        // PREMED (bytes 95-108) — average price
        let pre_med =
            Self::parse_implicit_decimal(&line[95..108], fat_cotacao, line_num, "PREMED")?;

        // PREULT (bytes 108-121) — last/closing price
        let pre_ult =
            Self::parse_implicit_decimal(&line[108..121], fat_cotacao, line_num, "PREULT")?;

        // PREOFC (bytes 121-134) — buy offer price
        let pre_ofe_compra =
            Self::parse_implicit_decimal(&line[121..134], fat_cotacao, line_num, "PREOFC")?;

        // PREOFV (bytes 134-147) — sell offer price
        let pre_ofe_venda =
            Self::parse_implicit_decimal(&line[134..147], fat_cotacao, line_num, "PREOFV")?;

        // TOTNEG (bytes 147-152) — trade count
        let tot_neg = line[147..152].trim().parse::<u32>().ok();

        // QUATOT (bytes 152-170) — volume (shares)
        let quat_tot = line[152..170].trim().parse::<u64>().ok();

        // VOLTOT (bytes 170-188) — volume (BRL)
        let vol_tot = Self::parse_implicit_decimal(&line[170..188], 0, line_num, "VOLTOT")?;

        // CODISIN (bytes 191-203) — ISIN code
        let isin = {
            let raw = if line.len() >= 203 {
                &line[191..203]
            } else {
                ""
            };
            let code = raw.trim();
            if code.is_empty() {
                None
            } else {
                Some(code.to_string())
            }
        };

        Ok(Some(CotahistRecord {
            ticker,
            isin,
            company_name,
            date,
            year: date.year(),
            market_type,
            pre_abr,
            pre_max,
            pre_min,
            pre_med,
            pre_ult,
            pre_ofe_compra,
            pre_ofe_venda,
            tot_neg,
            quat_tot,
            vol_tot,
        }))
    }

    /// Parse date from YYYYMMDD format.
    fn parse_date(s: &str, line_num: usize) -> Result<NaiveDate, CotahistParseError> {
        NaiveDate::parse_from_str(s, "%Y%m%d").map_err(|_| CotahistParseError::InvalidDate {
            line: line_num,
            value: s.to_string(),
        })
    }

    /// Parse implicit decimal field.
    ///
    /// COTAHIST stores prices as integers with implied decimal places.
    /// The decimal factor determines where the decimal point goes.
    /// Example: "0000000001234" with factor 2 → 12.34
    pub fn parse_implicit_decimal(
        s: &str,
        factor: u8,
        line_num: usize,
        field_name: &str,
    ) -> Result<Option<f64>, CotahistParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.chars().all(|c| c == '0') {
            return Ok(Some(0.0));
        }

        let raw: u64 = trimmed
            .parse()
            .map_err(|_| CotahistParseError::InvalidDecimal {
                line: line_num,
                field: field_name.to_string(),
                value: trimmed.to_string(),
            })?;

        let value = raw as f64 / 10f64.powi(factor as i32);
        Ok(Some(value))
    }

    /// Normalize ticker by stripping market-type suffixes.
    ///
    /// Examples:
    /// - "PETR4" → "PETR4"
    /// - "PETR4F" → "PETR4" (futures reference)
    /// - "VALE3 " → "VALE3" (trailing space)
    pub fn normalize_ticker(raw: &str) -> String {
        let trimmed = raw.trim().to_uppercase();
        // Strip trailing 'F' (futures reference) or 'A' (old format)
        if trimmed.len() > 4 && (trimmed.ends_with('F') || trimmed.ends_with('A')) {
            trimmed[..trimmed.len() - 1].to_string()
        } else {
            trimmed
        }
    }
}

/// Detect COTAHIST file type from filename.
///
/// Returns: "yearly", "monthly", or "daily"
pub fn detect_file_type(filename: &str) -> &'static str {
    let upper = filename.to_uppercase();
    let name = upper.rsplit(['/', '\\']).next().unwrap_or(&upper);

    if name.contains("COTAHIST_D") || name.contains("COTAHIST_A") {
        let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 4 {
            return "yearly";
        } else if digits.len() == 8 {
            return "daily";
        } else if name.len() <= 25 {
            return "yearly";
        }
    }

    if name.contains("COTAHIST_M") {
        "monthly"
    } else if name.contains("COTAHIST_") {
        "daily"
    } else {
        "unknown"
    }
}
