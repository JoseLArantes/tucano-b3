use chrono::NaiveDate;
use std::io::Read;

use super::canonical::*;
use crate::error::Result;
use crate::parsers::bvbg_028::ParsedOutput;

#[derive(Debug, Clone, Default)]
pub struct TradableSecurityRow {
    pub reference_date: NaiveDate,
    pub security_id: Option<String>,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub market_code: Option<String>,
    pub segment_code: Option<String>,
    pub tradable_indicator: Option<bool>,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
}

fn as_bool(val: Option<&str>) -> Option<bool> {
    let s = val?.trim().to_uppercase();
    match s.as_str() {
        "1" | "Y" | "YES" | "S" | "SIM" | "TRUE" => Some(true),
        "0" | "N" | "NO" | "NAO" | "FALSE" => Some(false),
        _ => None,
    }
}

pub fn parse_tradable_security_list<R: Read>(mut reader: R) -> Result<ParsedOutput<TradableSecurityRow>> {
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
        let mut row = TradableSecurityRow::default();

        for (idx, field) in record.iter().enumerate() {
            if idx >= lower_headers.len() {
                continue;
            }
            let key = &lower_headers[idx];
            let val = Some(field);

            match key.as_str() {
                "referencedate" | "date" => {
                    if let Some(d) = canonicalize_date(val)? {
                        row.reference_date = d;
                    }
                }
                "securityid" | "id" => {
                    row.security_id = canonicalize_string(val);
                }
                "tickersymbol" | "ticker" => {
                    row.ticker_symbol = canonicalize_ticker(val);
                }
                "isin" => {
                    row.isin = canonicalize_isin(val);
                }
                "marketcode" | "mkt" => {
                    row.market_code = canonicalize_string(val);
                }
                "segmentcode" | "sgmt" => {
                    row.segment_code = canonicalize_string(val);
                }
                "tradableindicator" | "tradable" | "available" => {
                    row.tradable_indicator = as_bool(val);
                }
                "tradingstartdate" | "startdate" => {
                    row.trading_start_date = canonicalize_date(val)?;
                }
                "tradingenddate" | "enddate" => {
                    row.trading_end_date = canonicalize_date(val)?;
                }
                _ => {}
            }
        }

        if row.reference_date != NaiveDate::default()
            && (row.security_id.is_some() || row.ticker_symbol.is_some())
        {
            rows.push(row);
        }
    }

    Ok(ParsedOutput {
        rows,
        warnings: Vec::new(),
    })
}

