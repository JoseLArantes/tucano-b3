use tucano_b3::models::envelope::{ApiResponse, ResponseMeta};

#[test]
fn test_api_response_envelope() {
    let response = ApiResponse::new("test_data", 1.5);
    assert_eq!(response.data, "test_data");
    assert!(!response.meta.cached);
    assert!(!response.meta.request_id.is_empty());
    assert!(!response.meta.timestamp.is_empty());
    assert!((response.meta.query_time_ms - 1.5).abs() < f64::EPSILON);
}

#[test]
fn test_api_response_envelope_cached() {
    let meta = ResponseMeta::cached(0.5);
    assert!(meta.cached);
    assert!(!meta.request_id.is_empty());
    assert!(!meta.timestamp.is_empty());
    assert!((meta.query_time_ms - 0.5).abs() < f64::EPSILON);
}
