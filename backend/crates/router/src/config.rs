use crate::error::RoutingError;

// ---------------------------------------------------------------------------
// Top-level router config
// ---------------------------------------------------------------------------

/// Top-level configuration for the routing engine.
#[derive(Debug, Clone, Default)]
pub struct RouterConfig {
    pub scoring: ScoringWeights,
    pub circuit_breaker: CircuitBreakerConfig,
    pub idempotency: IdempotencyConfig,
}

// ---------------------------------------------------------------------------
// Scoring weights
// ---------------------------------------------------------------------------

/// Weights for the multi-factor provider scoring algorithm.
///
/// Each weight must be finite and non-negative. Weights do NOT need to sum
/// to 1.0 — the scorer uses relative ratios, so `{cost: 3, speed: 2}` is
/// equivalent to `{cost: 0.3, speed: 0.2}`.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    /// Weight for estimated transaction fee (lower fee = higher score).
    pub cost: f64,
    /// Weight for latency (lower p50 = higher score).
    pub speed: f64,
    /// Weight for error rate (lower error rate = higher score).
    pub health: f64,
    /// Weight for matching the agent's preferred rail.
    pub preference: f64,
}

impl ScoringWeights {
    /// Validate that all weights are finite and non-negative.
    pub fn validate(&self) -> Result<(), RoutingError> {
        let fields = [
            ("cost", self.cost),
            ("speed", self.speed),
            ("health", self.health),
            ("preference", self.preference),
        ];
        for (name, value) in fields {
            if !value.is_finite() || value < 0.0 {
                return Err(RoutingError::Config(format!(
                    "scoring weight '{name}' must be finite and non-negative, got {value}"
                )));
            }
        }
        if self.cost + self.speed + self.health + self.preference == 0.0 {
            return Err(RoutingError::Config(
                "at least one scoring weight must be non-zero".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            cost: 0.3,
            speed: 0.2,
            health: 0.3,
            preference: 0.2,
        }
    }
}

// ---------------------------------------------------------------------------
// Circuit breaker config
// ---------------------------------------------------------------------------

/// Configuration for the per-provider circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Error rate threshold (0.0–1.0) to trip the breaker from Closed to Open.
    pub error_threshold: f64,
    /// Rolling window in seconds for error rate calculation.
    pub window_secs: u64,
    /// Seconds to wait in Open before transitioning to HalfOpen.
    pub cooldown_secs: u64,
    /// Max requests allowed through in HalfOpen before deciding.
    pub half_open_max_requests: u32,
}

impl CircuitBreakerConfig {
    /// Validate that the configuration is well-formed.
    pub fn validate(&self) -> Result<(), RoutingError> {
        if !self.error_threshold.is_finite()
            || self.error_threshold < 0.0
            || self.error_threshold > 1.0
        {
            return Err(RoutingError::Config(format!(
                "error_threshold must be in [0.0, 1.0], got {}",
                self.error_threshold
            )));
        }
        if self.window_secs == 0 {
            return Err(RoutingError::Config("window_secs must be > 0".to_string()));
        }
        // cooldown_secs == 0 is valid: means "retry on next request" (instant HalfOpen)
        if self.half_open_max_requests == 0 {
            return Err(RoutingError::Config(
                "half_open_max_requests must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_threshold: 0.5,
            window_secs: 300,
            cooldown_secs: 60,
            half_open_max_requests: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Idempotency config
// ---------------------------------------------------------------------------

/// Configuration for the idempotency lock.
#[derive(Debug, Clone)]
pub struct IdempotencyConfig {
    /// TTL for the Redis lock in seconds.
    pub lock_ttl_secs: u64,
}

impl IdempotencyConfig {
    /// Validate that the configuration is well-formed.
    pub fn validate(&self) -> Result<(), RoutingError> {
        if self.lock_ttl_secs == 0 {
            return Err(RoutingError::Config(
                "lock_ttl_secs must be > 0 — a zero TTL would either never expire \
                 (permanent payment block) or expire instantly (no idempotency protection)"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self { lock_ttl_secs: 300 }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scoring_weights() {
        let w = ScoringWeights::default();
        assert!((w.cost - 0.3).abs() < f64::EPSILON);
        assert!((w.speed - 0.2).abs() < f64::EPSILON);
        assert!((w.health - 0.3).abs() < f64::EPSILON);
        assert!((w.preference - 0.2).abs() < f64::EPSILON);
        assert!(w.validate().is_ok());
    }

    #[test]
    fn default_circuit_breaker_config() {
        let c = CircuitBreakerConfig::default();
        assert!((c.error_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(c.window_secs, 300);
        assert_eq!(c.cooldown_secs, 60);
        assert_eq!(c.half_open_max_requests, 3);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn scoring_weights_rejects_negative() {
        let w = ScoringWeights {
            cost: -0.1,
            ..Default::default()
        };
        let err = w.validate().unwrap_err();
        assert!(err.to_string().contains("cost"));
    }

    #[test]
    fn scoring_weights_rejects_nan() {
        let w = ScoringWeights {
            health: f64::NAN,
            ..Default::default()
        };
        let err = w.validate().unwrap_err();
        assert!(err.to_string().contains("health"));
    }

    #[test]
    fn circuit_breaker_rejects_threshold_above_one() {
        let c = CircuitBreakerConfig {
            error_threshold: 1.5,
            ..Default::default()
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn circuit_breaker_rejects_zero_window() {
        let c = CircuitBreakerConfig {
            window_secs: 0,
            ..Default::default()
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn default_idempotency_config() {
        let c = IdempotencyConfig::default();
        assert_eq!(c.lock_ttl_secs, 300);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn idempotency_config_rejects_zero_ttl() {
        let c = IdempotencyConfig { lock_ttl_secs: 0 };
        let err = c.validate().unwrap_err();
        assert!(err.to_string().contains("lock_ttl_secs"));
    }

    #[test]
    fn idempotency_config_accepts_nonzero_ttl() {
        let c = IdempotencyConfig { lock_ttl_secs: 1 };
        assert!(c.validate().is_ok());
    }

    // -----------------------------------------------------------------------
    // Phase 7.5: all-zero scoring weights rejected
    // -----------------------------------------------------------------------

    #[test]
    fn scoring_weights_rejects_all_zero() {
        let w = ScoringWeights {
            cost: 0.0,
            speed: 0.0,
            health: 0.0,
            preference: 0.0,
        };
        let err = w.validate().unwrap_err();
        assert!(err.to_string().contains("non-zero"));
    }
}
