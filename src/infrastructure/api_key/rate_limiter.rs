//! Rate limiter implementation
//!
//! Provides sliding window rate limiting for API keys.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::domain::api_key::RateLimitConfig;

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in the current window
    pub remaining: u32,
    /// Total limit for the window
    pub limit: u32,
    /// Time until the limit resets (in seconds)
    pub reset_in_seconds: u64,
    /// Which limit was hit (if any)
    pub limit_type: Option<LimitType>,
}

/// Type of rate limit that was hit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitType {
    PerMinute,
    PerHour,
    PerDay,
    TokensPerMinute,
}

impl std::fmt::Display for LimitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PerMinute => write!(f, "per_minute"),
            Self::PerHour => write!(f, "per_hour"),
            Self::PerDay => write!(f, "per_day"),
            Self::TokensPerMinute => write!(f, "tokens_per_minute"),
        }
    }
}

/// Request record for rate limiting
#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp: Instant,
    tokens: u32,
}

/// Rate limiter for API keys
#[derive(Debug)]
pub struct RateLimiter {
    /// Per-key request records
    records: Arc<RwLock<HashMap<String, Vec<RequestRecord>>>>,
    /// Cleanup interval
    cleanup_interval: Duration,
    /// Last cleanup time
    last_cleanup: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Check if a request is allowed under the rate limits
    pub async fn check(
        &self,
        key_id: &str,
        config: &RateLimitConfig,
        tokens: Option<u32>,
    ) -> RateLimitResult {
        self.maybe_cleanup().await;

        let now = Instant::now();
        let records = self.records.read().await;

        let key_records = records.get(key_id);
        let result = self.calculate_limits(key_records, config, tokens, now);

        result
    }

    /// Record a request
    pub async fn record(&self, key_id: &str, tokens: u32) {
        let mut records = self.records.write().await;
        let key_records = records.entry(key_id.to_string()).or_insert_with(Vec::new);

        key_records.push(RequestRecord {
            timestamp: Instant::now(),
            tokens,
        });
    }

    /// Check and record in one operation
    pub async fn check_and_record(
        &self,
        key_id: &str,
        config: &RateLimitConfig,
        tokens: Option<u32>,
    ) -> RateLimitResult {
        self.maybe_cleanup().await;

        let now = Instant::now();
        let mut records = self.records.write().await;

        let key_records = records.get(key_id);
        let result = self.calculate_limits(key_records, config, tokens, now);

        if result.allowed {
            let key_records = records.entry(key_id.to_string()).or_insert_with(Vec::new);
            key_records.push(RequestRecord {
                timestamp: now,
                tokens: tokens.unwrap_or(0),
            });
        }

        result
    }

    /// Reset rate limits for a key
    pub async fn reset(&self, key_id: &str) {
        let mut records = self.records.write().await;
        records.remove(key_id);
    }

    fn calculate_limits(
        &self,
        records: Option<&Vec<RequestRecord>>,
        config: &RateLimitConfig,
        tokens: Option<u32>,
        now: Instant,
    ) -> RateLimitResult {
        let records = match records {
            Some(r) => r,
            None => {
                return RateLimitResult {
                    allowed: true,
                    remaining: config.requests_per_minute.saturating_sub(1),
                    limit: config.requests_per_minute,
                    reset_in_seconds: 60,
                    limit_type: None,
                };
            }
        };

        let minute_ago = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);
        let hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let day_ago = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);

        let minute_count = records.iter().filter(|r| r.timestamp >= minute_ago).count() as u32;
        let hour_count = records.iter().filter(|r| r.timestamp >= hour_ago).count() as u32;
        let day_count = records.iter().filter(|r| r.timestamp >= day_ago).count() as u32;

        // Check per-minute limit
        if minute_count >= config.requests_per_minute {
            let oldest_in_window = records
                .iter()
                .filter(|r| r.timestamp >= minute_ago)
                .map(|r| r.timestamp)
                .min();

            let reset_in = oldest_in_window
                .map(|t| {
                    let elapsed = now.duration_since(t);
                    60u64.saturating_sub(elapsed.as_secs())
                })
                .unwrap_or(60);

            return RateLimitResult {
                allowed: false,
                remaining: 0,
                limit: config.requests_per_minute,
                reset_in_seconds: reset_in,
                limit_type: Some(LimitType::PerMinute),
            };
        }

        // Check per-hour limit
        if hour_count >= config.requests_per_hour {
            let oldest_in_window = records
                .iter()
                .filter(|r| r.timestamp >= hour_ago)
                .map(|r| r.timestamp)
                .min();

            let reset_in = oldest_in_window
                .map(|t| {
                    let elapsed = now.duration_since(t);
                    3600u64.saturating_sub(elapsed.as_secs())
                })
                .unwrap_or(3600);

            return RateLimitResult {
                allowed: false,
                remaining: 0,
                limit: config.requests_per_hour,
                reset_in_seconds: reset_in,
                limit_type: Some(LimitType::PerHour),
            };
        }

        // Check per-day limit
        if day_count >= config.requests_per_day {
            let oldest_in_window = records
                .iter()
                .filter(|r| r.timestamp >= day_ago)
                .map(|r| r.timestamp)
                .min();

            let reset_in = oldest_in_window
                .map(|t| {
                    let elapsed = now.duration_since(t);
                    86400u64.saturating_sub(elapsed.as_secs())
                })
                .unwrap_or(86400);

            return RateLimitResult {
                allowed: false,
                remaining: 0,
                limit: config.requests_per_day,
                reset_in_seconds: reset_in,
                limit_type: Some(LimitType::PerDay),
            };
        }

        // Check token limit if applicable
        if let (Some(token_limit), Some(request_tokens)) = (config.tokens_per_minute, tokens) {
            let minute_tokens: u32 = records
                .iter()
                .filter(|r| r.timestamp >= minute_ago)
                .map(|r| r.tokens)
                .sum();

            if minute_tokens + request_tokens > token_limit {
                return RateLimitResult {
                    allowed: false,
                    remaining: token_limit.saturating_sub(minute_tokens),
                    limit: token_limit,
                    reset_in_seconds: 60,
                    limit_type: Some(LimitType::TokensPerMinute),
                };
            }
        }

        RateLimitResult {
            allowed: true,
            remaining: config.requests_per_minute.saturating_sub(minute_count + 1),
            limit: config.requests_per_minute,
            reset_in_seconds: 60,
            limit_type: None,
        }
    }

    async fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read().await;
            last.elapsed() >= self.cleanup_interval
        };

        if should_cleanup {
            let mut last = self.last_cleanup.write().await;
            *last = Instant::now();

            let now = Instant::now();
            let cutoff = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);

            let mut records = self.records.write().await;

            for key_records in records.values_mut() {
                key_records.retain(|r| r.timestamp >= cutoff);
            }

            records.retain(|_, v| !v.is_empty());
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RateLimitConfig {
        RateLimitConfig::new(10, 100, 1000)
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_first_request() {
        let limiter = RateLimiter::new();
        let config = default_config();

        let result = limiter.check("key1", &config, None).await;

        assert!(result.allowed);
        assert_eq!(result.remaining, 9);
        assert_eq!(result.limit, 10);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(2, 100, 1000);

        // Make two requests
        limiter.check_and_record("key1", &config, None).await;
        limiter.check_and_record("key1", &config, None).await;

        // Third should be blocked
        let result = limiter.check("key1", &config, None).await;

        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert_eq!(result.limit_type, Some(LimitType::PerMinute));
    }

    #[tokio::test]
    async fn test_rate_limiter_different_keys() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, 100, 1000);

        limiter.check_and_record("key1", &config, None).await;

        // Different key should still be allowed
        let result = limiter.check("key2", &config, None).await;
        assert!(result.allowed);

        // Same key should be blocked
        let result = limiter.check("key1", &config, None).await;
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, 100, 1000);

        limiter.check_and_record("key1", &config, None).await;

        let result = limiter.check("key1", &config, None).await;
        assert!(!result.allowed);

        limiter.reset("key1").await;

        let result = limiter.check("key1", &config, None).await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_token_limit() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(100, 1000, 10000).with_tokens_per_minute(100);

        // Use 60 tokens
        limiter.check_and_record("key1", &config, Some(60)).await;

        // 50 more should fail (would be 110)
        let result = limiter.check("key1", &config, Some(50)).await;
        assert!(!result.allowed);
        assert_eq!(result.limit_type, Some(LimitType::TokensPerMinute));

        // 40 more should succeed (would be 100)
        let result = limiter.check("key1", &config, Some(40)).await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_unlimited() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::unlimited();

        for _ in 0..1000 {
            let result = limiter.check_and_record("key1", &config, None).await;
            assert!(result.allowed);
        }
    }
}
