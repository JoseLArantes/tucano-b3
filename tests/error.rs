use axum::http::StatusCode;
use tucano_b3::error::AppError;

#[test]
fn test_ticker_not_found_status() {
    let err = AppError::TickerNotFound {
        ticker: "XYZXYZ".to_string(),
    };
    assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(err.code(), "TICKER_NOT_FOUND");
}

#[test]
fn test_invalid_date_status() {
    let err = AppError::InvalidDate {
        value: "not-a-date".to_string(),
    };
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(err.code(), "INVALID_DATE");
}

#[test]
fn test_rate_limited_status() {
    let err = AppError::RateLimited;
    assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(err.code(), "RATE_LIMITED");
}

#[test]
fn test_internal_error_status() {
    let err = AppError::Internal(anyhow::anyhow!("something broke"));
    assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.code(), "INTERNAL_ERROR");
}

#[test]
fn test_error_message_is_portuguese() {
    let err = AppError::TickerNotFound {
        ticker: "XYZXYZ".to_string(),
    };
    let message = err.to_string();
    assert!(message.contains("não encontrado"));
    assert!(message.contains("XYZXYZ"));
}

#[test]
fn test_error_details_present() {
    let err = AppError::TickerNotFound {
        ticker: "XYZXYZ".to_string(),
    };
    assert!(err.details().is_some());
}

#[test]
fn test_error_details_absent_for_internal() {
    let err = AppError::Internal(anyhow::anyhow!("something broke"));
    assert!(err.details().is_none());
}
