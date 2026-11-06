//! Synthetic data using field/group structures from B3 Security Definition Report v1.0.1.
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::io::{BufReader, Cursor};
use tucano_b3::parsers::{
    registry_stream::RegistryEvent,
    security_definition::{
        parse_security_definition_csv, parse_security_definition_json, SecurityDefinitionRecord,
    },
};

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()
}
fn json() -> String {
    r#"[{"Symbol":"EXMP3","SecurityID":"000123","SecurityIDSource":"8","SecurityExchange":"BVMF","ISINNumber":"BREXMPACNOR0","SecurityType":"CS","SecurityDesc":"EMPRESA EXEMPLO","MinPriceIncrement":"0.0100","NoApplIDs":[{"ApplID":"MBO1"},{"ApplID":"MBP101"}],"NoUnderlyings":[{"UnderlyingSymbol":"EXMP4","UnderlyingSecurityID":"456","UnderlyingSecurityIDSource":"8","UnderlyingSecurityExchange":"BVMF","IndexPct":"0.1250"}],"NoInstrAttrib":[{"InstrAttribType":"24","InstrAttribValue":"1"}],"NoLegs":[{"LegSymbol":"EXMP4","LegRatioQty":"-1.0000","LegSide":"2"}],"NoLotTypeRules":[{"LotType":"2","MinLotSize":"100"}]}]"#.into()
}

fn collect(
    event: RegistryEvent<SecurityDefinitionRecord>,
    out: &mut Vec<SecurityDefinitionRecord>,
) -> tucano_b3::error::Result<()> {
    match event {
        RegistryEvent::Valid { row, .. } => out.push(row),
        RegistryEvent::Rejected { reason, .. } => panic!("unexpected rejection: {reason}"),
    }
    Ok(())
}

#[test]
fn official_csv_and_json_groups_have_identical_typed_meaning() {
    let csv="Symbol,SecurityID,SecurityIDSource,SecurityExchange,ISINNumber,SecurityType,SecurityDesc,MinPriceIncrement,NoApplIDs,ApplID,NoUnderlyings,UnderlyingSymbol,UnderlyingSecurityID,UnderlyingSecurityIDSource,UnderlyingSecurityExchange,IndexPct,NoInstrAttrib,InstrAttribType,InstrAttribValue,NoLegs,LegSymbol,LegRatioQty,LegSide,NoLotTypeRules,LotType,MinLotSize\nEXMP3,000123,8,BVMF,BREXMPACNOR0,CS,EMPRESA EXEMPLO,0.01,2,MBO1/MBP101,1,EXMP4,456,8,BVMF,0.125,1,24,1,1,EXMP4,-1,2,1,2,100\n";
    let mut a = Vec::new();
    let mut b = Vec::new();
    parse_security_definition_csv(
        BufReader::with_capacity(13, Cursor::new(csv)),
        date(),
        |e| collect(e, &mut a),
    )
    .unwrap();
    parse_security_definition_json(
        BufReader::with_capacity(7, Cursor::new(json())),
        date(),
        |e| collect(e, &mut b),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    assert_eq!(a[0].appl_ids.len(), 2);
    assert_eq!(a[0].underlying[0].index_pct, Some(Decimal::new(125, 3)));
    assert_eq!(a[0].legs[0].ratio_quantity, Some(Decimal::new(-1, 0)));
    let instrument = a[0].to_instrument_row().unwrap();
    assert_eq!(instrument.security_id, "000123");
    assert_eq!(instrument.security_source.as_deref(), Some("8"));
    assert_eq!(instrument.market_identifier_code.as_deref(), Some("BVMF"));
    assert_eq!(instrument.reference_date, Some(date()));
}

#[test]
fn optional_qualifiers_are_not_invented_and_bad_records_are_isolated() {
    let input = r#"[{"Symbol":"EXMP3","SecurityID":"1"},{"Symbol":"NOID3"},{"Symbol":"EXMP4","SecurityID":"2","MinOrderQty":1.25},{"Symbol":"EXMP5","SecurityID":"3"}]"#;
    let mut valid = Vec::new();
    let mut rejected = 0;
    let stats = parse_security_definition_json(Cursor::new(input), date(), |event| {
        match event {
            RegistryEvent::Valid { row, .. } => valid.push(row),
            RegistryEvent::Rejected { .. } => rejected += 1,
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(stats.records, 4);
    assert_eq!(valid.len(), 2);
    assert_eq!(rejected, 2);
    assert_eq!(valid[0].security_id_source, None);
    assert_eq!(valid[0].security_exchange, None);
}

#[test]
fn unknown_fields_group_mismatch_and_truncation_are_visible() {
    let mut rejected = 0;
    let csv = "Symbol,SecurityID,NoApplIDs,ApplID\nEXMP3,1,2,MBO1\n";
    let stats = parse_security_definition_csv(Cursor::new(csv), date(), |event| {
        if matches!(event, RegistryEvent::Rejected { .. }) {
            rejected += 1;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(stats.rejected_records, 1);
    assert_eq!(rejected, 1);
    assert!(
        parse_security_definition_json(Cursor::new("[{\"Symbol\":\"EXMP3\""), date(), |_| Ok(()))
            .is_err()
    );
    let stats = parse_security_definition_json(
        Cursor::new(r#"[{"Symbol":"EXMP3","SecurityID":"1","Unknown": "x"}]"#),
        date(),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(stats.rejected_records, 1);
    assert!(parse_security_definition_json(
        Cursor::new(format!("[{{\"Symbol\":\"{}\"}}]", "x".repeat(1024 * 1024))),
        date(),
        |_| Ok(())
    )
    .is_err());
}

#[test]
fn financial_strings_preserve_28_digits_and_numeric_tokens_are_rejected() {
    use std::str::FromStr;
    let exact = "12345678901234567890.12345678";
    let input = format!(
        r#"[{{"Symbol":"EXMP3","SecurityID":"1","StrikePrice":"{exact}","PriceDivisor":"{exact}"}},{{"Symbol":"EXMP4","SecurityID":"2","StrikePrice":{exact}}}]"#
    );
    let mut rows = Vec::new();
    let mut rejected = 0;
    parse_security_definition_json(Cursor::new(input), date(), |event| {
        match event {
            RegistryEvent::Valid { row, .. } => rows.push(row),
            RegistryEvent::Rejected { .. } => rejected += 1,
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rejected, 1);
    let value = Decimal::from_str(exact).unwrap();
    assert_eq!(rows[0].strike_price, Some(value));
    assert_eq!(rows[0].price_divisor, Some(value));
    assert_eq!(rows[0].strike_price.unwrap().to_string(), exact);
}
