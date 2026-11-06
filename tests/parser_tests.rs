use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::io::Cursor;

use tucano_b3::parsers::bvbg_028::parse_bvbg_028;
use tucano_b3::parsers::bvbg_086::parse_bvbg_086;
use tucano_b3::parsers::bvbg_087::parse_bvbg_087;
use tucano_b3::parsers::bvbg_186::parse_bvbg_186;
use tucano_b3::parsers::cotahist::parse_cotahist;
use tucano_b3::parsers::index_composition::parse_index_composition_csv;
use tucano_b3::parsers::tradable_list::parse_tradable_security_list;

#[test]
fn test_parse_bvbg_028_instrument() {
    let xml = r#"
    <root>
      <instrument>
        <securityId>123456</securityId>
        <mic>BVMF</mic>
        <asset>PETR</asset>
        <marketCode>10</marketCode>
        <segmentCode>1</segmentCode>
        <tickerSymbol>petr4</tickerSymbol>
        <isin>brpetracnpr6</isin>
        <referenceDate>2026-05-30</referenceDate>
      </instrument>
    </root>
    "#;

    let parsed = parse_bvbg_028(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].security_id, "123456");
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("PETR4"));
    assert_eq!(parsed.rows[0].isin.as_deref(), Some("BRPETRACNPR6"));
}

#[test]
fn test_parse_bvbg_086_and_186() {
    let xml = r#"
    <root>
      <priceRecord>
        <tradeDate>2026-05-30</tradeDate>
        <tickerSymbol>vale3</tickerSymbol>
        <securityId>9988</securityId>
        <firstPrice>10.10</firstPrice>
        <minimumPrice>10.00</minimumPrice>
        <averagePrice>10.20</averagePrice>
        <maximumPrice>10.40</maximumPrice>
        <lastPrice>10.30</lastPrice>
        <financialVolumeBrl>1000000.00</financialVolumeBrl>
      </priceRecord>
    </root>
    "#;

    let full = parse_bvbg_086(Cursor::new(xml)).unwrap();
    let simple = parse_bvbg_186(Cursor::new(xml)).unwrap();

    assert_eq!(full.rows.len(), 1);
    assert_eq!(full.rows[0].ticker_symbol.as_deref(), Some("VALE3"));
    assert_eq!(full.rows[0].last_price, Some(Decimal::new(1030, 2)));

    assert_eq!(simple.rows.len(), 1);
    assert_eq!(
        simple.rows[0].source_file_type,
        "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT"
    );
}

#[test]
fn test_parse_bvbg_087() {
    let xml = r#"
    <root>
      <indexValue>
        <tradeDate>2026-05-30</tradeDate>
        <indexCode>IBOV</indexCode>
        <indexName>Ibovespa</indexName>
        <valueType>index_value</valueType>
        <value>132456.98</value>
      </indexValue>
    </root>
    "#;

    let parsed = parse_bvbg_087(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].index_code.as_deref(), Some("IBOV"));
    assert_eq!(parsed.rows[0].value, Some(Decimal::new(13245698, 2)));
}

#[test]
fn test_parse_tradable_and_index_composition_csv() {
    let tradable_csv = "reference_date;security_id;ticker_symbol;market_code;tradable_indicator\n2026-05-30;123456;PETR4;10;SIM\n";
    let tradable = parse_tradable_security_list(Cursor::new(tradable_csv)).unwrap();
    assert_eq!(tradable.rows.len(), 1);
    assert_eq!(tradable.rows[0].tradable_indicator, Some(true));

    let composition_csv = "composition_date,index_code,ticker_symbol,isin,theoretical_quantity,portfolio_weight\n2026-05-30,IBOV,VALE3,BRVALEACNOR0,1000,0.12\n";
    let composition = parse_index_composition_csv(Cursor::new(composition_csv)).unwrap();
    assert_eq!(composition.rows.len(), 1);
    assert_eq!(composition.rows[0].ticker_symbol.as_deref(), Some("VALE3"));
    assert_eq!(
        composition.rows[0].portfolio_weight,
        Some(Decimal::new(12, 2))
    );
}

fn fit(val: &str, width: usize) -> String {
    let s = if val.len() > width {
        &val[..width]
    } else {
        val
    };
    format!("{:<width$}", s, width = width)
}

fn fit_num(val: &str, width: usize) -> String {
    let s = if val.len() > width {
        &val[..width]
    } else {
        val
    };
    format!("{:0>width$}", s, width = width)
}

fn cotahist_quote_line() -> String {
    let parts = vec![
        "01".to_string(),
        "20241230".to_string(),
        "02".to_string(),
        fit("PETR4", 12),
        "010".to_string(),
        fit("PETROBRAS", 12),
        fit("PN", 10),
        fit_num("0", 3),
        fit("BRL", 4),
        fit_num("3625", 13),
        fit_num("3680", 13),
        fit_num("3590", 13),
        fit_num("3630", 13),
        fit_num("3645", 13),
        fit_num("3640", 13),
        fit_num("3650", 13),
        fit_num("123", 5),
        fit_num("10000000", 18),
        fit_num("36300000000", 18),
        fit_num("0", 13),
        "0".to_string(),
        "99991231".to_string(),
        fit_num("1", 7),
        fit_num("0", 13),
        fit("BRPETRACNPR6", 12),
        fit_num("123", 3),
    ];
    let line = parts.join("");
    assert_eq!(line.len(), 245);
    line
}

fn cotahist_header_line() -> String {
    let line = format!("00{: <243}", "");
    assert_eq!(line.len(), 245);
    line
}

fn cotahist_trailer_line(record_count: usize) -> String {
    let prefix = format!("99{: <29}{:0>11}", "", record_count);
    let line = format!("{:<245}", prefix);
    assert_eq!(line.len(), 245);
    line
}

#[test]
fn test_parse_cotahist_fixed_width() {
    let content = format!(
        "{}\n{}\n{}",
        cotahist_header_line(),
        cotahist_quote_line(),
        cotahist_trailer_line(1)
    );
    let parsed = parse_cotahist(&content).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("PETR4"));
    assert_eq!(parsed.rows[0].open_price, Some(Decimal::new(3625, 2)));
    assert_eq!(
        parsed.rows[0].traded_quantity,
        Some(Decimal::new(10000000, 0))
    );
    assert!(parsed.warnings.is_empty());
}

#[test]
fn test_parse_cotahist_invalid_record_size() {
    let res = parse_cotahist("01TOO_SHORT");
    assert!(res.is_err());
}

#[test]
fn test_parse_bvbg_028_synthetic_simplified_xml() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
    <bvbg:Document xmlns:bvbg="urn:bvmf.052.01.xsd" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
      <bvbg:BizData>
        <bvbg:Pyld>
          <bvbg:Document>
            <bvbg:InstrmInf>
              <bvbg:Instrm>
                <bvbg:SctyId>20000001</bvbg:SctyId>
                <bvbg:MktIdrCd>BVMF</bvbg:MktIdrCd>
                <bvbg:Asst>VALE</bvbg:Asst>
                <bvbg:AsstDesc>VALE S.A. ON</bvbg:AsstDesc>
                <bvbg:MktCd>10</bvbg:MktCd>
                <bvbg:SgmtCd>1</bvbg:SgmtCd>
                <bvbg:TckrSymb>VALE3</bvbg:TckrSymb>
                <bvbg:ISIN>BRVALEACNOR0</bvbg:ISIN>
                <bvbg:RptDt>2026-03-01</bvbg:RptDt>
              </bvbg:Instrm>
            </bvbg:InstrmInf>
          </bvbg:Document>
        </bvbg:Pyld>
      </bvbg:BizData>
    </bvbg:Document>
    "#;

    let parsed = parse_bvbg_028(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].security_id, "20000001");
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("VALE3"));
    assert_eq!(parsed.rows[0].isin.as_deref(), Some("BRVALEACNOR0"));
    assert_eq!(parsed.rows[0].asset.as_deref(), Some("VALE"));
    assert_eq!(
        parsed.rows[0].asset_description.as_deref(),
        Some("VALE S.A. ON")
    );
}

/// Synthetic nested XML test for BVBG.028.
///
/// NOTE: This fixture is explicitly SYNTHETIC to test nested tag recursion.
/// It does NOT represent a verified B3 BVBG.028 production schema or versioned XSD,
/// as no raw production XML has been captured in the repository.
/// Official-file compatibility for BVBG.028 remains UNPROVEN pending acquisition
/// and homologation of verified source artifacts under Issue 01 and Issue 03.
#[test]
fn test_parse_bvbg_028_synthetic_nested_xml() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
    <bvbg:Document xmlns:bvbg="urn:bvmf.052.01.xsd" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
      <bvbg:BizData>
        <bvbg:Pyld>
          <bvbg:Document>
            <bvbg:InstrmInf>
              <bvbg:Instrm>
                <bvbg:FinInstrmId>
                  <bvbg:OthrId>
                    <bvbg:Id>20000001</bvbg:Id>
                    <bvbg:Tp>
                      <bvbg:Prtry>SctyId</bvbg:Prtry>
                    </bvbg:Tp>
                  </bvbg:OthrId>
                  <bvbg:PlcOfListg>
                    <bvbg:MktIdrCd>BVMF</bvbg:MktIdrCd>
                  </bvbg:PlcOfListg>
                </bvbg:FinInstrmId>
                <bvbg:FinInstrmGnlAttrbts>
                  <bvbg:PlrtyTckrSymb>VALE3</bvbg:PlrtyTckrSymb>
                  <bvbg:ISIN>BRVALEACNOR0</bvbg:ISIN>
                  <bvbg:AsstDesc>VALE S.A. ON</bvbg:AsstDesc>
                  <bvbg:Asst>VALE</bvbg:Asst>
                </bvbg:FinInstrmGnlAttrbts>
                <bvbg:TradDt>
                  <bvbg:Dt>2026-03-01</bvbg:Dt>
                </bvbg:TradDt>
              </bvbg:Instrm>
            </bvbg:InstrmInf>
          </bvbg:Document>
        </bvbg:Pyld>
      </bvbg:BizData>
    </bvbg:Document>
    "#;

    let parsed = parse_bvbg_028(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].security_id, "20000001");
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("VALE3"));
    assert_eq!(parsed.rows[0].isin.as_deref(), Some("BRVALEACNOR0"));
    assert_eq!(
        parsed.rows[0].market_identifier_code.as_deref(),
        Some("BVMF")
    );
    assert_eq!(
        parsed.rows[0].asset_description.as_deref(),
        Some("VALE S.A. ON")
    );
    assert_eq!(
        parsed.rows[0].reference_date,
        NaiveDate::from_ymd_opt(2026, 3, 1)
    );
}

#[test]
fn test_parse_bvbg_086_synthetic_simplified_xml() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
    <Document xmlns="urn:bvmf.086.01.xsd">
      <PricRpt>
        <TradDt>2026-03-01</TradDt>
        <TckrSymb>PETR4</TckrSymb>
        <SctyId>10000001</SctyId>
        <FrstPric>35.50</FrstPric>
        <MinPric>35.10</MinPric>
        <MaxPric>36.20</MaxPric>
        <TradAvrgPric>35.80</TradAvrgPric>
        <LastPric>36.00</LastPric>
        <NtlRglrVol>125000000.00</NtlRglrVol>
      </PricRpt>
    </Document>
    "#;

    let parsed = parse_bvbg_086(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("PETR4"));
    assert_eq!(parsed.rows[0].last_price, Some(Decimal::new(3600, 2)));
    assert_eq!(parsed.rows[0].first_price, Some(Decimal::new(3550, 2)));
}

/// Source-backed nested ISO 20022 structure test for BVBG.086 (PriceReport):
/// - Provenance: B3 Catálogo Preços e Índices v1.2.1 (07/10/2021), message family `bvmf.217.01` (R13 in docs/RESEARCH.md).
/// - Element hierarchy:
///   - Message family: `urn:bvmf.217.01.xsd`
///   - Trade Date: `PricRpt/TradDt/Dt`
///   - Numeric SecurityID: `PricRpt/FinInstrmId/OthrId/Id`
///   - Market Identifier Code: `PricRpt/FinInstrmId/PlcOfListg/MktIdrCd`
///   - Ticker Symbol: `PricRpt/SctyId/TckrSymb`
///   - Prices: `FrstPric`, `MinPric`, `MaxPric`, `TradAvrgPric`, `LastPric`, `NtlRglrVol`
///
/// NOTE: While this fixture reflects the documented 2021 catalog specification, full
/// compatibility with current production archives remains unproven until live samples
/// are verified under Issue 01 and Issue 07.
#[test]
fn test_parse_bvbg_086_catalog_nested_iso_20022() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
    <Document xmlns="urn:bvmf.217.01.xsd" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
      <PricRpt>
        <TradDt>
          <Dt>2026-03-01</Dt>
        </TradDt>
        <FinInstrmId>
          <OthrId>
            <Id>10000001</Id>
            <Tp>
              <Prtry>SctyId</Prtry>
            </Tp>
          </OthrId>
          <PlcOfListg>
            <MktIdrCd>BVMF</MktIdrCd>
          </PlcOfListg>
        </FinInstrmId>
        <SctyId>
          <TckrSymb>PETR4</TckrSymb>
        </SctyId>
        <FrstPric>35.50</FrstPric>
        <MinPric>35.10</MinPric>
        <MaxPric>36.20</MaxPric>
        <TradAvrgPric>35.80</TradAvrgPric>
        <LastPric>36.00</LastPric>
        <NtlRglrVol>125000000.00</NtlRglrVol>
      </PricRpt>
    </Document>
    "#;

    let parsed = parse_bvbg_086(Cursor::new(xml)).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].security_id.as_deref(), Some("10000001"));
    assert_eq!(parsed.rows[0].ticker_symbol.as_deref(), Some("PETR4"));
    assert_eq!(
        parsed.rows[0].market_identifier_code.as_deref(),
        Some("BVMF")
    );
    assert_eq!(
        parsed.rows[0].trade_date,
        NaiveDate::from_ymd_opt(2026, 3, 1)
    );
    assert_eq!(parsed.rows[0].last_price, Some(Decimal::new(3600, 2)));
    assert_eq!(parsed.rows[0].first_price, Some(Decimal::new(3550, 2)));
}
