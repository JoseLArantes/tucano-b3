use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::canonical::*;
use crate::error::{AppError, Result};
use crate::parsers::bvbg_028::ParsedOutput;

#[derive(Debug, Clone, Default)]
pub struct CotahistRow {
    pub trade_date: Option<NaiveDate>,
    pub bdi_code: Option<String>,
    pub ticker_symbol: Option<String>,
    pub market_type: Option<String>,
    pub issuer_short_name: Option<String>,
    pub paper_specification: Option<String>,
    pub term_days: Option<String>,
    pub currency: Option<String>,
    pub open_price: Option<Decimal>,
    pub high_price: Option<Decimal>,
    pub low_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub best_bid_price: Option<Decimal>,
    pub best_ask_price: Option<Decimal>,
    pub trades_count: Option<i32>,
    pub traded_quantity: Option<Decimal>,
    pub traded_volume: Option<Decimal>,
    pub option_exercise_price: Option<Decimal>,
    pub price_correction_indicator: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub quotation_factor: Option<i32>,
    pub exercise_price_points: Option<Decimal>,
    pub isin: Option<String>,
    pub distribution_number: Option<String>,
}

fn slice_1based(line: &str, start: usize, end: usize) -> &str {
    if line.len() >= end {
        &line[start - 1..end]
    } else if line.len() >= start {
        &line[start - 1..]
    } else {
        ""
    }
}

pub fn parse_cotahist(content: &str) -> Result<ParsedOutput<CotahistRow>> {
    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    let mut header_count = 0;
    let mut trailer_count = 0;
    let mut declared_count: Option<usize> = None;

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        if line.len() != 245 {
            return Err(AppError::Layout(format!(
                "Linha {line_num} com tamanho invalido: {}",
                line.len()
            )));
        }

        let record_type = slice_1based(line, 1, 2);
        if record_type == "00" {
            header_count += 1;
            continue;
        }
        if record_type == "99" {
            trailer_count += 1;
            let trailer_val = canonicalize_string(Some(slice_1based(line, 32, 42)));
            declared_count = trailer_val.and_then(|v| v.parse::<usize>().ok());
            continue;
        }
        if record_type != "01" {
            warnings.push(format!(
                "Linha {line_num}: tipo de registro invalido {record_type}"
            ));
            continue;
        }

        let row = CotahistRow {
            trade_date: canonicalize_date(Some(slice_1based(line, 3, 10)))?,
            bdi_code: canonicalize_string(Some(slice_1based(line, 11, 12))),
            ticker_symbol: canonicalize_ticker(Some(slice_1based(line, 13, 24))),
            market_type: canonicalize_string(Some(slice_1based(line, 25, 27))),
            issuer_short_name: canonicalize_string(Some(slice_1based(line, 28, 39))),
            paper_specification: canonicalize_string(Some(slice_1based(line, 40, 49))),
            term_days: canonicalize_string(Some(slice_1based(line, 50, 52))),
            currency: canonicalize_string(Some(slice_1based(line, 53, 56))),
            open_price: parse_implied_decimal(Some(slice_1based(line, 57, 69)), 2)?,
            high_price: parse_implied_decimal(Some(slice_1based(line, 70, 82)), 2)?,
            low_price: parse_implied_decimal(Some(slice_1based(line, 83, 95)), 2)?,
            average_price: parse_implied_decimal(Some(slice_1based(line, 96, 108)), 2)?,
            last_price: parse_implied_decimal(Some(slice_1based(line, 109, 121)), 2)?,
            best_bid_price: parse_implied_decimal(Some(slice_1based(line, 122, 134)), 2)?,
            best_ask_price: parse_implied_decimal(Some(slice_1based(line, 135, 147)), 2)?,
            trades_count: canonicalize_int(Some(slice_1based(line, 148, 152)))?.map(|v| v as i32),
            traded_quantity: parse_implied_decimal(Some(slice_1based(line, 153, 170)), 0)?,
            traded_volume: parse_implied_decimal(Some(slice_1based(line, 171, 188)), 2)?,
            option_exercise_price: parse_implied_decimal(Some(slice_1based(line, 189, 201)), 2)?,
            price_correction_indicator: canonicalize_string(Some(slice_1based(line, 202, 202))),
            maturity_date: canonicalize_date(Some(slice_1based(line, 203, 210)))?,
            quotation_factor: canonicalize_int(Some(slice_1based(line, 211, 217)))?
                .map(|v| v as i32),
            exercise_price_points: parse_implied_decimal(Some(slice_1based(line, 218, 230)), 6)?,
            isin: canonicalize_isin(Some(slice_1based(line, 231, 242))),
            distribution_number: canonicalize_string(Some(slice_1based(line, 243, 245))),
        };
        rows.push(row);
    }

    if header_count != 1 {
        warnings.push("Arquivo sem cabecalho valido (tipo 00 unico).".to_string());
    }
    if trailer_count != 1 {
        warnings.push("Arquivo sem trailer valido (tipo 99 unico).".to_string());
    }
    if let Some(dec) = declared_count {
        // Accepts both conventions: count of quotes or count of all lines (quotes + header + trailer)
        if dec != rows.len() && dec != rows.len() + 2 {
            warnings.push(format!(
                "Trailer informa {dec} registros, mas foram lidos {}.",
                rows.len()
            ));
        }
    }

    Ok(ParsedOutput { rows, warnings })
}
