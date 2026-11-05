use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::BufRead;

use super::canonical::*;
use super::xml_utils::local_name;
use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct Bvbg028Row {
    pub security_id: String,
    pub security_source: Option<String>,
    pub market_identifier_code: Option<String>,
    pub asset: Option<String>,
    pub asset_description: Option<String>,
    pub market_code: Option<String>,
    pub market_name: Option<String>,
    pub segment_code: Option<String>,
    pub segment_name: Option<String>,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub cfi_code: Option<String>,
    pub security_category: Option<String>,
    pub instrument_status: Option<String>,
    pub instrument_type: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub reference_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedOutput<T> {
    pub rows: Vec<T>,
    pub warnings: Vec<String>,
}

#[allow(clippy::collapsible_match)]
pub fn parse_bvbg_028<R: BufRead>(reader: R) -> Result<ParsedOutput<Bvbg028Row>> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut rows = Vec::new();
    let mut in_record = false;
    let mut current_tag = String::new();
    let mut current_text = String::new();
    let mut current_row = Bvbg028Row::default();
    let mut tag_stack: Vec<String> = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let name = local_name(e.name().as_ref());
                let lower_name = name.to_lowercase();
                tag_stack.push(lower_name.clone());

                if lower_name == "instrument" || lower_name == "instrm" {
                    in_record = true;
                    current_row = Bvbg028Row::default();
                } else if in_record {
                    current_tag = lower_name;
                    current_text.clear();
                }
            }
            Event::End(ref e) => {
                let name = local_name(e.name().as_ref());
                let lower_name = name.to_lowercase();
                if let Some(pos) = tag_stack.iter().rposition(|x| x == &lower_name) {
                    tag_stack.truncate(pos);
                }

                if lower_name == "instrument" || lower_name == "instrm" {
                    in_record = false;
                    if current_row.security_id.is_empty() {
                        if let Some(ref tckr) = current_row.ticker_symbol {
                            current_row.security_id = tckr.clone();
                        } else if let Some(ref isin) = current_row.isin {
                            current_row.security_id = isin.clone();
                        }
                    }
                    if !current_row.security_id.is_empty() {
                        rows.push(current_row.clone());
                    }
                    current_row = Bvbg028Row::default();
                } else if in_record {
                    let text = current_text.trim();
                    match current_tag.as_str() {
                        "sctyid" | "securityid" | "security_id" | "id" => {
                            if current_row.security_id.is_empty() && !text.is_empty() {
                                current_row.security_id =
                                    canonicalize_string(Some(text)).unwrap_or_default();
                            }
                        }
                        "securitysource" | "security_source" => {
                            if current_row.security_source.is_none() {
                                current_row.security_source = canonicalize_string(Some(text));
                            }
                        }
                        "marketidentifiercode" | "mic" => {
                            if current_row.market_identifier_code.is_none() {
                                current_row.market_identifier_code =
                                    canonicalize_string(Some(text));
                            }
                        }
                        "asset" | "asst" => {
                            if current_row.asset.is_none() {
                                current_row.asset = canonicalize_string(Some(text));
                            }
                        }
                        "assetdescription" | "asset_description" | "asstdesc" | "desc"
                        | "description" => {
                            if current_row.asset_description.is_none() {
                                current_row.asset_description = canonicalize_string(Some(text));
                            }
                        }
                        "marketcode" | "market_code" | "mkt" => {
                            if current_row.market_code.is_none() {
                                current_row.market_code = canonicalize_string(Some(text));
                            }
                        }
                        "marketname" | "market_name" | "mktdesc" => {
                            if current_row.market_name.is_none() {
                                current_row.market_name = canonicalize_string(Some(text));
                            }
                        }
                        "segmentcode" | "segment_code" | "sgmt" => {
                            if current_row.segment_code.is_none() {
                                current_row.segment_code = canonicalize_string(Some(text));
                            }
                        }
                        "segmentname" | "segment_name" | "sgmtdesc" => {
                            if current_row.segment_name.is_none() {
                                current_row.segment_name = canonicalize_string(Some(text));
                            }
                        }
                        "tickersymbol" | "ticker_symbol" | "ticker" | "tckrsymb" => {
                            if current_row.ticker_symbol.is_none() {
                                current_row.ticker_symbol = canonicalize_ticker(Some(text));
                            }
                        }
                        "isin" => {
                            if current_row.isin.is_none() {
                                current_row.isin = canonicalize_isin(Some(text));
                            }
                        }
                        "cficode" | "cfi" | "cficd" => {
                            if current_row.cfi_code.is_none() {
                                current_row.cfi_code = canonicalize_string(Some(text));
                            }
                        }
                        "securitycategory" | "security_category" | "sctyctgy" => {
                            if current_row.security_category.is_none() {
                                current_row.security_category = canonicalize_string(Some(text));
                            }
                        }
                        "instrumentstatus" | "instrument_status" | "status" | "instrmsts" => {
                            if current_row.instrument_status.is_none() {
                                current_row.instrument_status = canonicalize_string(Some(text));
                            }
                        }
                        "maturitydate" | "maturity_date" | "mtrtydt" | "xprtndt" => {
                            if current_row.maturity_date.is_none() {
                                current_row.maturity_date = canonicalize_date(Some(text))?;
                            }
                        }
                        "referencedate" | "reference_date" | "dt" => {
                            if current_row.reference_date.is_none() {
                                current_row.reference_date = canonicalize_date(Some(text))?;
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
