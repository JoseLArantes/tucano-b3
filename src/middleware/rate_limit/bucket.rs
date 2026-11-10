//! Token bucket rate limiter.
//!
//! Each client (identified by API key or IP) gets a bucket.
//! Tokens are replenished at a fixed rate.
//! Each request consumes one token.
//! When the bucket is empty, the request is rejected with 429.

use std::time::Instant;

/// A single token bucket for one client.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Maximum number of tokens (burst capacity)
    max_tokens: f64,
    /// Current number of tokens
    tokens: f64,
    /// Token replenishment rate (tokens per second)
    refill_rate: f64,
    /// Last time tokens were replenished
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// `requests_per_minute`: the sustained rate limit.
    /// Burst capacity equals the per-minute limit (allows short bursts).
    pub fn new(requests_per_minute: u32) -> Self {
        let max_tokens = requests_per_minute as f64;
        let refill_rate = requests_per_minute as f64 / 60.0; // tokens per second

        Self {
            max_tokens,
            tokens: max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token.
    ///
    /// Returns Ok(()) if a token was available.
    /// Returns Err(seconds_until_next_token) if the bucket is empty.
    pub fn try_acquire(&mut self) -> Result<(), f64> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate seconds until next token
            let deficit = 1.0 - self.tokens;
            let wait_seconds = deficit / self.refill_rate;
            Err(wait_seconds)
        }
    }

    /// Replenish tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Get current token count (for metrics/debugging).
    pub fn available_tokens(&self) -> f64 {
        self.tokens
    }
}
