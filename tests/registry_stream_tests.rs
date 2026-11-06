use std::io::{BufReader, Cursor};
use tucano_b3::parsers::registry_stream::{
    parse_bvbg028_stream, parse_tradable_csv_stream, RegistryEvent,
};

#[test]
fn xml_uses_published_id_and_isolates_missing_id() {
    let xml=br#"<Document xmlns="urn:bvmf.052.01.xsd" xmlns:x="urn:foreign"><Instrm><FinInstrmId><OthrId><Id>123</Id></OthrId></FinInstrmId><x:SecurityId>wrong</x:SecurityId><TckrSymb>EXMP3</TckrSymb></Instrm><Instrm><TckrSymb>NOID3</TckrSymb></Instrm></Document>"#;
    let mut records = Vec::new();
    let summary = parse_bvbg028_stream(BufReader::with_capacity(7, Cursor::new(xml)), |event| {
        records.push(event);
        Ok(())
    })
    .unwrap();
    assert_eq!(summary.records, 2);
    assert_eq!(summary.valid_records, 1);
    assert_eq!(summary.rejected_records, 1);
    let RegistryEvent::Valid { row, .. } = &records[0] else {
        panic!()
    };
    assert_eq!(row.security_id, "123");
    assert_eq!(row.ticker_symbol.as_deref(), Some("EXMP3"));
    assert!(matches!(&records[1], RegistryEvent::Rejected { .. }));
}

#[test]
fn xml_failures_are_structural_and_large_tokens_are_bounded() {
    for xml in [
        "<Document><Instrm><SecurityId>1</SecurityId>",
        "<!DOCTYPE x [<!ENTITY a SYSTEM 'file:///etc/passwd'>]><Document/>",
        "<Document xmlns='urn:foreign'><Instrm><SecurityId>1</SecurityId></Instrm></Document>",
    ] {
        assert!(parse_bvbg028_stream(Cursor::new(xml), |_| Ok(())).is_err());
    }
    let xml=format!("<Document><Instrm><SecurityId>1</SecurityId><Desc><![CDATA[{}]]></Desc></Instrm></Document>","<>".repeat(100_000));
    assert!(
        parse_bvbg028_stream(BufReader::with_capacity(13, Cursor::new(xml)), |_| Ok(())).is_err()
    );
}

#[test]
fn csv_is_incremental_strict_and_never_invents_security_id() {
    let csv="reference_date;security_id;ticker;market_code;tradable\n2025-12-30;123;exmp3;010;Y\n2025-12-30;;NOID3;010;Y\n2025-12-30;456;EXMP4;010;N\n";
    let mut ids = Vec::new();
    let summary = parse_tradable_csv_stream(
        BufReader::with_capacity(9, Cursor::new(csv)),
        b';',
        |event| {
            if let RegistryEvent::Valid { row, .. } = event {
                ids.push(row.security_id.unwrap());
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(ids, ["123", "456"]);
    assert_eq!(summary.rejected_records, 1);
    assert!(parse_tradable_csv_stream(
        Cursor::new("reference_date;security_id;unknown\n2025-01-01;1;x"),
        b';',
        |_| Ok(())
    )
    .is_err());
    assert!(
        parse_tradable_csv_stream(Cursor::new("[{\"SecurityID\":1}]"), b';', |_| Ok(())).is_err()
    );
}
