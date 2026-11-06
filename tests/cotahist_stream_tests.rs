use rust_decimal::Decimal;
use std::io::{BufReader, Cursor};
use tucano_b3::parsers::cotahist_stream::{parse_cotahist_stream, CotahistEvent};

fn detail() -> Vec<u8> {
    let mut line = vec![b' '; 245];
    let mut set = |a: usize, b: usize, value: &[u8]| {
        line[a - 1..b].copy_from_slice(value);
    };
    set(1, 2, b"01");
    set(3, 10, b"20251230");
    set(11, 12, b"02");
    set(13, 24, b"EXMP3       ");
    set(25, 27, b"010");
    set(28, 39, b"A\xc7\xc3O TESTE  ");
    set(50, 52, b"000");
    set(53, 56, b"BRL ");
    for (a, b) in [
        (57, 69),
        (70, 82),
        (83, 95),
        (96, 108),
        (109, 121),
        (122, 134),
        (135, 147),
        (189, 201),
        (218, 230),
    ] {
        set(a, b, b"0000000001234");
    }
    set(148, 152, b"00001");
    set(153, 170, b"000000000000000100");
    set(171, 188, b"000000000000001234");
    set(202, 202, b"0");
    set(203, 210, b"99991231");
    set(211, 217, b"0000001");
    set(231, 242, b"BREXMPACNOR0");
    set(243, 245, b"001");
    line
}
fn file(details: &[Vec<u8>], count: Option<usize>, crlf: bool) -> Vec<u8> {
    let mut header = vec![b' '; 245];
    header[..2].copy_from_slice(b"00");
    let mut trailer = vec![b' '; 245];
    trailer[..2].copy_from_slice(b"99");
    trailer[31..42]
        .copy_from_slice(format!("{:011}", count.unwrap_or(details.len() + 2)).as_bytes());
    let newline = if crlf {
        b"\r\n".as_slice()
    } else {
        b"\n".as_slice()
    };
    let mut out = header;
    out.extend(newline);
    for line in details {
        out.extend(line);
        out.extend(newline);
    }
    out.extend(trailer);
    out
}

#[test]
fn bytes_encoding_precision_and_locations_survive_streaming() {
    let bytes = file(&[detail()], None, true);
    let mut seen = 0;
    let summary =
        parse_cotahist_stream(BufReader::with_capacity(17, Cursor::new(bytes)), |event| {
            let CotahistEvent::Valid {
                row,
                line_number,
                byte_offset,
            } = event
            else {
                panic!("unexpected quarantine")
            };
            assert_eq!(line_number, 2);
            assert_eq!(byte_offset, 247);
            assert_eq!(row.issuer_short_name.as_deref(), Some("AÇÃO TESTE"));
            assert_eq!(row.last_price, Some(Decimal::new(1234, 2)));
            assert_eq!(row.distribution_number.as_deref(), Some("001"));
            assert_eq!(row.maturity_date, None);
            seen += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(seen, 1);
    assert_eq!(summary.physical_records, 3);
    assert_eq!(summary.rejected_records, 0);
}

#[test]
fn invalid_detail_is_quarantined_without_losing_valid_neighbors() {
    let mut bad = detail();
    bad[108] = b'X';
    let bytes = file(&[detail(), bad.clone(), detail()], None, false);
    let mut positions = Vec::new();
    let summary = parse_cotahist_stream(Cursor::new(bytes), |event| {
        if let CotahistEvent::Rejected {
            raw,
            line_number,
            byte_offset,
            ..
        } = event
        {
            assert_eq!(raw, bad);
            positions.push((line_number, byte_offset));
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(summary.detail_records, 3);
    assert_eq!(summary.valid_records, 2);
    assert_eq!(summary.rejected_records, 1);
    assert_eq!(positions, [(3, 492)]);
}

#[test]
fn structural_failures_never_report_a_complete_member() {
    for data in [
        file(&[detail()], Some(1), false),
        b"01short".to_vec(),
        vec![b'0'; 1_000_000],
    ] {
        assert!(parse_cotahist_stream(Cursor::new(data), |_| Ok(())).is_err());
    }
    let mut after = file(&[detail()], None, false);
    after.extend(b"\n\n");
    assert!(parse_cotahist_stream(Cursor::new(after), |_| Ok(())).is_err());
    let mut missing = file(&[detail()], None, false);
    missing.truncate(missing.len() - 245);
    assert!(parse_cotahist_stream(Cursor::new(missing), |_| Ok(())).is_err());
}

#[test]
fn callback_failure_stops_parsing() {
    let error = parse_cotahist_stream(Cursor::new(file(&[detail()], None, false)), |_| {
        Err(tucano_b3::error::AppError::Internal("stop".into()))
    })
    .unwrap_err();
    assert!(error.to_string().contains("stop"));
}
