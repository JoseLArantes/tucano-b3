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
    pub raw_ticker: String,
    pub bdi_code: Option<String>,
    pub market_code: String,
    pub security_specification: Option<String>,
    pub currency_code: Option<String>,
    pub quotation_factor: Option<u32>,
    pub pre_ult_raw: Option<u64>,
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
    /// Exact COTAHIST VOLTOT in cents, available only for an 18-digit source field.
    pub turnover_cents: Option<u64>,
    /// One-based physical line number in the original member.
    pub source_line: usize,
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
            let bytes_read = buf_reader
                .read_until(b'\n', &mut raw_bytes)
                .map_err(CotahistParseError::Io)?;

            if bytes_read == 0 {
                break;
            }

            line_num += 1;
            stats.lines_total += 1;

            let mut line = raw_bytes.as_slice();
            while line
                .last()
                .is_some_and(|byte| *byte == b'\r' || *byte == b'\n')
            {
                line = &line[..line.len() - 1];
            }

            // Skip empty lines
            if line.iter().all(u8::is_ascii_whitespace) {
                stats.lines_skipped += 1;
                continue;
            }

            // Parse the line
            match Self::parse_line_bytes(line, line_num) {
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
        Self::parse_line_bytes(line.as_bytes(), line_num)
    }

    fn parse_line_bytes(
        input: &[u8],
        line_num: usize,
    ) -> Result<Option<CotahistRecord>, CotahistParseError> {
        let mut line = input;
        while line
            .last()
            .is_some_and(|byte| *byte == b'\r' || *byte == b'\n')
        {
            line = &line[..line.len() - 1];
        }
        // Minimum length check
        if line.len() < 11 {
            return Err(CotahistParseError::WrongLineLength {
                line: line_num,
                actual: line.len(),
            });
        }

        // Record type (bytes 0-1)
        let tipo_reg = field_ascii(line, 0, 2);
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
        let date_str = field_ascii(line, 2, 10).trim();
        let date = Self::parse_date(date_str, line_num)?;

        let bdi_code = Some(field_ascii(line, 10, 12).trim().to_string());

        // CODNEG (bytes 12-24) — ticker
        let raw_ticker = field_ascii(line, 12, 24).trim().to_string();
        let ticker = Self::normalize_ticker(&raw_ticker);

        // TPMERC (bytes 24-27) — market type
        let market_code = field_ascii(line, 24, 27).trim().to_string();
        let market_type = market_code.parse::<u8>().ok();

        // NOMRES (bytes 27-39) — company name (LATIN1)
        let company_name = {
            let raw = &line[27..39];
            let name = decode_latin1(raw);
            let name = name.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        };

        let security_specification = {
            let value = field_ascii(line, 39, 49).trim();
            (!value.is_empty()).then(|| value.to_string())
        };

        let currency_code = Self::normalize_currency(field_ascii(line, 52, 56));

        let quotation_factor = field_ascii(line, 210, 217).trim().parse::<u32>().ok();

        // COTAHIST prices use two implied decimal places. FATCOT is the
        // quotation unit (e.g. 1 share or a lot of 1,000), not a decimal scale.
        let pre_abr =
            Self::parse_implicit_decimal(field_ascii(line, 56, 69), 2, line_num, "PREABR")?;

        // PREMAX (bytes 69-82) — max price
        let pre_max =
            Self::parse_implicit_decimal(field_ascii(line, 69, 82), 2, line_num, "PREMAX")?;

        // PREMIN (bytes 82-95) — min price
        let pre_min =
            Self::parse_implicit_decimal(field_ascii(line, 82, 95), 2, line_num, "PREMIN")?;

        // PREMED (bytes 95-108) — average price
        let pre_med =
            Self::parse_implicit_decimal(field_ascii(line, 95, 108), 2, line_num, "PREMED")?;

        // PREULT (bytes 108-121) — last/closing price
        let pre_ult_raw = field_ascii(line, 108, 121).trim().parse::<u64>().ok();
        let pre_ult =
            Self::parse_implicit_decimal(field_ascii(line, 108, 121), 2, line_num, "PREULT")?;

        // PREOFC (bytes 121-134) — buy offer price
        let pre_ofe_compra =
            Self::parse_implicit_decimal(field_ascii(line, 121, 134), 2, line_num, "PREOFC")?;

        // PREOFV (bytes 134-147) — sell offer price
        let pre_ofe_venda =
            Self::parse_implicit_decimal(field_ascii(line, 134, 147), 2, line_num, "PREOFV")?;

        // TOTNEG (bytes 147-152) — trade count
        let tot_neg = field_ascii(line, 147, 152).trim().parse::<u32>().ok();

        // QUATOT (bytes 152-170) — volume (shares)
        let quat_tot = field_ascii(line, 152, 170).trim().parse::<u64>().ok();

        // VOLTOT (bytes 170-188) — volume (BRL)
        let turnover_field = field_ascii(line, 170, 188);
        let turnover_cents = (turnover_field.len() == 18
            && turnover_field.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| turnover_field.parse::<u64>().ok())
        .flatten();
        let vol_tot = Self::parse_implicit_decimal(turnover_field, 2, line_num, "VOLTOT")?;

        // CODISI (bytes 231-242) — ISIN code or B3 internal security code.
        let isin = {
            let raw = if line.len() >= 242 {
                field_ascii(line, 230, 242)
            } else {
                field_ascii(&[], 0, 0)
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
            raw_ticker,
            bdi_code,
            market_code,
            security_specification,
            currency_code,
            quotation_factor,
            pre_ult_raw,
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
            turnover_cents,
            source_line: line_num,
        }))
    }

    fn normalize_currency(value: &str) -> Option<String> {
        let raw = value.trim();
        if raw.is_empty() {
            return None;
        }
        match raw {
            "R$" => Some("BRL".to_string()),
            "US$" => Some("USD".to_string()),
            _ => Some(raw.to_string()),
        }
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

fn field_ascii(line: &[u8], start: usize, end: usize) -> &str {
    line.get(start..end)
        .and_then(|field| std::str::from_utf8(field).ok())
        .unwrap_or("")
}

fn decode_latin1(field: &[u8]) -> String {
    field.iter().map(|byte| char::from(*byte)).collect()
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
