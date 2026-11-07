//! COTAHIST byte-oriented streaming interpretation. Callbacks must stage, not publish.
use super::{canonical::*, cotahist::CotahistRow};
use crate::error::{AppError, Result};
use std::io::{BufRead, Read};

#[derive(Debug)]
pub enum CotahistEvent {
    Valid {
        row: Box<CotahistRow>,
        line_number: u64,
        byte_offset: u64,
    },
    Rejected {
        raw: Vec<u8>,
        reason: String,
        line_number: u64,
        byte_offset: u64,
    },
}

#[derive(Debug, Default, Clone)]
pub struct CotahistSummary {
    pub physical_records: u64,
    pub detail_records: u64,
    pub valid_records: u64,
    pub rejected_records: u64,
    pub normalized_encoding_records: u64,
}

/// At most 248 bytes are buffered even when an attacker omits line delimiters.
/// The trailer count covers detail records only; header and trailer lines are excluded.
pub fn parse_cotahist_stream<R: BufRead, F: FnMut(CotahistEvent) -> Result<()>>(
    mut reader: R,
    mut on_record: F,
) -> Result<CotahistSummary> {
    let mut summary = CotahistSummary::default();
    let mut offset = 0u64;
    let mut trailer = false;
    let mut line = Vec::with_capacity(248);
    loop {
        line.clear();
        let size = reader.by_ref().take(248).read_until(b'\n', &mut line)?;
        if size == 0 {
            break;
        }
        let start = offset;
        offset += size as u64;
        let terminated = line.ends_with(b"\n");
        if line.ends_with(b"\n") {
            line.pop();
            if line.ends_with(b"\r") {
                line.pop();
            }
        }
        if trailer {
            return Err(layout("Conteúdo após o trailer COTAHIST."));
        }
        summary.physical_records += 1;
        let line_number = summary.physical_records;
        if line_number == 1 {
            if line.len() != 245 || !line.starts_with(b"00") {
                return Err(layout("Cabeçalho COTAHIST ausente ou inválido."));
            }
            continue;
        }
        if line.starts_with(b"00") {
            return Err(layout("Cabeçalho COTAHIST duplicado."));
        }
        if line.starts_with(b"99") {
            if line.len() != 245 {
                return Err(layout("Trailer COTAHIST com largura inválida."));
            }
            let declared = std::str::from_utf8(&line[31..42])
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| layout("Contagem do trailer COTAHIST inválida."))?;
            if declared != summary.detail_records {
                return Err(layout(
                    "Contagem de registros de detalhe diverge do trailer COTAHIST.",
                ));
            }
            trailer = true;
            continue;
        }
        summary.detail_records += 1;
        if line.len() > 245 {
            if !terminated || !line.starts_with(b"01") {
                return Err(layout("Registro excede a largura máxima de 245 bytes."));
            }
            if let Some(normalized) = normalize_utf8_text_overwidth(&line) {
                line = normalized;
                summary.normalized_encoding_records += 1;
            } else {
                on_record(CotahistEvent::Rejected {
                    raw: line.clone(),
                    reason: "Registro de detalhe excede a largura máxima de 245 bytes.".into(),
                    line_number,
                    byte_offset: start,
                })?;
                summary.rejected_records += 1;
                continue;
            }
        }
        match parse_detail(&line) {
            Ok(row) => {
                on_record(CotahistEvent::Valid {
                    row: Box::new(row),
                    line_number,
                    byte_offset: start,
                })?;
                summary.valid_records += 1;
            }
            Err(error) => {
                on_record(CotahistEvent::Rejected {
                    raw: line.clone(),
                    reason: error.to_string(),
                    line_number,
                    byte_offset: start,
                })?;
                summary.rejected_records += 1;
            }
        }
    }
    if !trailer {
        return Err(layout("Trailer COTAHIST ausente."));
    }
    Ok(summary)
}

/// Recover one UTF-8 encoded Latin-1 character inside COTAHIST text columns
/// when that encoding expansion is the record's only extra byte. The source
/// artifact remains unchanged and the reconstructed record must parse fully.
fn normalize_utf8_text_overwidth(line: &[u8]) -> Option<Vec<u8>> {
    if line.len() != 246 || !line.starts_with(b"01") {
        return None;
    }
    let mut valid = None;
    // Text columns occupy bytes 28-49. Index 48 may start a character whose
    // continuation byte spills into position 50, accounting for the extra byte.
    for index in 27..=48 {
        let first = *line.get(index)?;
        let second = *line.get(index + 1)?;
        if !(0xC2..=0xC3).contains(&first) || !(0x80..=0xBF).contains(&second) {
            continue;
        }
        let latin1 = ((first & 0x1F) << 6) | (second & 0x3F);
        let mut candidate = Vec::with_capacity(245);
        candidate.extend_from_slice(&line[..index]);
        candidate.push(latin1);
        candidate.extend_from_slice(&line[index + 2..]);
        if candidate.len() != 245 || parse_detail(&candidate).is_err() {
            continue;
        }
        if valid.replace(candidate).is_some() {
            // More than one reconstruction is valid, so the source is ambiguous.
            return None;
        }
    }
    valid
}

/// Decode each fixed byte slice independently as ISO-8859-1. Unicode length never changes offsets.
fn field(line: &[u8], start: usize, end: usize) -> String {
    line[start - 1..end]
        .iter()
        .map(|b| char::from(*b))
        .collect()
}
fn parse_detail(line: &[u8]) -> Result<CotahistRow> {
    if line.len() != 245 || !line.starts_with(b"01") {
        return Err(layout("Registro de detalhe COTAHIST inválido."));
    }
    let f = |a, b| field(line, a, b);
    let row = CotahistRow {
        trade_date: canonicalize_date(Some(&f(3, 10)))?,
        bdi_code: canonicalize_string(Some(&f(11, 12))),
        ticker_symbol: canonicalize_ticker(Some(&f(13, 24))),
        market_type: canonicalize_string(Some(&f(25, 27))),
        issuer_short_name: canonicalize_string(Some(&f(28, 39))),
        paper_specification: canonicalize_string(Some(&f(40, 49))),
        term_days: canonicalize_string(Some(&f(50, 52))),
        currency: canonicalize_string(Some(&f(53, 56))),
        open_price: parse_implied_decimal(Some(&f(57, 69)), 2)?,
        high_price: parse_implied_decimal(Some(&f(70, 82)), 2)?,
        low_price: parse_implied_decimal(Some(&f(83, 95)), 2)?,
        average_price: parse_implied_decimal(Some(&f(96, 108)), 2)?,
        last_price: parse_implied_decimal(Some(&f(109, 121)), 2)?,
        best_bid_price: parse_implied_decimal(Some(&f(122, 134)), 2)?,
        best_ask_price: parse_implied_decimal(Some(&f(135, 147)), 2)?,
        trades_count: canonicalize_int(Some(&f(148, 152)))?
            .map(i32::try_from)
            .transpose()
            .map_err(|_| layout("Quantidade de negócios excede o tipo suportado."))?,
        traded_quantity: parse_implied_decimal(Some(&f(153, 170)), 0)?,
        traded_volume: parse_implied_decimal(Some(&f(171, 188)), 2)?,
        option_exercise_price: parse_implied_decimal(Some(&f(189, 201)), 2)?,
        price_correction_indicator: canonicalize_string(Some(&f(202, 202))),
        maturity_date: canonicalize_date(Some(&f(203, 210)))?,
        quotation_factor: canonicalize_int(Some(&f(211, 217)))?
            .map(i32::try_from)
            .transpose()
            .map_err(|_| layout("Fator de cotação excede o tipo suportado."))?,
        exercise_price_points: parse_implied_decimal(Some(&f(218, 230)), 6)?,
        isin: canonicalize_isin(Some(&f(231, 242))),
        distribution_number: canonicalize_string(Some(&f(243, 245))),
    };
    if row.trade_date.is_none()
        || row.ticker_symbol.is_none()
        || row.market_type.is_none()
        || row.distribution_number.is_none()
        || row.currency.is_none()
        || row.quotation_factor.is_none_or(|n| n <= 0)
    {
        return Err(layout(
            "Detalhe sem chave, moeda ou fator de cotação válido.",
        ));
    }
    Ok(row)
}
fn layout(message: &str) -> AppError {
    AppError::Layout(message.into())
}
