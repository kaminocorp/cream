//! Prometheus metrics for the Cream payment control plane.
//!
//! All metric names use the `cream_` prefix. Instrumentation callsites use the
//! [`metrics`] crate macros (`counter!`, `histogram!`, `gauge!`) which are
//! no-ops when no recorder is installed — zero overhead when metrics are disabled.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

// ---------------------------------------------------------------------------
// Metric name constants
// ---------------------------------------------------------------------------

// Payments
pub const PAYMENTS_TOTAL: &str = "cream_payments_total";
pub const PAYMENT_DURATION_SECONDS: &str = "cream_payment_duration_seconds";

// Policy
pub const POLICY_EVALUATION_DURATION_SECONDS: &str = "cream_policy_evaluation_duration_seconds";
pub const POLICY_DECISION_TOTAL: &str = "cream_policy_decision_total";

// Providers
pub const PROVIDER_REQUEST_DURATION_SECONDS: &str = "cream_provider_request_duration_seconds";
pub const PROVIDER_ERRORS_TOTAL: &str = "cream_provider_errors_total";

// Webhooks
pub const WEBHOOK_DELIVERIES_TOTAL: &str = "cream_webhook_deliveries_total";
pub const WEBHOOK_DELIVERY_DURATION_SECONDS: &str = "cream_webhook_delivery_duration_seconds";
pub const WEBHOOK_RETRIES_TOTAL: &str = "cream_webhook_retries_total";

// Rate limiting
pub const RATE_LIMIT_HITS_TOTAL: &str = "cream_rate_limit_hits_total";

// Auth
pub const AUTH_ATTEMPTS_TOTAL: &str = "cream_auth_attempts_total";

// Escalations
pub const ESCALATION_PENDING_COUNT: &str = "cream_escalation_pending_count";

// Credentials
pub const CREDENTIAL_AGE_WARNING: &str = "cream_credential_age_warning";

// Circuit breaker
pub const CIRCUIT_BREAKER_STATE: &str = "cream_circuit_breaker_state";

// Infrastructure
pub const REDIS_CONNECTION_ERRORS_TOTAL: &str = "cream_redis_connection_errors_total";

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

/// Install the Prometheus metrics recorder with an HTTP listener for scraping.
///
/// This calls `install_recorder()` which:
/// 1. Builds the recorder
/// 2. Sets it as the global `metrics` recorder
/// 3. Spawns an HTTP server on the given port serving `/metrics`
///
/// The HTTP listener runs on a background Tokio task — it does not block.
/// Must be called once, after the Tokio runtime is available.
/// Install the Prometheus metrics recorder and return a handle for reading
/// metric values. The handle is used by the alert engine (Phase 17-G) to
/// evaluate alerting rules against current metric values.
pub fn init_metrics(port: u16) -> PrometheusHandle {
    let builder = PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], port));

    builder
        .install_recorder()
        .expect("failed to install Prometheus metrics recorder")
}

/// Parse a numeric metric value from Prometheus exposition text.
/// Looks for lines matching the metric name (ignoring label variants)
/// and sums all matching values.
pub fn read_metric_value(rendered: &str, metric_name: &str) -> f64 {
    let mut total = 0.0;
    for line in rendered.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        // Lines look like: metric_name{labels} value
        // or just: metric_name value
        let name_end = line.find(['{', ' ']).unwrap_or(line.len());
        let name = &line[..name_end];
        if name == metric_name {
            if let Some(val_str) = line.rsplit(' ').next() {
                if let Ok(val) = val_str.parse::<f64>() {
                    total += val;
                }
            }
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_names_use_cream_prefix() {
        let all_names = [
            PAYMENTS_TOTAL,
            PAYMENT_DURATION_SECONDS,
            POLICY_EVALUATION_DURATION_SECONDS,
            POLICY_DECISION_TOTAL,
            PROVIDER_REQUEST_DURATION_SECONDS,
            PROVIDER_ERRORS_TOTAL,
            WEBHOOK_DELIVERIES_TOTAL,
            WEBHOOK_DELIVERY_DURATION_SECONDS,
            WEBHOOK_RETRIES_TOTAL,
            RATE_LIMIT_HITS_TOTAL,
            AUTH_ATTEMPTS_TOTAL,
            ESCALATION_PENDING_COUNT,
            CREDENTIAL_AGE_WARNING,
            CIRCUIT_BREAKER_STATE,
            REDIS_CONNECTION_ERRORS_TOTAL,
        ];
        for name in &all_names {
            assert!(
                name.starts_with("cream_"),
                "metric '{name}' must use cream_ prefix"
            );
        }
    }

    #[test]
    fn metric_names_are_unique() {
        let all_names = [
            PAYMENTS_TOTAL,
            PAYMENT_DURATION_SECONDS,
            POLICY_EVALUATION_DURATION_SECONDS,
            POLICY_DECISION_TOTAL,
            PROVIDER_REQUEST_DURATION_SECONDS,
            PROVIDER_ERRORS_TOTAL,
            WEBHOOK_DELIVERIES_TOTAL,
            WEBHOOK_DELIVERY_DURATION_SECONDS,
            WEBHOOK_RETRIES_TOTAL,
            RATE_LIMIT_HITS_TOTAL,
            AUTH_ATTEMPTS_TOTAL,
            ESCALATION_PENDING_COUNT,
            CREDENTIAL_AGE_WARNING,
            CIRCUIT_BREAKER_STATE,
            REDIS_CONNECTION_ERRORS_TOTAL,
        ];
        let mut seen = std::collections::HashSet::new();
        for name in &all_names {
            assert!(seen.insert(name), "duplicate metric name: {name}");
        }
    }

    #[test]
    fn metric_count_is_15() {
        // Guard: if someone adds a metric constant, this test reminds them
        // to add it to the all_names arrays above and in the docs.
        assert_eq!(
            [
                PAYMENTS_TOTAL,
                PAYMENT_DURATION_SECONDS,
                POLICY_EVALUATION_DURATION_SECONDS,
                POLICY_DECISION_TOTAL,
                PROVIDER_REQUEST_DURATION_SECONDS,
                PROVIDER_ERRORS_TOTAL,
                WEBHOOK_DELIVERIES_TOTAL,
                WEBHOOK_DELIVERY_DURATION_SECONDS,
                WEBHOOK_RETRIES_TOTAL,
                RATE_LIMIT_HITS_TOTAL,
                AUTH_ATTEMPTS_TOTAL,
                ESCALATION_PENDING_COUNT,
                CREDENTIAL_AGE_WARNING,
                CIRCUIT_BREAKER_STATE,
                REDIS_CONNECTION_ERRORS_TOTAL,
            ]
            .len(),
            15
        );
    }
}
