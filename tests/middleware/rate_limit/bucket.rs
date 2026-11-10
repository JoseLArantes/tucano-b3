use std::time::Duration;
use tucano_b3::middleware::rate_limit::bucket::TokenBucket;

#[test]
fn test_new_bucket_has_full_tokens() {
    let bucket = TokenBucket::new(100);
    assert!(bucket.available_tokens() > 99.0);
}

#[test]
fn test_acquire_consumes_token() {
    let mut bucket = TokenBucket::new(10);
    let initial = bucket.available_tokens();
    assert!(bucket.try_acquire().is_ok());
    assert!(bucket.available_tokens() < initial);
}

#[test]
fn test_bucket_exhaustion() {
    let mut bucket = TokenBucket::new(3);

    // Consume all 3 tokens
    assert!(bucket.try_acquire().is_ok());
    assert!(bucket.try_acquire().is_ok());
    assert!(bucket.try_acquire().is_ok());

    // 4th request should fail
    let result = bucket.try_acquire();
    assert!(result.is_err());

    // Error should contain wait time > 0
    if let Err(wait) = result {
        assert!(wait > 0.0);
    }
}

#[test]
fn test_bucket_refills_over_time() {
    let mut bucket = TokenBucket::new(60); // 1 token/second

    // Consume all tokens
    for _ in 0..60 {
        let _ = bucket.try_acquire();
    }

    // Should be empty
    assert!(bucket.try_acquire().is_err());

    // Wait 1 second — should refill 1 token
    std::thread::sleep(Duration::from_millis(1100));
    assert!(bucket.try_acquire().is_ok());
}

#[test]
fn test_burst_capacity() {
    let mut bucket = TokenBucket::new(100);

    // Should allow burst of 100 requests immediately
    for _ in 0..100 {
        assert!(bucket.try_acquire().is_ok());
    }

    // 101st should fail
    assert!(bucket.try_acquire().is_err());
}
