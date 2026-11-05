use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::str::FromStr;

use crate::error::{AppError, Result};

pub fn canonicalize_string(val: Option<&str>) -> Option<String> {
    match val {
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    }
}

pub fn canonicalize_ticker(val: Option<&str>) -> Option<String> {
    canonicalize_string(val).map(|s| s.to_uppercase())
}

pub fn canonicalize_isin(val: Option<&str>) -> Option<String> {
    canonicalize_string(val).map(|s| s.to_uppercase())
}

pub fn canonicalize_date(val: Option<&str>) -> Result<Option<NaiveDate>> {
    let text = match canonicalize_string(val) {
        Some(t) => t,
        None => return Ok(None),
    };

    if text == "00000000" || text == "99991231" {
        return Ok(None);
    }

    if text.len() == 8 && text.chars().all(|c| c.is_ascii_digit()) {
        NaiveDate::parse_from_str(&text, "%Y%m%d")
            .map(Some)
            .map_err(|e| {
                AppError::Validation(format!("Invalid date format YYYYMMDD: {text} ({e})"))
            })
    } else {
        NaiveDate::parse_from_str(&text, "%Y-%m-%d")
            .map(Some)
            .map_err(|e| {
                AppError::Validation(format!("Invalid date format YYYY-MM-DD: {text} ({e})"))
            })
    }
}

pub fn canonicalize_decimal(val: Option<&str>) -> Result<Option<Decimal>> {
    let text = match canonicalize_string(val) {
        Some(t) => t,
        None => return Ok(None),
    };

    let cleaned = text.replace(',', ".");
    Decimal::from_str(&cleaned)
        .map(|d| Some(d.normalize()))
        .map_err(|e| AppError::Validation(format!("Invalid decimal: {text} ({e})")))
}

pub fn canonicalize_int(val: Option<&str>) -> Result<Option<i64>> {
    let text = match canonicalize_string(val) {
        Some(t) => t,
        None => return Ok(None),
    };

    text.parse::<i64>()
        .map(Some)
        .map_err(|e| AppError::Validation(format!("Invalid integer: {text} ({e})")))
}

pub fn parse_implied_decimal(val: Option<&str>, scale: u32) -> Result<Option<Decimal>> {
    let text = match canonicalize_string(val) {
        Some(t) => t,
        None => return Ok(None),
    };

    if !text.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Validation(format!(
            "Valor com decimal implicito invalido: {text}"
        )));
    }

    let parsed_i128 = text
        .parse::<i128>()
        .map_err(|e| AppError::Validation(format!("Implied decimal overflow for {text}: {e}")))?;

    if scale == 0 {
        Ok(Some(
            Decimal::from_i128_with_scale(parsed_i128, 0).normalize(),
        ))
    } else {
        Ok(Some(
            Decimal::from_i128_with_scale(parsed_i128, scale).normalize(),
        ))
    }
}

pub fn stable_row_hash(fields: &BTreeMap<String, Value>) -> String {
    let raw = serde_json::to_string(fields).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}
