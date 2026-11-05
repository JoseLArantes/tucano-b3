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
pub struct Bvbg086Row {
    pub trade_date: Option<NaiveDate>,
    pub ticker_symbol: Option<String>,
    pub security_id: Option<String>,
    pub market_identifier_code: Option<String>,
    pub trade_quantity: Option<Decimal>,
    pub days_to_settlement: Option<i32>,
    pub financial_volume_brl: Option<Decimal>,
    pub financial_volume_usd: Option<Decimal>,
    pub open_interest: Option<Decimal>,
    pub financial_instrument_quantity: Option<Decimal>,
    pub best_bid_price: Option<Decimal>,
    pub best_ask_price: Option<Decimal>,
    pub first_price: Option<Decimal>,
    pub minimum_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub maximum_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub regular_transactions_quantity: Option<i32>,
    pub non_regular_transactions_quantity: Option<i32>,
    pub regular_traded_contracts: Option<Decimal>,
    pub non_regular_traded_contracts: Option<Decimal>,
    pub national_regular_volume: Option<Decimal>,
    pub national_non_regular_volume: Option<Decimal>,
    pub international_regular_volume: Option<Decimal>,
    pub international_non_regular_volume: Option<Decimal>,
    pub oscillation_percentage: Option<Decimal>,
    pub maximum_trade_limit: Option<Decimal>,
    pub minimum_trade_limit: Option<Decimal>,
}

pub fn parse_bvbg_086<R: BufRead>(reader: R) -> Result<ParsedOutput<Bvbg086Row>> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut rows = Vec::new();
    let mut in_record = false;
    let mut current_tag = String::new();
    let mut current_text = String::new();
    let mut current_row = Bvbg086Row::default();

    loop {
        match xml_reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "pricerecord" || name == "price" || name == "quote" || name == "pricrpt" {
                    in_record = true;
                    current_row = Bvbg086Row::default();
                } else if in_record {
                    current_tag = name;
                    current_text.clear();
                }
            }
            Event::End(ref e) => {
                let name = local_name(e.name().as_ref()).to_lowercase();
                if name == "pricerecord" || name == "price" || name == "quote" || name == "pricrpt" {
                    in_record = false;
                    if current_row.trade_date.is_some()
                        && (current_row.security_id.is_some() || current_row.ticker_symbol.is_some())
                    {
                        rows.push(current_row.clone());
                    }
                    current_row = Bvbg086Row::default();
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
                        "tradequantity" | "trade_quantity" | "tradqty" => {
                            if current_row.trade_quantity.is_none() {
                                current_row.trade_quantity = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "daystosettlement" | "days_to_settlement" | "daystosttlm" => {
                            if current_row.days_to_settlement.is_none() {
                                current_row.days_to_settlement = canonicalize_int(Some(text))?.map(|v| v as i32);
                            }
                        }
                        "financialvolumebrl" | "financial_volume_brl" | "ntlfinvol" => {
                            if current_row.financial_volume_brl.is_none() {
                                current_row.financial_volume_brl = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "financialvolumeusd" | "financial_volume_usd" | "intlfinvol" => {
                            if current_row.financial_volume_usd.is_none() {
                                current_row.financial_volume_usd = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "openinterest" | "open_interest" | "opnintrst" => {
                            if current_row.open_interest.is_none() {
                                current_row.open_interest = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "financialinstrumentquantity" | "financial_instrument_quantity" | "fininstrmqty" => {
                            if current_row.financial_instrument_quantity.is_none() {
                                current_row.financial_instrument_quantity = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "bestbidprice" | "best_bid_price" | "bestbidpric" => {
                            if current_row.best_bid_price.is_none() {
                                current_row.best_bid_price = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "bestaskprice" | "best_ask_price" | "bestaskpric" => {
                            if current_row.best_ask_price.is_none() {
                                current_row.best_ask_price = canonicalize_decimal(Some(text))?;
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
                        "regulartransactionsquantity" | "regular_transactions_quantity" | "tradescount" | "rglrtxsqty" => {
                            if current_row.regular_transactions_quantity.is_none() {
                                current_row.regular_transactions_quantity = canonicalize_int(Some(text))?.map(|v| v as i32);
                            }
                        }
                        "nonregulartransactionsquantity" | "non_regular_transactions_quantity" | "nonrglrtxsqty" => {
                            if current_row.non_regular_transactions_quantity.is_none() {
                                current_row.non_regular_transactions_quantity = canonicalize_int(Some(text))?.map(|v| v as i32);
                            }
                        }
                        "regulartradedcontracts" | "regular_traded_contracts" | "rglrtraddctrcts" => {
                            if current_row.regular_traded_contracts.is_none() {
                                current_row.regular_traded_contracts = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "nonregulartradedcontracts" | "non_regular_traded_contracts" | "nonrglrtraddctrcts" => {
                            if current_row.non_regular_traded_contracts.is_none() {
                                current_row.non_regular_traded_contracts = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "nationalregularvolume" | "national_regular_volume" | "ntlrglrvol" => {
                            if current_row.national_regular_volume.is_none() {
                                current_row.national_regular_volume = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "nationalnonregularvolume" | "national_non_regular_volume" | "ntlnonrglrvol" => {
                            if current_row.national_non_regular_volume.is_none() {
                                current_row.national_non_regular_volume = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "internationalregularvolume" | "international_regular_volume" | "intlrglrvol" => {
                            if current_row.international_regular_volume.is_none() {
                                current_row.international_regular_volume = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "internationalnonregularvolume" | "international_non_regular_volume" | "intlnonrglrvol" => {
                            if current_row.international_non_regular_volume.is_none() {
                                current_row.international_non_regular_volume = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "oscillationpercentage" | "oscillation_percentage" | "oscnpctg" => {
                            if current_row.oscillation_percentage.is_none() {
                                current_row.oscillation_percentage = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "maximumtradelimit" | "maximum_trade_limit" | "maxtradlmt" => {
                            if current_row.maximum_trade_limit.is_none() {
                                current_row.maximum_trade_limit = canonicalize_decimal(Some(text))?;
                            }
                        }
                        "minimumtradelimit" | "minimum_trade_limit" | "mintradlmt" => {
                            if current_row.minimum_trade_limit.is_none() {
                                current_row.minimum_trade_limit = canonicalize_decimal(Some(text))?;
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

