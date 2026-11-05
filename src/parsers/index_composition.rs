use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::io::Read;

use super::canonical::*;
use crate::error::Result;
use crate::parsers::bvbg_028::ParsedOutput;

#[derive(Debug, Clone, Default)]
pub struct IndexCompositionRow {
    pub composition_date: NaiveDate,
    pub index_code: String,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub asset_name: Option<String>,
    pub theoretical_quantity: Option<Decimal>,
    pub portfolio_weight: Option<Decimal>,
    pub participation_percentage: Option<Decimal>,
}

pub fn parse_index_composition_csv<R: Read>(
    mut reader: R,
) -> Result<ParsedOutput<IndexCompositionRow>> {
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    let delimiter = if content.contains(';') {
        b';'
    } else if content.contains('\t') {
        b'\t'
    } else if content.contains('|') {
        b'|'
    } else {
        b','
    };

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers = rdr.headers()?.clone();
    let lower_headers: Vec<String> = headers
        .iter()
        .map(|h| h.trim().to_lowercase().replace('_', ""))
        .collect();

    let mut rows = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let mut row = IndexCompositionRow::default();

        for (idx, field) in record.iter().enumerate() {
            if idx >= lower_headers.len() {
                continue;
            }
            let key = &lower_headers[idx];
            let val = Some(field);

            match key.as_str() {
                "compositiondate" | "date" => {
                    if let Some(d) = canonicalize_date(val)? {
                        row.composition_date = d;
                    }
                }
                "indexcode" | "index" => {
                    if let Some(ic) = canonicalize_string(val) {
                        row.index_code = ic;
                    }
                }
                "tickersymbol" | "ticker" => {
                    row.ticker_symbol = canonicalize_ticker(val);
                }
                "isin" => {
                    row.isin = canonicalize_isin(val);
                }
                "assetname" | "name" => {
                    row.asset_name = canonicalize_string(val);
                }
                "theoreticalquantity" | "quantity" => {
                    row.theoretical_quantity = canonicalize_decimal(val)?;
                }
                "portfolioweight" | "weight" => {
                    row.portfolio_weight = canonicalize_decimal(val)?;
                }
                "participationpercentage" | "participation" => {
                    row.participation_percentage = canonicalize_decimal(val)?;
                }
                _ => {}
            }
        }

        if row.composition_date != NaiveDate::default() && !row.index_code.is_empty() {
            rows.push(row);
        }
    }

    Ok(ParsedOutput {
        rows,
        warnings: Vec::new(),
    })
}
