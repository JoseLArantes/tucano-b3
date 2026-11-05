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
pub struct Bvbg087Row {
    pub trade_date: Option<NaiveDate>,
    pub index_code: Option<String>,
    pub index_name: Option<String>,
    pub value_type: Option<String>,
    pub value: Option<Decimal>,
    pub currency: Option<String>,
    pub market_identifier_code: Option<String>,
}

pub fn parse_bvbg_087<R: BufRead>(reader: R) -> Result<ParsedOutput<Bvbg087Row>> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut rows = Vec::new();
    let mut in_record = false;
    let mut current_tag = String::new();
    let mut current_text = String::new();
    let mut current_row = Bvbg087Row::default();

    loop {
        match xml_reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "indexvalue" || name == "indexrecord" || name == "index" || name == "indxinf" || name == "iopvinf" || name == "bdrinf" {
                    in_record = true;
                    current_row = Bvbg087Row::default();
                    if name == "iopvinf" {
                        current_row.value_type = Some("IOPV".to_string());
                    } else if name == "bdrinf" {
                        current_row.value_type = Some("BDR_REFERENCE_PRICE".to_string());
                    }
                } else if in_record {
                    current_tag = name;
                    current_text.clear();
                }
            }
            Event::End(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "indexvalue" || name == "indexrecord" || name == "index" || name == "indxinf" || name == "iopvinf" || name == "bdrinf" {
                    in_record = false;
                    if current_row.trade_date.is_some()
                        && current_row.index_code.is_some()
                        && current_row.value_type.is_some()
                    {
                        rows.push(current_row.clone());
                    }
                    current_row = Bvbg087Row::default();
                } else if in_record {
                    let text = current_text.trim();
                    match current_tag.as_str() {
                        "tradedate" | "trade_date" | "date" | "traddt" | "dt" => {
                            if current_row.trade_date.is_none() {
                                current_row.trade_date = canonicalize_date(Some(text))?;
                            }
                        }
                        "indexcode" | "index_code" | "symbol" | "tckrsymb" => {
                            if current_row.index_code.is_none() {
                                current_row.index_code = canonicalize_string(Some(text));
                            }
                        }
                        "indexname" | "index_name" | "name" | "sctydesc" => {
                            if current_row.index_name.is_none() {
                                current_row.index_name = canonicalize_string(Some(text));
                            }
                        }
                        "valuetype" | "value_type" => {
                            if current_row.value_type.is_none() {
                                current_row.value_type = canonicalize_string(Some(text));
                            }
                        }
                        "value" | "clsgpric" | "val" | "refpric" => {
                            if current_row.value.is_none() {
                                current_row.value = canonicalize_decimal(Some(text))?;
                            }
                            if current_row.value_type.is_none() {
                                current_row.value_type = Some("index_value".to_string());
                            }
                        }
                        "currency" | "ccy" => {
                            if current_row.currency.is_none() {
                                current_row.currency = canonicalize_string(Some(text));
                            }
                        }
                        "marketidentifiercode" | "mic" | "mktidrcd" => {
                            if current_row.market_identifier_code.is_none() {
                                current_row.market_identifier_code = canonicalize_string(Some(text));
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

