use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use rust_decimal::Decimal;
use std::io::BufRead;

use super::canonical::*;
use super::xml_utils::local_name;
use crate::error::Result;
use crate::parsers::bvbg_028::ParsedOutput;

#[derive(Debug, Clone, Default)]
pub struct Bvbg186Row {
    pub trade_date: Option<NaiveDate>,
    pub ticker_symbol: Option<String>,
    pub security_id: Option<String>,
    pub market_identifier_code: Option<String>,
    pub first_price: Option<Decimal>,
    pub minimum_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub maximum_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub financial_volume_brl: Option<Decimal>,
    pub source_file_type: String,
}

pub fn parse_bvbg_186<R: BufRead>(reader: R) -> Result<ParsedOutput<Bvbg186Row>> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut rows = Vec::new();
    let mut in_record = false;
    let mut current_tag = String::new();
    let mut current_text = String::new();
    let mut current_row = Bvbg186Row {
        source_file_type: "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT".to_string(),
        ..Default::default()
    };

    loop {
        match xml_reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "equityprice" || name == "pricerecord" || name == "quote" || name == "pricrpt" {
                    in_record = true;
                    current_row = Bvbg186Row {
                        source_file_type: "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT".to_string(),
                        ..Default::default()
                    };
                } else if in_record {
                    current_tag = name;
                    current_text.clear();
                }
            }
            Event::End(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "equityprice" || name == "pricerecord" || name == "quote" || name == "pricrpt" {
                    in_record = false;
                    if current_row.trade_date.is_some()
                        && (current_row.security_id.is_some() || current_row.ticker_symbol.is_some())
                    {
                        rows.push(current_row.clone());
                    }
                    current_row = Bvbg186Row {
                        source_file_type: "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT".to_string(),
                        ..Default::default()
                    };
                } else if in_record {
                    let text = current_text.trim();
                    match current_tag.as_str() {
                        "tradedate" | "trade_date" | "date" | "traddt" | "dt" => {
                            if current_row.trade_date.is_none() {
                                current_row.trade_date = canonicalize_date(Some(text))?;
                            }
                        }
                        "tickersymbol" | "ticker_symbol" | "ticker" | "tckrsymb" => {
                            if current_row.ticker_symbol.is_none() {
                                current_row.ticker_symbol = canonicalize_ticker(Some(text));
                            }
                        }
                        "securityid" | "security_id" | "id" => {
                            if current_row.security_id.is_none() {
                                current_row.security_id = canonicalize_string(Some(text));
                            }
                        }
                        "marketidentifiercode" | "mic" | "mktidrcd" => {
                            if current_row.market_identifier_code.is_none() {
                                current_row.market_identifier_code = canonicalize_string(Some(text));
                            }
                        }
                        "firstprice" | "first_price" | "openprice" | "frstpric" => {
                            if current_row.first_price.is_none() {
                                current_row.first_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "minimumprice" | "minimum_price" | "lowprice" | "minpric" => {
                            if current_row.minimum_price.is_none() {
                                current_row.minimum_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "averageprice" | "average_price" | "tradavrgpric" => {
                            if current_row.average_price.is_none() {
                                current_row.average_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "maximumprice" | "maximum_price" | "highprice" | "maxpric" => {
                            if current_row.maximum_price.is_none() {
                                current_row.maximum_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "lastprice" | "last_price" | "closeprice" | "lastpric" => {
                            if current_row.last_price.is_none() {
                                current_row.last_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "financialvolumebrl" | "financial_volume_brl" | "ntlfinvol" => {
                            if current_row.financial_volume_brl.is_none() {
                                current_row.financial_volume_brl = canonicalize_decimal(Some(text))?;
                            }
                        }
                        _ => {}
                    }
                    current_tag.clear();
                    current_text.clear();
                }
            }
            Event::Text(ref e) => {
                if in_record {
                    current_text.push_str(&e.unescape()?);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedOutput {
        rows,
        warnings: Vec::new(),
    })
}

