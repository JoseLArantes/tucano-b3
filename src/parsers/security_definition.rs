//! Security Definition Report JSON/CSV, B3 specification v1.0.1 (2024-10-14).
//! Publication/reference date is supplied by source evidence, not invented from record fields.
use super::{
    bvbg_028::Bvbg028Row,
    canonical::*,
    registry_stream::{RegistryEvent, RegistrySummary},
};
use crate::error::{AppError, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    io::{BufRead, Read},
    str::FromStr,
};

const MAX_RECORD: usize = 1024 * 1024;
const MAX_GROUP: usize = 10_000;
const SCALARS: &[&str] = &[
    "Symbol",
    "SecurityID",
    "SecurityIDSource",
    "SecurityExchange",
    "ISINNumber",
    "Product",
    "CFICode",
    "SecurityGroup",
    "SecurityType",
    "SecuritySubType",
    "MaturityMonthYear",
    "MaturityDate",
    "IssueDate",
    "CountryOfIssue",
    "PutOrCall",
    "StrikePrice",
    "StrikeCurrency",
    "ExerciseStyle",
    "ContractMultiplier",
    "SecurityDesc",
    "ContractSettlMonth",
    "DatedDate",
    "SettlType",
    "SettlDate",
    "PriceDivisor",
    "MinPriceIncrement",
    "TickSizeDenominator",
    "MinOrderQty",
    "MaxOrderQty",
    "MultiLegModel",
    "MultiLegPriceMethod",
    "StartDate",
    "EndDate",
    "Currency",
    "SettlCurrency",
    "SecurityStrategyType",
    "Asset",
    "NoSharesIssued",
    "SecurityValidityTimestamp",
    "MarketSegmentID",
    "GovernanceIndicator",
    "CorporateActionEventID",
    "SecurityMatchType",
    "ImpliedMarketIndicator",
    "MinCrossQty",
    "MinTradeVol",
    "ClearingHouseID",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityDefinitionRecord {
    #[serde(skip)]
    pub reference_date: Option<NaiveDate>,
    #[serde(rename = "Symbol")]
    pub symbol: String,
    #[serde(rename = "SecurityID")]
    pub security_id: String,
    #[serde(rename = "SecurityIDSource", default)]
    pub security_id_source: Option<String>,
    #[serde(rename = "SecurityExchange", default)]
    pub security_exchange: Option<String>,
    #[serde(rename = "ISINNumber", default)]
    pub isin_number: Option<String>,
    #[serde(rename = "Product", default)]
    pub product: Option<String>,
    #[serde(rename = "CFICode", default)]
    pub cfi_code: Option<String>,
    #[serde(rename = "SecurityGroup", default)]
    pub security_group: Option<String>,
    #[serde(rename = "SecurityType", default)]
    pub security_type: Option<String>,
    #[serde(rename = "SecuritySubType", default)]
    pub security_sub_type: Option<String>,
    #[serde(rename = "MaturityMonthYear", default)]
    pub maturity_month_year: Option<String>,
    #[serde(rename = "MaturityDate", default)]
    pub maturity_date: Option<String>,
    #[serde(rename = "IssueDate", default)]
    pub issue_date: Option<String>,
    #[serde(rename = "CountryOfIssue", default)]
    pub country_of_issue: Option<String>,
    #[serde(rename = "PutOrCall", default)]
    pub put_or_call: Option<String>,
    #[serde(rename = "StrikePrice", default, deserialize_with = "optional_decimal")]
    pub strike_price: Option<Decimal>,
    #[serde(rename = "StrikeCurrency", default)]
    pub strike_currency: Option<String>,
    #[serde(rename = "ExerciseStyle", default)]
    pub exercise_style: Option<String>,
    #[serde(
        rename = "ContractMultiplier",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub contract_multiplier: Option<Decimal>,
    #[serde(rename = "SecurityDesc", default)]
    pub security_desc: Option<String>,
    #[serde(rename = "ContractSettlMonth", default)]
    pub contract_settl_month: Option<String>,
    #[serde(rename = "DatedDate", default)]
    pub dated_date: Option<String>,
    #[serde(rename = "SettlType", default)]
    pub settl_type: Option<String>,
    #[serde(rename = "SettlDate", default)]
    pub settl_date: Option<String>,
    #[serde(
        rename = "PriceDivisor",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub price_divisor: Option<Decimal>,
    #[serde(
        rename = "MinPriceIncrement",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub min_price_increment: Option<Decimal>,
    #[serde(rename = "TickSizeDenominator", default)]
    pub tick_size_denominator: Option<String>,
    #[serde(rename = "MinOrderQty", default, deserialize_with = "optional_decimal")]
    pub min_order_qty: Option<Decimal>,
    #[serde(rename = "MaxOrderQty", default, deserialize_with = "optional_decimal")]
    pub max_order_qty: Option<Decimal>,
    #[serde(rename = "MultiLegModel", default)]
    pub multi_leg_model: Option<String>,
    #[serde(rename = "MultiLegPriceMethod", default)]
    pub multi_leg_price_method: Option<String>,
    #[serde(rename = "StartDate", default)]
    pub start_date: Option<String>,
    #[serde(rename = "EndDate", default)]
    pub end_date: Option<String>,
    #[serde(rename = "Currency", default)]
    pub currency: Option<String>,
    #[serde(rename = "SettlCurrency", default)]
    pub settl_currency: Option<String>,
    #[serde(rename = "SecurityStrategyType", default)]
    pub security_strategy_type: Option<String>,
    #[serde(rename = "Asset", default)]
    pub asset: Option<String>,
    #[serde(
        rename = "NoSharesIssued",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub no_shares_issued: Option<Decimal>,
    #[serde(rename = "SecurityValidityTimestamp", default)]
    pub security_validity_timestamp: Option<String>,
    #[serde(rename = "MarketSegmentID", default)]
    pub market_segment_id: Option<String>,
    #[serde(rename = "GovernanceIndicator", default)]
    pub governance_indicator: Option<String>,
    #[serde(rename = "CorporateActionEventID", default)]
    pub corporate_action_event_id: Option<String>,
    #[serde(rename = "SecurityMatchType", default)]
    pub security_match_type: Option<String>,
    #[serde(rename = "ImpliedMarketIndicator", default)]
    pub implied_market_indicator: Option<String>,
    #[serde(rename = "MinCrossQty", default, deserialize_with = "optional_decimal")]
    pub min_cross_qty: Option<Decimal>,
    #[serde(rename = "MinTradeVol", default, deserialize_with = "optional_decimal")]
    pub min_trade_vol: Option<Decimal>,
    #[serde(rename = "ClearingHouseID", default)]
    pub clearing_house_id: Option<String>,
    #[serde(rename = "NoApplIDs", default)]
    pub appl_ids: Vec<ApplicationId>,
    #[serde(rename = "NoUnderlyings", alias = "NoUnderlyingIs", default)]
    pub underlying: Vec<Underlying>,
    #[serde(rename = "NoInstrAttrib", alias = "NoInstrAttribs", default)]
    pub attributes: Vec<InstrumentAttribute>,
    #[serde(rename = "NoLegs", default)]
    pub legs: Vec<Leg>,
    #[serde(rename = "NoTickRules", default)]
    pub tick_rules: Vec<TickRule>,
    #[serde(rename = "NoLotTypeRules", default)]
    pub lot_rules: Vec<LotRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationId {
    #[serde(rename = "ApplID")]
    pub appl_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Underlying {
    #[serde(rename = "UnderlyingSymbol", default)]
    pub symbol: Option<String>,
    #[serde(rename = "UnderlyingSecurityID", default)]
    pub security_id: Option<String>,
    #[serde(rename = "UnderlyingSecurityIDSource", default)]
    pub security_id_source: Option<String>,
    #[serde(rename = "UnderlyingSecurityExchange", default)]
    pub security_exchange: Option<String>,
    #[serde(rename = "IndexPct", default, deserialize_with = "optional_decimal")]
    pub index_pct: Option<Decimal>,
    #[serde(
        rename = "IndexTheoreticalQty",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub index_theoretical_qty: Option<Decimal>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentAttribute {
    #[serde(rename = "InstrAttribType")]
    pub attribute_type: String,
    #[serde(rename = "InstrAttribValue")]
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leg {
    #[serde(rename = "LegSymbol", default)]
    pub symbol: Option<String>,
    #[serde(rename = "LegSecurityID", default)]
    pub security_id: Option<String>,
    #[serde(rename = "LegSecurityIDSource", default)]
    pub security_id_source: Option<String>,
    #[serde(rename = "LegSecurityExchange", default)]
    pub security_exchange: Option<String>,
    #[serde(rename = "LegSecurityType", default)]
    pub security_type: Option<String>,
    #[serde(rename = "LegSide", default)]
    pub side: Option<String>,
    #[serde(rename = "LegRatioQty", default, deserialize_with = "optional_decimal")]
    pub ratio_quantity: Option<Decimal>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TickRule {
    #[serde(
        rename = "StartTickPriceRange",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub start_price: Option<Decimal>,
    #[serde(
        rename = "EndTickPriceRange",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub end_price: Option<Decimal>,
    #[serde(
        rename = "TickIncrement",
        default,
        deserialize_with = "optional_decimal"
    )]
    pub increment: Option<Decimal>,
    #[serde(rename = "TickRuleType", default)]
    pub rule_type: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LotRule {
    #[serde(rename = "LotType", default)]
    pub lot_type: Option<String>,
    #[serde(rename = "MinLotSize", default, deserialize_with = "optional_decimal")]
    pub min_lot_size: Option<Decimal>,
}

fn optional_decimal<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Decimal>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            Decimal::from_str(s.trim())
                .map(|d| d.normalize())
                .map_err(serde::de::Error::custom)
        })
        .transpose()
}
impl SecurityDefinitionRecord {
    pub fn normalize(&mut self) {
        self.symbol = self.symbol.trim().to_ascii_uppercase();
        self.security_id = self.security_id.trim().into();
        for value in [
            &mut self.security_id_source,
            &mut self.security_exchange,
            &mut self.isin_number,
            &mut self.product,
            &mut self.cfi_code,
            &mut self.security_group,
            &mut self.security_type,
            &mut self.security_sub_type,
            &mut self.maturity_month_year,
            &mut self.maturity_date,
            &mut self.issue_date,
            &mut self.country_of_issue,
            &mut self.put_or_call,
            &mut self.strike_currency,
            &mut self.exercise_style,
            &mut self.security_desc,
            &mut self.contract_settl_month,
            &mut self.dated_date,
            &mut self.settl_type,
            &mut self.settl_date,
            &mut self.tick_size_denominator,
            &mut self.multi_leg_model,
            &mut self.multi_leg_price_method,
            &mut self.start_date,
            &mut self.end_date,
            &mut self.currency,
            &mut self.settl_currency,
            &mut self.security_strategy_type,
            &mut self.asset,
            &mut self.security_validity_timestamp,
            &mut self.market_segment_id,
            &mut self.governance_indicator,
            &mut self.corporate_action_event_id,
            &mut self.security_match_type,
            &mut self.implied_market_indicator,
            &mut self.clearing_house_id,
        ] {
            *value = canonicalize_string(value.as_deref());
        }
        for value in [
            &mut self.strike_price,
            &mut self.contract_multiplier,
            &mut self.price_divisor,
            &mut self.min_price_increment,
            &mut self.min_order_qty,
            &mut self.max_order_qty,
            &mut self.no_shares_issued,
            &mut self.min_cross_qty,
            &mut self.min_trade_vol,
        ] {
            *value = value.map(|d| d.normalize());
        }
        for value in [
            &mut self.maturity_date,
            &mut self.issue_date,
            &mut self.dated_date,
            &mut self.settl_date,
            &mut self.start_date,
            &mut self.end_date,
        ] {
            if let Ok(parsed) = date(value.as_deref()) {
                *value = parsed.map(|d| d.to_string());
            }
        }
        if self
            .security_validity_timestamp
            .as_deref()
            .is_some_and(|v| v.starts_with("9999-12-31"))
        {
            self.security_validity_timestamp = None;
        }
        self.isin_number = canonicalize_isin(self.isin_number.as_deref());
        for value in [
            &mut self.security_exchange,
            &mut self.cfi_code,
            &mut self.currency,
            &mut self.settl_currency,
            &mut self.strike_currency,
        ] {
            *value = value.take().map(|s| s.to_ascii_uppercase());
        }
        for row in &mut self.appl_ids {
            row.appl_id = row.appl_id.trim().into();
        }
        for row in &mut self.attributes {
            row.attribute_type = row.attribute_type.trim().into();
            row.value = row.value.trim().into();
        }
        for row in &mut self.underlying {
            for value in [
                &mut row.symbol,
                &mut row.security_id,
                &mut row.security_id_source,
                &mut row.security_exchange,
            ] {
                *value = canonicalize_string(value.as_deref());
            }
            row.index_pct = row.index_pct.map(|d| d.normalize());
            row.index_theoretical_qty = row.index_theoretical_qty.map(|d| d.normalize());
        }
        for row in &mut self.legs {
            for value in [
                &mut row.symbol,
                &mut row.security_id,
                &mut row.security_id_source,
                &mut row.security_exchange,
                &mut row.security_type,
                &mut row.side,
            ] {
                *value = canonicalize_string(value.as_deref());
            }
            row.ratio_quantity = row.ratio_quantity.map(|d| d.normalize());
        }
        for row in &mut self.tick_rules {
            for value in [&mut row.start_price, &mut row.end_price, &mut row.increment] {
                *value = value.map(|d| d.normalize());
            }
            row.rule_type = canonicalize_string(row.rule_type.as_deref());
        }
        for row in &mut self.lot_rules {
            row.lot_type = canonicalize_string(row.lot_type.as_deref());
            row.min_lot_size = row.min_lot_size.map(|d| d.normalize());
        }
    }
    pub fn validate(&self) -> Result<()> {
        for value in [
            &self.maturity_date,
            &self.issue_date,
            &self.dated_date,
            &self.settl_date,
            &self.start_date,
            &self.end_date,
        ] {
            date(value.as_deref())?;
        }
        if self.symbol.trim().is_empty()
            || self.symbol.len() > 32
            || self.security_id.is_empty()
            || self.security_id.len() > 64
            || !self.security_id.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(layout("Definição sem Symbol/SecurityID válido."));
        }
        for length in [
            self.appl_ids.len(),
            self.underlying.len(),
            self.attributes.len(),
            self.legs.len(),
            self.tick_rules.len(),
            self.lot_rules.len(),
        ] {
            if length > MAX_GROUP {
                return Err(layout("Grupo repetido excede limite."));
            }
        }
        if self.appl_ids.iter().any(|a| a.appl_id.trim().is_empty())
            || self
                .attributes
                .iter()
                .any(|a| a.attribute_type.trim().is_empty() || a.value.trim().is_empty())
        {
            return Err(layout("Grupo repetido contém campo obrigatório vazio."));
        }
        Ok(())
    }
    pub fn to_instrument_row(&self) -> Result<Bvbg028Row> {
        self.validate()?;
        Ok(Bvbg028Row {
            security_id: self.security_id.trim().into(),
            security_source: canonicalize_string(self.security_id_source.as_deref()),
            market_identifier_code: canonicalize_string(self.security_exchange.as_deref()),
            ticker_symbol: canonicalize_ticker(Some(&self.symbol)),
            isin: canonicalize_isin(self.isin_number.as_deref()),
            asset: self.asset.clone(),
            asset_description: self.security_desc.clone(),
            segment_code: self.market_segment_id.clone(),
            instrument_type: self.security_type.clone(),
            security_category: self.security_sub_type.clone(),
            cfi_code: self.cfi_code.clone(),
            maturity_date: date(self.maturity_date.as_deref())?,
            reference_date: self.reference_date,
            ..Bvbg028Row::default()
        })
    }
}
fn date(value: Option<&str>) -> Result<Option<NaiveDate>> {
    match value {
        Some("9999-12-31") => Ok(None),
        value => canonicalize_date(value),
    }
}
fn layout(message: &str) -> AppError {
    AppError::Layout(message.into())
}

const GROUPS: &[(&str, &[&str])] = &[
    ("NoApplIDs", &["ApplID"]),
    (
        "NoUnderlyings",
        &[
            "UnderlyingSymbol",
            "UnderlyingSecurityID",
            "UnderlyingSecurityIDSource",
            "UnderlyingSecurityExchange",
            "IndexPct",
            "IndexTheoreticalQty",
        ],
    ),
    ("NoInstrAttrib", &["InstrAttribType", "InstrAttribValue"]),
    (
        "NoLegs",
        &[
            "LegSymbol",
            "LegSecurityID",
            "LegSecurityIDSource",
            "LegSecurityExchange",
            "LegSecurityType",
            "LegRatioQty",
            "LegSide",
        ],
    ),
    (
        "NoTickRules",
        &[
            "StartTickPriceRange",
            "EndTickPriceRange",
            "TickIncrement",
            "TickRuleType",
        ],
    ),
    ("NoLotTypeRules", &["LotType", "MinLotSize"]),
];
fn normalize_header(name: &str) -> &str {
    match name {
        "NoUnderlyingIs" => "NoUnderlyings",
        "NoInstrAttribs" => "NoInstrAttrib",
        name => name,
    }
}

pub fn parse_security_definition_csv<
    R: BufRead,
    F: FnMut(RegistryEvent<SecurityDefinitionRecord>) -> Result<()>,
>(
    mut reader: R,
    reference_date: NaiveDate,
    mut callback: F,
) -> Result<RegistrySummary> {
    let mut line = Vec::new();
    let mut offset = 0u64;
    let size = read_line(&mut reader, &mut line)?;
    offset += size as u64;
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_slice());
    let headers = csv
        .records()
        .next()
        .transpose()?
        .ok_or_else(|| layout("Header Security Definition ausente."))?;
    let columns: Vec<_> = headers.iter().map(normalize_header).collect();
    let mut seen = HashSet::new();
    if columns.len() > 128
        || columns.iter().any(|name| {
            !seen.insert(*name)
                || !(SCALARS.contains(name)
                    || GROUPS
                        .iter()
                        .any(|(group, children)| group == name || children.contains(name)))
        })
        || !columns.contains(&"Symbol")
        || !columns.contains(&"SecurityID")
    {
        return Err(layout("Header fora do perfil Security Definition v1.0.1."));
    }
    let mut summary = RegistrySummary::default();
    loop {
        line.clear();
        let start = offset;
        let size = read_line(&mut reader, &mut line)?;
        if size == 0 {
            break;
        }
        offset += size as u64;
        summary.records += 1;
        let parsed = (|| -> Result<SecurityDefinitionRecord> {
            if line.iter().filter(|b| **b == b'"').count() % 2 != 0 {
                return Err(layout("Aspas CSV incompletas."));
            }
            let mut csv = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(line.as_slice());
            let values = csv
                .records()
                .next()
                .transpose()?
                .ok_or_else(|| layout("Registro CSV vazio."))?;
            if values.len() != columns.len() {
                return Err(layout("Quantidade de campos CSV divergente."));
            }
            let mut object = Map::new();
            for (name, value) in columns.iter().zip(values.iter()) {
                if SCALARS.contains(name) && !value.trim().is_empty() {
                    object.insert((*name).into(), Value::String(value.trim().into()));
                }
            }
            for (group, children) in GROUPS {
                let count = columns
                    .iter()
                    .position(|name| name == group)
                    .and_then(|i| values.get(i))
                    .filter(|s| !s.trim().is_empty())
                    .map(|v| {
                        v.parse::<usize>()
                            .map_err(|_| layout("Contagem de grupo CSV inválida."))
                    })
                    .transpose()?
                    .unwrap_or(0);
                if count > MAX_GROUP {
                    return Err(layout("Contagem de grupo CSV excede limite."));
                }
                let mut rows: Vec<Map<String, Value>> = (0..count).map(|_| Map::new()).collect();
                for child in *children {
                    if let Some(i) = columns.iter().position(|name| name == child) {
                        let field = values.get(i).unwrap_or("");
                        if field.is_empty() {
                            continue;
                        }
                        let parts: Vec<_> = field.split('/').collect();
                        if parts.len() != count {
                            return Err(layout("Grupo CSV não corresponde à contagem publicada."));
                        }
                        for (row, value) in rows.iter_mut().zip(parts) {
                            if !value.is_empty() {
                                row.insert((*child).into(), Value::String(value.into()));
                            }
                        }
                    }
                }
                if count > 0 {
                    object.insert(
                        (*group).into(),
                        Value::Array(rows.into_iter().map(Value::Object).collect()),
                    );
                }
            }
            let mut record: SecurityDefinitionRecord =
                serde_json::from_value(Value::Object(object))?;
            record.normalize();
            record.reference_date = Some(reference_date);
            record.validate()?;
            Ok(record)
        })();
        emit(
            parsed,
            line.clone(),
            summary.records,
            start,
            &mut callback,
            &mut summary,
        )?;
    }
    if summary.records == 0 {
        return Err(layout("Security Definition sem registros."));
    }
    Ok(summary)
}
fn read_line<R: BufRead>(reader: &mut R, buffer: &mut Vec<u8>) -> Result<usize> {
    let n = reader
        .take((MAX_RECORD + 1) as u64)
        .read_until(b'\n', buffer)?;
    if n > MAX_RECORD {
        return Err(layout("Registro Security Definition excede limite."));
    }
    Ok(n)
}
fn emit<F: FnMut(RegistryEvent<SecurityDefinitionRecord>) -> Result<()>>(
    record: Result<SecurityDefinitionRecord>,
    raw: Vec<u8>,
    record_index: u64,
    byte_offset: u64,
    callback: &mut F,
    summary: &mut RegistrySummary,
) -> Result<()> {
    match record {
        Ok(row) => {
            callback(RegistryEvent::Valid {
                row,
                record_index,
                byte_offset,
            })?;
            summary.valid_records += 1;
        }
        Err(e) => {
            callback(RegistryEvent::Rejected {
                raw,
                reason: e.to_string(),
                record_index,
                byte_offset,
            })?;
            summary.rejected_records += 1;
        }
    }
    Ok(())
}

/// Frames one JSON object at a time, with explicit byte/depth limits and structural failure on truncation.
pub fn parse_security_definition_json<
    R: BufRead,
    F: FnMut(RegistryEvent<SecurityDefinitionRecord>) -> Result<()>,
>(
    mut reader: R,
    reference_date: NaiveDate,
    mut callback: F,
) -> Result<RegistrySummary> {
    let mut offset = 0u64;
    let mut summary = RegistrySummary::default();
    if next_nonspace(&mut reader, &mut offset)? != Some(b'[') {
        return Err(layout("Security Definition JSON exige array raiz."));
    }
    let mut next = next_nonspace(&mut reader, &mut offset)?;
    if next == Some(b']') {
        return Err(layout("Security Definition sem registros."));
    }
    loop {
        if next != Some(b'{') {
            return Err(layout("Objeto Security Definition esperado."));
        }
        let start = offset - 1;
        let mut raw = vec![b'{'];
        let mut depth = 1usize;
        let mut string = false;
        let mut escaped = false;
        let mut non_string_scalar = false;
        while depth > 0 {
            let byte = read_byte(&mut reader, &mut offset)?
                .ok_or_else(|| layout("JSON Security Definition truncado."))?;
            raw.push(byte);
            if raw.len() > MAX_RECORD {
                return Err(layout("Registro Security Definition excede limite."));
            }
            if string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    string = false;
                }
            } else {
                match byte {
                    b'"' => string = true,
                    b'{' | b'[' => {
                        depth += 1;
                        if depth > 64 {
                            return Err(layout("Profundidade JSON excede limite."));
                        }
                    }
                    b'}' | b']' => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or_else(|| layout("Estrutura JSON inválida."))?
                    }
                    b':' | b',' => {}
                    b if b.is_ascii_whitespace() => {}
                    _ => non_string_scalar = true,
                }
            }
        }
        // Invalid syntax is a member failure; typed/schema-invalid objects are quarantinable records.
        let mut syntax = serde_json::Deserializer::from_slice(&raw);
        serde::de::IgnoredAny::deserialize(&mut syntax)?;
        syntax.end()?;
        let parsed = (|| -> Result<SecurityDefinitionRecord> {
            if non_string_scalar {
                return Err(layout(
                    "Campos escalares do Security Definition devem ser strings.",
                ));
            }
            let mut row: SecurityDefinitionRecord = serde_json::from_slice(&raw)?;
            row.normalize();
            row.reference_date = Some(reference_date);
            row.validate()?;
            Ok(row)
        })();
        summary.records += 1;
        emit(
            parsed,
            raw,
            summary.records,
            start,
            &mut callback,
            &mut summary,
        )?;
        match next_nonspace(&mut reader, &mut offset)? {
            Some(b',') => next = next_nonspace(&mut reader, &mut offset)?,
            Some(b']') => break,
            _ => return Err(layout("Separador JSON inválido.")),
        }
    }
    if next_nonspace(&mut reader, &mut offset)?.is_some() {
        return Err(layout("Conteúdo após array JSON."));
    }
    Ok(summary)
}
fn read_byte<R: BufRead>(reader: &mut R, offset: &mut u64) -> Result<Option<u8>> {
    let buffer = reader.fill_buf()?;
    let byte = buffer.first().copied();
    if byte.is_some() {
        reader.consume(1);
        *offset += 1;
    }
    Ok(byte)
}
fn next_nonspace<R: BufRead>(reader: &mut R, offset: &mut u64) -> Result<Option<u8>> {
    loop {
        let next = read_byte(reader, offset)?;
        if next.is_none_or(|b| !b.is_ascii_whitespace()) {
            return Ok(next);
        }
    }
}
