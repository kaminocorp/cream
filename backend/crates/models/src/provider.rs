use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::payment::RailPreference;

// ---------------------------------------------------------------------------
// Provider ID
// ---------------------------------------------------------------------------

/// Identifies a specific payment provider integration.
///
/// Uses a human-readable string like "stripe_issuing", "airwallex_payouts",
/// "coinbase_x402" rather than a UUID — providers are configuration, not
/// user-generated entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Circuit Breaker State
// ---------------------------------------------------------------------------

/// The circuit breaker state for a provider.
///
/// Implements the standard circuit breaker pattern:
/// - Closed = healthy, accepting traffic
/// - Open = unhealthy, all traffic rejected and rerouted
/// - HalfOpen = cooldown expired, testing with limited traffic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCandidate {
    pub provider_id: ProviderId,
    pub rail: RailPreference,
    pub estimated_fee: Decimal,
    pub estimated_latency_ms: u64,
    /// Composite score (higher = better). Computed from cost, speed, health,
    /// and corridor weights.
    pub score: f64,
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
}
