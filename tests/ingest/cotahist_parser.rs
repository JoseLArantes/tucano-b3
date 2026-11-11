use std::io::Cursor;
use tucano_b3::ingest::cotahist_parser::*;

fn build_valid_quote_line() -> String {
    let mut line = String::new();
    line.push_str("01"); // TIPOREG (0..2)
    line.push_str("20260102"); // DATA_PREGAO (2..10)
    line.push_str("  "); // CODBDI (10..12)
    line.push_str("VALE3       "); // CODNEG (12..24)
    line.push_str("010"); // TPMERC (24..27)
    line.push_str("VALE S.A.   "); // NOMRES (27..39)
    line.push_str("ON        "); // ESPECI (39..49)
    line.push_str("   "); // PRAZOT (49..52)
    line.push_str(" 2  "); // FAT_COTACAO (52..56, 52..53 has ' 2')
    line.push_str("0000000006205"); // PREABR (56..69)
    line.push_str("0000000006289"); // PREMAX (69..82)
    line.push_str("0000000006172"); // PREMIN (82..95)
    line.push_str("0000000006231"); // PREMED (95..108)
    line.push_str("0000000006217"); // PREULT (108..121)
    line.push_str("0000000006215"); // PREOFC (121..134)
    line.push_str("0000000006219"); // PREOFV (134..147)
    line.push_str("00245"); // TOTNEG (147..152)
    line.push_str("000000000029847362"); // QUATOT (152..170)
    line.push_str("000000184729344000"); // VOLTOT (170..188)
    line.push_str("   "); // FILLER (188..191)
    line.push_str("BRVALEACNPA3"); // CODISIN (191..203)
                                   // Pad to 249
    while line.len() < 249 {
        line.push(' ');
    }
    line
}

#[test]
fn test_parse_empty_input() {
    let input = Cursor::new(b"");
    let (records, stats) = CotahistParser::parse(input).unwrap();
    assert!(records.is_empty());
    assert_eq!(stats.lines_total, 0);
}

#[test]
fn test_parse_header_line() {
    let header = "00COTAHIST.2026".to_string() + &" ".repeat(235);
    let input = Cursor::new(header.into_bytes());
    let (records, stats) = CotahistParser::parse(input).unwrap();
    assert!(records.is_empty());
    assert_eq!(stats.lines_skipped, 1);
}

#[test]
fn test_parse_trailer_line() {
    let trailer = "02COTAHIST.2026".to_string() + &" ".repeat(235);
    let input = Cursor::new(trailer.into_bytes());
    let (records, stats) = CotahistParser::parse(input).unwrap();
    assert!(records.is_empty());
    assert_eq!(stats.lines_skipped, 1);
}

#[test]
fn test_parse_quote_line() {
    let line = build_valid_quote_line();
    let input = Cursor::new(line.into_bytes());
    let (records, stats) = CotahistParser::parse(input).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(stats.lines_parsed, 1);

    let record = &records[0];
    assert_eq!(record.ticker, "VALE3");
    assert_eq!(
        record.date,
        chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()
    );
    assert_eq!(record.year, 2026);
    assert_eq!(record.company_name.as_deref(), Some("VALE S.A."));
    assert_eq!(record.isin.as_deref(), Some("BRVALEACNPA3"));
    assert_eq!(record.market_type, Some(10));
    assert!(record.pre_ult.is_some());
    assert_eq!(record.pre_abr, Some(62.05));
    assert_eq!(record.pre_max, Some(62.89));
    assert_eq!(record.pre_min, Some(61.72));
    assert_eq!(record.pre_med, Some(62.31));
    assert_eq!(record.pre_ult, Some(62.17));
    assert_eq!(record.pre_ofe_compra, Some(62.15));
    assert_eq!(record.pre_ofe_venda, Some(62.19));
    assert_eq!(record.tot_neg, Some(245));
    assert_eq!(record.quat_tot, Some(29847362));
    assert_eq!(record.vol_tot, Some(184729344000.0));
}

#[test]
fn test_normalize_ticker() {
    assert_eq!(CotahistParser::normalize_ticker("PETR4"), "PETR4");
    assert_eq!(CotahistParser::normalize_ticker("PETR4F"), "PETR4");
    assert_eq!(CotahistParser::normalize_ticker("PETR4A"), "PETR4");
    assert_eq!(CotahistParser::normalize_ticker("VALE3 "), "VALE3");
    assert_eq!(CotahistParser::normalize_ticker("vale3"), "VALE3");
}

#[test]
fn test_parse_implicit_decimal() {
    let result = CotahistParser::parse_implicit_decimal("0000000001234", 2, 1, "TEST").unwrap();
    assert_eq!(result, Some(12.34));

    let result = CotahistParser::parse_implicit_decimal("0000000000000", 2, 1, "TEST").unwrap();
    assert_eq!(result, Some(0.0));

    let result = CotahistParser::parse_implicit_decimal("", 2, 1, "TEST").unwrap();
    assert_eq!(result, Some(0.0));

    let err = CotahistParser::parse_implicit_decimal("invalid", 2, 1, "TEST");
    assert!(err.is_err());
}

#[test]
fn test_detect_file_type() {
    assert_eq!(detect_file_type("COTAHIST_D2026.TXT"), "yearly");
    assert_eq!(detect_file_type("COTAHIST_A2025.TXT"), "yearly");
    assert_eq!(detect_file_type("COTAHIST_M202601.TXT"), "monthly");
    assert_eq!(detect_file_type("COTAHIST_20260102.TXT"), "daily");
    assert_eq!(detect_file_type("COTAHIST_D06112026.ZIP"), "daily");
    assert_eq!(detect_file_type("random.txt"), "unknown");
}

#[test]
fn test_parse_invalid_lines_skipped_and_logged() {
    let mut content = String::new();
    // Valid line
    let line1 = build_valid_quote_line();
    content.push_str(&line1);
    content.push('\n');

    // Too short line
    content.push_str("short line\n");

    // Invalid record type line
    content.push_str(&("99invalid_record_type".to_string() + &" ".repeat(230) + "\n"));

    // Invalid date line
    let mut line4 = line1.clone();
    line4.replace_range(2..10, "99999999");
    content.push_str(&line4);
    content.push('\n');

    // Empty line
    content.push('\n');

    let input = Cursor::new(content.into_bytes());
    let (records, stats) = CotahistParser::parse(input).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(stats.lines_total, 5);
    assert_eq!(stats.lines_parsed, 1);
    assert_eq!(stats.lines_skipped, 1); // empty line
    assert_eq!(stats.lines_errored, 3); // short line, invalid record type, invalid date
}
