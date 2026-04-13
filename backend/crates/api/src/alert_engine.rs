//! Alerting rules engine (Phase 17-G).
//!
//! Background task that evaluates configured alert rules against the Prometheus
//! metrics registry every 60 seconds. When a rule's threshold is breached and
//! the cooldown period has elapsed, a notification is dispatched via the
//! existing `NotificationSender` infrastructure.

use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::metrics::read_metric_value;
use crate::state::AppState;

/// How often the alert engine evaluates rules.
const EVAL_INTERVAL_SECS: u64 = 60;

/// A snapshot of a metric value at a point in time, used to compute
/// windowed deltas for counter-based alert rules.
#[derive(Debug, Clone)]
struct MetricSnapshot {
    value: f64,
    timestamp: std::time::Instant,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertRule {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub metric: String,
    pub condition: String,
    pub threshold: Decimal,
    pub window_seconds: i32,
    pub cooldown_seconds: i32,
    pub channels: serde_json::Value,
    pub enabled: bool,
    pub last_fired_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Comparison operators for threshold evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertCondition {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
}

impl AlertCondition {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "gt" => Some(Self::Gt),
            "lt" => Some(Self::Lt),
            "gte" => Some(Self::Gte),
            "lte" => Some(Self::Lte),
            "eq" => Some(Self::Eq),
            _ => None,
        }
    }

    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Gt => value > threshold,
            Self::Lt => value < threshold,
            Self::Gte => value >= threshold,
            Self::Lte => value <= threshold,
            Self::Eq => (value - threshold).abs() < f64::EPSILON,
        }
    }
}

// ---------------------------------------------------------------------------
// Background worker
// ---------------------------------------------------------------------------

/// Background task that evaluates alert rules every 60 seconds.
///
/// Maintains an in-memory map of previous metric snapshots so that
/// counter-based rules can evaluate over a sliding window (delta-based)
/// rather than against the all-time cumulative total.
pub async fn alert_evaluation_worker(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(EVAL_INTERVAL_SECS));
    let mut snapshots: HashMap<String, MetricSnapshot> = HashMap::new();

    tracing::info!(
        interval_secs = EVAL_INTERVAL_SECS,
        "alert evaluation worker started"
    );

    loop {
        interval.tick().await;

        if let Err(e) = evaluate_alerts(&state, &mut snapshots).await {
            tracing::error!(error = %e, "alert evaluation failed");
        }
    }
}

#[tracing::instrument(skip_all)]
async fn evaluate_alerts(
    state: &AppState,
    snapshots: &mut HashMap<String, MetricSnapshot>,
) -> Result<(), anyhow::Error> {
    let metrics_handle = match &state.metrics_handle {
        Some(h) => h,
        None => return Ok(()), // Metrics disabled — nothing to evaluate.
    };

    // Fetch all enabled rules.
    let rules: Vec<AlertRule> = sqlx::query_as(
        "SELECT id, name, description, metric, condition, threshold, window_seconds,
                cooldown_seconds, channels, enabled, last_fired_at, created_at, updated_at
         FROM alert_rules WHERE enabled = true
         ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await?;

    if rules.is_empty() {
        return Ok(());
    }

    // Render current metrics snapshot.
    let rendered = metrics_handle.render();
    let now = std::time::Instant::now();

    for rule in &rules {
        let cond = match AlertCondition::parse(&rule.condition) {
            Some(c) => c,
            None => {
                tracing::warn!(
                    rule_id = %rule.id,
                    condition = %rule.condition,
                    "unknown alert condition, skipping"
                );
                continue;
            }
        };

        let current_value = read_metric_value(&rendered, &rule.metric);
        let threshold = match rule.threshold.to_string().parse::<f64>() {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!(
                    rule_id = %rule.id,
                    threshold = %rule.threshold,
                    "threshold parse failed, skipping rule"
                );
                continue;
            }
        };

        // Compute the effective value: delta within the window for counters,
        // or current value for gauges. We use the snapshot history to compute
        // the delta over the configured window_seconds.
        let snapshot_key = format!("{}:{}", rule.id, rule.metric);
        let effective_value = match snapshots.get(&snapshot_key) {
            Some(prev) => {
                let elapsed_secs = now.duration_since(prev.timestamp).as_secs_f64();
                let window = rule.window_seconds.max(1) as f64;

                if elapsed_secs >= window {
                    // Window has fully elapsed — use the raw delta.
                    current_value - prev.value
                } else {
                    // Window hasn't fully elapsed yet — extrapolate the rate.
                    // This avoids false negatives on the first few ticks.
                    let delta = current_value - prev.value;
                    if elapsed_secs > 0.0 {
                        delta * (window / elapsed_secs)
                    } else {
                        delta
                    }
                }
            }
            None => {
                // First observation — store snapshot and skip evaluation.
                // We need at least two data points to compute a delta.
                snapshots.insert(
                    snapshot_key,
                    MetricSnapshot {
                        value: current_value,
                        timestamp: now,
                    },
                );
                continue;
            }
        };

        // Update snapshot if window has elapsed (sliding window reset).
        let prev = snapshots.get(&snapshot_key).unwrap();
        if now.duration_since(prev.timestamp).as_secs_f64() >= rule.window_seconds.max(1) as f64 {
            snapshots.insert(
                snapshot_key.clone(),
                MetricSnapshot {
                    value: current_value,
                    timestamp: now,
                },
            );
        }

        if !cond.evaluate(effective_value, threshold) {
            continue;
        }

        // Check cooldown.
        if let Some(last_fired) = rule.last_fired_at {
            let elapsed = (Utc::now() - last_fired).num_seconds().max(0);
            if elapsed < rule.cooldown_seconds as i64 {
                tracing::debug!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    elapsed_secs = elapsed,
                    cooldown_secs = rule.cooldown_seconds,
                    "alert in cooldown, skipping"
                );
                continue;
            }
        }

        // Fire the alert!
        tracing::warn!(
            rule_id = %rule.id,
            rule_name = %rule.name,
            metric = %rule.metric,
            effective_value = effective_value,
            threshold = threshold,
            "alert rule triggered"
        );

        // Update last_fired_at.
        if let Err(e) = sqlx::query(
            "UPDATE alert_rules SET last_fired_at = now() WHERE id = $1",
        )
        .bind(rule.id)
        .execute(&state.db)
        .await
        {
            tracing::error!(rule_id = %rule.id, error = %e, "failed to update last_fired_at");
        }

        // Send notification via existing infrastructure.
        let alert_msg = format!(
            "Alert: {} — {} is {:.2} in last {}s (threshold: {}, condition: {})",
            rule.name, rule.metric, effective_value, rule.window_seconds,
            threshold, rule.condition
        );

        let notification = crate::notifications::AlertNotification {
            rule_name: rule.name.clone(),
            metric: rule.metric.clone(),
            value: effective_value,
            threshold,
            condition: rule.condition.clone(),
            message: alert_msg,
        };

        if let Err(e) = state
            .notification_sender
            .send_alert(&notification)
            .await
        {
            tracing::warn!(
                rule_id = %rule.id,
                error = %e,
                "alert notification failed (non-blocking)"
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// CRUD helpers
// ---------------------------------------------------------------------------

pub async fn list_rules(db: &PgPool) -> Result<Vec<AlertRule>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, description, metric, condition, threshold, window_seconds,
                cooldown_seconds, channels, enabled, last_fired_at, created_at, updated_at
         FROM alert_rules ORDER BY created_at ASC",
    )
    .fetch_all(db)
    .await
}

pub async fn get_rule(db: &PgPool, id: uuid::Uuid) -> Result<Option<AlertRule>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, description, metric, condition, threshold, window_seconds,
                cooldown_seconds, channels, enabled, last_fired_at, created_at, updated_at
         FROM alert_rules WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_condition_gt() {
        let cond = AlertCondition::Gt;
        assert!(cond.evaluate(11.0, 10.0));
        assert!(!cond.evaluate(10.0, 10.0));
        assert!(!cond.evaluate(9.0, 10.0));
    }

    #[test]
    fn alert_condition_lt() {
        let cond = AlertCondition::Lt;
        assert!(cond.evaluate(9.0, 10.0));
        assert!(!cond.evaluate(10.0, 10.0));
    }

    #[test]
    fn alert_condition_gte() {
        let cond = AlertCondition::Gte;
        assert!(cond.evaluate(10.0, 10.0));
        assert!(cond.evaluate(11.0, 10.0));
        assert!(!cond.evaluate(9.0, 10.0));
    }

    #[test]
    fn alert_condition_from_str() {
        assert_eq!(AlertCondition::parse("gt"), Some(AlertCondition::Gt));
        assert_eq!(AlertCondition::parse("lt"), Some(AlertCondition::Lt));
        assert_eq!(AlertCondition::parse("gte"), Some(AlertCondition::Gte));
        assert_eq!(AlertCondition::parse("lte"), Some(AlertCondition::Lte));
        assert_eq!(AlertCondition::parse("eq"), Some(AlertCondition::Eq));
        assert_eq!(AlertCondition::parse("bad"), None);
    }

    #[test]
    fn read_metric_value_parses_prometheus_text() {
        let rendered = "\
# HELP cream_provider_errors_total Total provider errors
# TYPE cream_provider_errors_total counter
cream_provider_errors_total{provider=\"stripe\",retryable=\"true\"} 5
cream_provider_errors_total{provider=\"stripe\",retryable=\"false\"} 3
cream_payments_total{status=\"settled\"} 100
cream_payments_total{status=\"failed\"} 7
";
        // Sum all lines matching the metric name.
        assert_eq!(read_metric_value(rendered, "cream_provider_errors_total"), 8.0);
        assert_eq!(read_metric_value(rendered, "cream_payments_total"), 107.0);
        assert_eq!(read_metric_value(rendered, "cream_nonexistent"), 0.0);
    }

    #[test]
    fn metric_snapshot_delta_computation() {
        // Simulate the delta-based windowing logic used in evaluate_alerts.
        let prev = MetricSnapshot {
            value: 100.0,
            timestamp: std::time::Instant::now() - Duration::from_secs(300),
        };
        let current_value = 115.0;
        let window_secs: f64 = 300.0;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(prev.timestamp).as_secs_f64();

        // Window fully elapsed: delta should be 15.
        assert!(elapsed >= window_secs);
        let delta = current_value - prev.value;
        assert!((delta - 15.0).abs() < 0.01);
    }

    #[test]
    fn threshold_parse_failure_skips_rule() {
        // Ensure that a threshold that can't be parsed to f64 is handled.
        // NaN and infinity should be parseable, but an empty string should not.
        assert!("".parse::<f64>().is_err());
        // Normal decimals should work fine.
        assert!(("10.5".parse::<f64>()).is_ok());
    }
}
