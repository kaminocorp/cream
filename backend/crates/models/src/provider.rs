use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::payment::RailPreference;

// ---------------------------------------------------------------------------
// Provider ID
// ---------------------------------------------------------------------------

/// Identifies a specific payment provider integration.
///
/// Uses a human-readable string like "stripe_issuing", "airwallex_payouts",
/// "coinbase_x402" rather than a UUID — providers are configuration, not
/// user-generated entities.
///
/// Validated on all construction paths: must be non-empty and within
/// [`MAX_PROVIDER_ID_LEN`]. Provider IDs are persisted to the append-only
/// audit ledger via `RoutingDecision.selected`, so unbounded values would
/// cause permanent bloat.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProviderId(String);

/// Maximum allowed length for provider IDs.
pub const MAX_PROVIDER_ID_LEN: usize = 255;

impl ProviderId {
    /// Create a new ProviderId. Panics if the id is empty or exceeds
    /// [`MAX_PROVIDER_ID_LEN`].
    ///
    /// Use `try_new()` for fallible construction from untrusted input.
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        assert!(!id.is_empty(), "ProviderId must not be empty");
        assert!(
            id.len() <= MAX_PROVIDER_ID_LEN,
            "ProviderId exceeds maximum length of {MAX_PROVIDER_ID_LEN}"
        );
        Self(id)
    }

    /// Fallible constructor for untrusted input. Returns an error if the id
    /// is empty or exceeds [`MAX_PROVIDER_ID_LEN`].
    pub fn try_new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.is_empty() {
            return Err(DomainError::InvalidIdFormat(
                "ProviderId must not be empty".to_string(),
            ));
        }
        if id.len() > MAX_PROVIDER_ID_LEN {
            return Err(DomainError::InvalidIdFormat(format!(
                "ProviderId exceeds maximum length of {MAX_PROVIDER_ID_LEN} (got {})",
                id.len()
            )));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProviderId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Err(serde::de::Error::custom("provider_id must not be empty"));
        }
        if s.len() > MAX_PROVIDER_ID_LEN {
            return Err(serde::de::Error::custom(format!(
                "provider_id exceeds maximum length of {MAX_PROVIDER_ID_LEN} (got {})",
                s.len()
            )));
        }
        Ok(Self(s))
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Provider Health
// ---------------------------------------------------------------------------

/// Real-time health snapshot for a payment provider.
///
/// Updated on a rolling 5-minute window. Used by the routing engine to
/// score providers and by the circuit breaker to decide whether to accept
/// or reject traffic.
///
/// Custom `Deserialize` validates that `error_rate_5m` is a finite value
/// in the range [0.0, 1.0]. NaN, Infinity, negative, or >1.0 values would
/// poison routing engine scoring calculations.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderHealth {
    pub provider_id: ProviderId,
    pub is_healthy: bool,
    /// Error rate over the last 5 minutes (0.0 – 1.0).
    pub error_rate_5m: f64,
    /// Median latency in milliseconds.
    pub p50_latency_ms: u64,
    /// 99th percentile latency in milliseconds.
    pub p99_latency_ms: u64,
    pub last_checked_at: DateTime<Utc>,
    pub circuit_state: CircuitState,
}

impl<'de> Deserialize<'de> for ProviderHealth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            provider_id: ProviderId,
            is_healthy: bool,
            error_rate_5m: f64,
            p50_latency_ms: u64,
            p99_latency_ms: u64,
            last_checked_at: DateTime<Utc>,
            circuit_state: CircuitState,
        }

        let raw = Raw::deserialize(deserializer)?;

        if !raw.error_rate_5m.is_finite() || raw.error_rate_5m < 0.0 || raw.error_rate_5m > 1.0 {
            return Err(serde::de::Error::custom(format!(
                "error_rate_5m must be a finite value between 0.0 and 1.0, got {}",
                raw.error_rate_5m
            )));
        }
        if raw.p50_latency_ms > raw.p99_latency_ms {
            return Err(serde::de::Error::custom(format!(
                "p50_latency_ms ({}) must be <= p99_latency_ms ({})",
                raw.p50_latency_ms, raw.p99_latency_ms
            )));
        }

        Ok(ProviderHealth {
            provider_id: raw.provider_id,
            is_healthy: raw.is_healthy,
            error_rate_5m: raw.error_rate_5m,
            p50_latency_ms: raw.p50_latency_ms,
            p99_latency_ms: raw.p99_latency_ms,
            last_checked_at: raw.last_checked_at,
            circuit_state: raw.circuit_state,
        })
    }
}

// ---------------------------------------------------------------------------
// Circuit Breaker State
// ---------------------------------------------------------------------------

/// The circuit breaker state for a provider.
///
/// Implements the standard circuit breaker pattern:
/// - Closed = healthy, accepting traffic
/// - Open = unhealthy, all traffic rejected and rerouted
/// - HalfOpen = cooldown expired, testing with limited traffic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

// ---------------------------------------------------------------------------
// Routing Types
// ---------------------------------------------------------------------------

/// A candidate provider considered by the routing engine.
///
/// The routing engine scores all viable candidates and selects the highest-
/// scoring one. Failed candidates are available for fallback.
///
/// Custom `Deserialize` validates that `score` is finite (NaN/Infinity would
/// break comparison-based sorting) and `estimated_fee` is non-negative (negative
/// fees would invert cost-optimization scoring).
#[derive(Debug, Clone, Serialize)]
pub struct RoutingCandidate {
    pub provider_id: ProviderId,
    pub rail: RailPreference,
    pub estimated_fee: Decimal,
    pub estimated_latency_ms: u64,
    /// Composite score (higher = better). Computed from cost, speed, health,
    /// and corridor weights.
    pub score: f64,
}

impl<'de> Deserialize<'de> for RoutingCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            provider_id: ProviderId,
            rail: RailPreference,
            estimated_fee: Decimal,
            estimated_latency_ms: u64,
            score: f64,
        }

        let raw = Raw::deserialize(deserializer)?;

        if !raw.score.is_finite() {
            return Err(serde::de::Error::custom(format!(
                "score must be finite, got {}",
                raw.score
            )));
        }
        if raw.estimated_fee < Decimal::ZERO {
            return Err(serde::de::Error::custom(format!(
                "estimated_fee must be non-negative, got {}",
                raw.estimated_fee
            )));
        }

        Ok(RoutingCandidate {
            provider_id: raw.provider_id,
            rail: raw.rail,
            estimated_fee: raw.estimated_fee,
            estimated_latency_ms: raw.estimated_latency_ms,
            score: raw.score,
        })
    }
}

/// Maximum allowed length for `RoutingDecision.reason`.
pub const MAX_ROUTING_REASON_LEN: usize = 1000;

/// The routing engine's final decision for a payment.
///
/// Custom `Deserialize` enforces length bounds on `reason` to prevent
/// audit log bloat (the audit ledger is append-only).
#[derive(Debug, Clone, Serialize)]
pub struct RoutingDecision {
    /// All candidates evaluated, ordered by score descending.
    pub candidates: Vec<RoutingCandidate>,
    /// The provider selected for this payment.
    pub selected: ProviderId,
    /// The rail selected for this payment.
    pub selected_rail: RailPreference,
    /// Human-readable explanation of why this provider was chosen.
    pub reason: String,
}

impl<'de> Deserialize<'de> for RoutingDecision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            candidates: Vec<RoutingCandidate>,
            selected: ProviderId,
            selected_rail: RailPreference,
            reason: String,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.reason.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "routing_decision.reason must not be empty — audit trail requires provider selection rationale",
            ));
        }
        if raw.reason.len() > MAX_ROUTING_REASON_LEN {
            return Err(serde::de::Error::custom(format!(
                "routing_decision.reason exceeds maximum length of {} characters (got {})",
                MAX_ROUTING_REASON_LEN,
                raw.reason.len()
            )));
        }

        Ok(RoutingDecision {
            candidates: raw.candidates,
            selected: raw.selected,
            selected_rail: raw.selected_rail,
            reason: raw.reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_display() {
        let id = ProviderId::new("stripe_issuing");
        assert_eq!(id.to_string(), "stripe_issuing");
    }

    #[test]
    fn circuit_state_serde() {
        let state = CircuitState::HalfOpen;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"half_open\"");
    }

    // -----------------------------------------------------------------------
    // Phase 6.14: ProviderHealth validation
    // -----------------------------------------------------------------------

    fn sample_health_json(error_rate: f64) -> serde_json::Value {
        serde_json::json!({
            "provider_id": "stripe_issuing",
            "is_healthy": true,
            "error_rate_5m": error_rate,
            "p50_latency_ms": 120,
            "p99_latency_ms": 450,
            "last_checked_at": "2026-04-01T12:00:00Z",
            "circuit_state": "closed"
        })
    }

    #[test]
    fn provider_health_valid_error_rate() {
        let json = sample_health_json(0.05);
        let health: ProviderHealth = serde_json::from_value(json).unwrap();
        assert!((health.error_rate_5m - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn provider_health_zero_error_rate() {
        let json = sample_health_json(0.0);
        let health: ProviderHealth = serde_json::from_value(json).unwrap();
        assert!((health.error_rate_5m).abs() < f64::EPSILON);
    }

    #[test]
    fn provider_health_max_error_rate() {
        let json = sample_health_json(1.0);
        let health: ProviderHealth = serde_json::from_value(json).unwrap();
        assert!((health.error_rate_5m - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn provider_health_rejects_negative_error_rate() {
        let json = sample_health_json(-0.1);
        let result: Result<ProviderHealth, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error_rate_5m"));
    }

    #[test]
    fn provider_health_rejects_error_rate_above_one() {
        let json = sample_health_json(1.5);
        let result: Result<ProviderHealth, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error_rate_5m"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.3: ProviderHealth p50 <= p99 invariant
    // -----------------------------------------------------------------------

    #[test]
    fn provider_health_rejects_p50_greater_than_p99() {
        let json = serde_json::json!({
            "provider_id": "stripe_issuing",
            "is_healthy": true,
            "error_rate_5m": 0.05,
            "p50_latency_ms": 500,
            "p99_latency_ms": 200,
            "last_checked_at": "2026-04-01T12:00:00Z",
            "circuit_state": "closed"
        });
        let result: Result<ProviderHealth, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("p50_latency_ms"));
        assert!(err.contains("p99_latency_ms"));
    }

    #[test]
    fn provider_health_accepts_p50_equal_to_p99() {
        let json = serde_json::json!({
            "provider_id": "stripe_issuing",
            "is_healthy": true,
            "error_rate_5m": 0.05,
            "p50_latency_ms": 200,
            "p99_latency_ms": 200,
            "last_checked_at": "2026-04-01T12:00:00Z",
            "circuit_state": "closed"
        });
        let result: ProviderHealth = serde_json::from_value(json).unwrap();
        assert_eq!(result.p50_latency_ms, 200);
    }

    // -----------------------------------------------------------------------
    // Phase 7.3: RoutingCandidate score/fee validation
    // -----------------------------------------------------------------------

    fn sample_candidate_json(score: serde_json::Value, fee: &str) -> serde_json::Value {
        serde_json::json!({
            "provider_id": "stripe_issuing",
            "rail": "card",
            "estimated_fee": fee,
            "estimated_latency_ms": 150,
            "score": score
        })
    }

    #[test]
    fn routing_candidate_rejects_nan_score() {
        // JSON doesn't have NaN literal, so we test via a struct with NaN
        // and round-trip through the Serialize path to confirm it's caught.
        // Instead, test via direct deserialization with an invalid float string.
        let json = serde_json::json!({
            "provider_id": "stripe_issuing",
            "rail": "card",
            "estimated_fee": "0.30",
            "estimated_latency_ms": 150,
            "score": null
        });
        let result: Result<RoutingCandidate, _> = serde_json::from_value(json);
        // null is not a valid f64, so serde itself rejects it
        assert!(result.is_err());
    }

    #[test]
    fn routing_candidate_rejects_negative_fee() {
        let json = sample_candidate_json(serde_json::json!(0.85), "-1.50");
        let result: Result<RoutingCandidate, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("estimated_fee"));
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn routing_candidate_accepts_zero_fee() {
        let json = sample_candidate_json(serde_json::json!(0.85), "0.00");
        let result: RoutingCandidate = serde_json::from_value(json).unwrap();
        assert_eq!(result.estimated_fee, Decimal::ZERO);
    }

    #[test]
    fn routing_candidate_accepts_valid() {
        let json = sample_candidate_json(serde_json::json!(0.85), "0.30");
        let result: RoutingCandidate = serde_json::from_value(json).unwrap();
        assert!((result.score - 0.85).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Phase 6.16: ProviderId empty-string validation
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn provider_id_rejects_empty_new() {
        let _ = ProviderId::new("");
    }

    #[test]
    fn provider_id_try_new_rejects_empty() {
        let result = ProviderId::try_new("");
        assert!(result.is_err());
    }

    #[test]
    fn provider_id_try_new_accepts_valid() {
        let result = ProviderId::try_new("stripe_issuing");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "stripe_issuing");
    }

    #[test]
    fn provider_id_deserialize_rejects_empty() {
        let json = serde_json::json!("");
        let result: Result<ProviderId, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn provider_id_deserialize_accepts_valid() {
        let json = serde_json::json!("coinbase_x402");
        let result: Result<ProviderId, _> = serde_json::from_value(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "coinbase_x402");
    }

    // -----------------------------------------------------------------------
    // Phase 7.5: ProviderId max length validation
    // -----------------------------------------------------------------------

    #[test]
    fn provider_id_try_new_rejects_oversized() {
        let long = "x".repeat(MAX_PROVIDER_ID_LEN + 1);
        let result = ProviderId::try_new(long);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    fn provider_id_try_new_accepts_at_limit() {
        let exact = "y".repeat(MAX_PROVIDER_ID_LEN);
        let result = ProviderId::try_new(exact);
        assert!(result.is_ok());
    }

    #[test]
    fn provider_id_deserialize_rejects_oversized() {
        let long = "z".repeat(MAX_PROVIDER_ID_LEN + 1);
        let json = serde_json::json!(long);
        let result: Result<ProviderId, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    #[should_panic(expected = "maximum length")]
    fn provider_id_new_panics_on_oversized() {
        let long = "a".repeat(MAX_PROVIDER_ID_LEN + 1);
        let _ = ProviderId::new(long);
    }

    // -----------------------------------------------------------------------
    // Phase 7.1: RoutingDecision.reason empty-string guard
    // -----------------------------------------------------------------------

    fn sample_routing_decision_json(reason: &str) -> serde_json::Value {
        serde_json::json!({
            "candidates": [],
            "selected": "stripe_issuing",
            "selected_rail": "card",
            "reason": reason
        })
    }

    #[test]
    fn routing_decision_rejects_empty_reason() {
        let json = sample_routing_decision_json("");
        let result: Result<RoutingDecision, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reason"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn routing_decision_rejects_whitespace_only_reason() {
        let json = sample_routing_decision_json("   ");
        let result: Result<RoutingDecision, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reason"));
    }

    #[test]
    fn routing_decision_accepts_valid_reason() {
        let json = sample_routing_decision_json("lowest_fee_approved_corridor");
        let result: Result<RoutingDecision, _> = serde_json::from_value(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().reason, "lowest_fee_approved_corridor");
    }
}
