use tucano_b3::models::ibov::*;

#[test]
fn test_ibov_history_entry_serialization() {
    let entry = IbovHistoryEntry {
        data: "2026-11-07".to_string(),
        codigo: "IBOV".to_string(),
        nome: Some("IBOVESPA".to_string()),
        tipo: Some("INDICE".to_string()),
        pre_abr: Some(108230.45),
        pre_max: Some(108500.12),
        pre_min: Some(107900.33),
        pre_med: Some(108200.78),
        pre_ult: Some(108450.67),
        vol_tot: Some(15234567890.0),
        num_neg: Some(450123),
        qtd_tot: Some(1234567890.0),
        var_pct: Some(0.23),
    };

    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("IBOV"));
    assert!(json.contains("108450.67"));
}

#[test]
fn test_ibov_latest_response_serialization() {
    let response = IbovLatestResponse {
        codigo: "IBOV".to_string(),
        nome: "IBOVESPA".to_string(),
        data: "2026-11-07".to_string(),
        pre_abr: Some(108230.45),
        pre_max: Some(108500.12),
        pre_min: Some(107900.33),
        pre_med: Some(108200.78),
        pre_ult: Some(108450.67),
        vol_tot: Some(15234567890.0),
        num_neg: Some(450123),
        qtd_tot: Some(1234567890.0),
        var_pct: Some(0.23),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("IBOVESPA"));
}

#[test]
fn test_ibov_history_response_serialization() {
    let response = IbovHistoryResponse {
        codigo: "IBOV".to_string(),
        nome: "IBOVESPA".to_string(),
        history: vec![],
        paginacao: tucano_b3::models::pagination::PaginationMeta::new(0, 1, 50),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("paginacao"));
}
