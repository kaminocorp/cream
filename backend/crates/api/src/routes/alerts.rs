//! Alert rule management endpoints (Phase 17-G).
//!
//! All endpoints are operator-only. Alert rules are evaluated by the background
//! `alert_evaluation_worker` against the Prometheus metrics registry.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::alert_engine;
use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedOperator;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request/response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateAlertRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub metric: String,
    pub condition: String,
    pub threshold: Decimal,
    #[serde(default = "default_window")]
    pub window_seconds: i32,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: i32,
    #[serde(default = "default_channels")]
    pub channels: serde_json::Value,
}

fn default_window() -> i32 { 300 }
fn default_cooldown() -> i32 { 3600 }
fn default_channels() -> serde_json::Value { serde_json::json!(["dashboard"]) }

#[derive(Debug, Deserialize)]
pub struct UpdateAlertRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metric: Option<String>,
    pub condition: Option<String>,
    pub threshold: Option<Decimal>,
    pub window_seconds: Option<i32>,
    pub cooldown_seconds: Option<i32>,
    pub channels: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AlertRuleResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub metric: String,
    pub condition: String,
    pub threshold: Decimal,
    pub window_seconds: i32,
    pub cooldown_seconds: i32,
    pub channels: serde_json::Value,
    pub enabled: bool,
    pub last_fired_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<alert_engine::AlertRule> for AlertRuleResponse {
    fn from(r: alert_engine::AlertRule) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            description: r.description,
            metric: r.metric,
            condition: r.condition,
            threshold: r.threshold,
            window_seconds: r.window_seconds,
            cooldown_seconds: r.cooldown_seconds,
            channels: r.channels,
            enabled: r.enabled,
            last_fired_at: r.last_fired_at.map(|t| t.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

const VALID_CONDITIONS: &[&str] = &["gt", "lt", "gte", "lte", "eq"];

/// Reject the request if the operator is not an admin. Read-only endpoints
/// (list, history) are available to all authenticated operators; mutating
/// endpoints (create, update, delete) require the `admin` role.
fn require_admin(op: &AuthenticatedOperator) -> Result<(), ApiError> {
    if op.role != "admin" {
        return Err(ApiError::Forbidden(
            "admin role required to modify alert rules".into(),
        ));
    }
    Ok(())
}

/// Notification channels that have implementations in the notifications module.
const VALID_CHANNELS: &[&str] = &["dashboard", "slack", "email"];

/// Metrics that the alert engine can evaluate. Must stay in sync with the
/// constants in `crate::metrics`. Alerting on a nonexistent metric silently
/// evaluates to 0.0 and never fires — this validation prevents that.
const KNOWN_METRICS: &[&str] = &[
    crate::metrics::PAYMENTS_TOTAL,
    crate::metrics::PAYMENT_DURATION_SECONDS,
    crate::metrics::POLICY_EVALUATION_DURATION_SECONDS,
    crate::metrics::POLICY_DECISION_TOTAL,
    crate::metrics::PROVIDER_REQUEST_DURATION_SECONDS,
    crate::metrics::PROVIDER_ERRORS_TOTAL,
    crate::metrics::WEBHOOK_DELIVERIES_TOTAL,
    crate::metrics::WEBHOOK_DELIVERY_DURATION_SECONDS,
    crate::metrics::WEBHOOK_RETRIES_TOTAL,
    crate::metrics::RATE_LIMIT_HITS_TOTAL,
    crate::metrics::AUTH_ATTEMPTS_TOTAL,
    crate::metrics::ESCALATION_PENDING_COUNT,
    crate::metrics::CREDENTIAL_AGE_WARNING,
    crate::metrics::CIRCUIT_BREAKER_STATE,
    crate::metrics::ERROR_RECOVERY_FAILURES_TOTAL,
    crate::metrics::REDIS_CONNECTION_ERRORS_TOTAL,
];

/// Validate that `channels` JSON is an array of known channel strings.
fn validate_channels(channels: &serde_json::Value) -> Result<(), ApiError> {
    if let Some(arr) = channels.as_array() {
        for item in arr {
            match item.as_str() {
                Some(ch) => {
                    let lower = ch.to_lowercase();
                    if !VALID_CHANNELS.contains(&lower.as_str()) {
                        return Err(ApiError::ValidationError(format!(
                            "unknown channel '{ch}'; valid channels: {}",
                            VALID_CHANNELS.join(", ")
                        )));
                    }
                }
                None => {
                    return Err(ApiError::ValidationError(
                        "channels array must contain only strings".into(),
                    ));
                }
            }
        }
    } else if !channels.is_null() {
        return Err(ApiError::ValidationError(
            "channels must be a JSON array of strings".into(),
        ));
    }
    Ok(())
}

/// `GET /v1/alerts` — list all alert rules.
pub async fn list_alerts(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<AlertRuleResponse>>, ApiError> {
    let rules = alert_engine::list_rules(&state.db).await?;
    Ok(Json(rules.into_iter().map(AlertRuleResponse::from).collect()))
}

/// `POST /v1/alerts` — create a new alert rule (admin only).
pub async fn create_alert(
    State(state): State<AppState>,
    op: AuthenticatedOperator,
    Json(body): Json<CreateAlertRequest>,
) -> Result<(StatusCode, Json<AlertRuleResponse>), ApiError> {
    require_admin(&op)?;

    // Validate.
    if body.name.trim().is_empty() {
        return Err(ApiError::ValidationError("name must not be empty".into()));
    }
    if body.metric.trim().is_empty() {
        return Err(ApiError::ValidationError("metric must not be empty".into()));
    }
    if !KNOWN_METRICS.contains(&body.metric.trim()) {
        return Err(ApiError::ValidationError(format!(
            "unknown metric '{}'; use GET /metrics or check docs for available metric names",
            body.metric.trim()
        )));
    }
    if !VALID_CONDITIONS.contains(&body.condition.as_str()) {
        return Err(ApiError::ValidationError(format!(
            "condition must be one of: {}",
            VALID_CONDITIONS.join(", ")
        )));
    }
    if body.window_seconds <= 0 || body.window_seconds > 86_400 {
        return Err(ApiError::ValidationError(
            "window_seconds must be between 1 and 86400 (24 hours)".into(),
        ));
    }
    if body.cooldown_seconds <= 0 || body.cooldown_seconds > 604_800 {
        return Err(ApiError::ValidationError(
            "cooldown_seconds must be between 1 and 604800 (7 days)".into(),
        ));
    }
    if body.threshold < Decimal::ZERO {
        return Err(ApiError::ValidationError(
            "threshold must be non-negative".into(),
        ));
    }
    validate_channels(&body.channels)?;

    let id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO alert_rules (id, name, description, metric, condition, threshold,
                                   window_seconds, cooldown_seconds, channels)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(body.name.trim())
    .bind(&body.description)
    .bind(body.metric.trim())
    .bind(&body.condition)
    .bind(body.threshold)
    .bind(body.window_seconds)
    .bind(body.cooldown_seconds)
    .bind(&body.channels)
    .execute(&state.db)
    .await?;

    let rule = alert_engine::get_rule(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("just-created alert rule not found")))?;

    Ok((StatusCode::CREATED, Json(AlertRuleResponse::from(rule))))
}

/// `PATCH /v1/alerts/{id}` — update an alert rule (admin only).
pub async fn update_alert(
    State(state): State<AppState>,
    op: AuthenticatedOperator,
    Path(id): Path<String>,
    Json(body): Json<UpdateAlertRequest>,
) -> Result<Json<AlertRuleResponse>, ApiError> {
    require_admin(&op)?;

    let rule_id = id
        .parse::<uuid::Uuid>()
        .map_err(|e| ApiError::ValidationError(format!("invalid alert ID: {e}")))?;

    // Validate all provided fields — same rules as create_alert.
    if let Some(ref name) = body.name {
        if name.trim().is_empty() {
            return Err(ApiError::ValidationError("name must not be empty".into()));
        }
    }
    if let Some(ref metric) = body.metric {
        if metric.trim().is_empty() {
            return Err(ApiError::ValidationError("metric must not be empty".into()));
        }
        if !KNOWN_METRICS.contains(&metric.trim()) {
            return Err(ApiError::ValidationError(format!(
                "unknown metric '{}'; use GET /metrics or check docs for available metric names",
                metric.trim()
            )));
        }
    }
    if let Some(ref cond) = body.condition {
        if !VALID_CONDITIONS.contains(&cond.as_str()) {
            return Err(ApiError::ValidationError(format!(
                "condition must be one of: {}",
                VALID_CONDITIONS.join(", ")
            )));
        }
    }
    if let Some(ws) = body.window_seconds {
        if ws <= 0 || ws > 86_400 {
            return Err(ApiError::ValidationError(
                "window_seconds must be between 1 and 86400 (24 hours)".into(),
            ));
        }
    }
    if let Some(cs) = body.cooldown_seconds {
        if cs <= 0 || cs > 604_800 {
            return Err(ApiError::ValidationError(
                "cooldown_seconds must be between 1 and 604800 (7 days)".into(),
            ));
        }
    }
    if let Some(t) = body.threshold {
        if t < Decimal::ZERO {
            return Err(ApiError::ValidationError(
                "threshold must be non-negative".into(),
            ));
        }
    }
    if let Some(ref channels) = body.channels {
        validate_channels(channels)?;
    }

    let rows = sqlx::query(
        "UPDATE alert_rules SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            metric = COALESCE($3, metric),
            condition = COALESCE($4, condition),
            threshold = COALESCE($5, threshold),
            window_seconds = COALESCE($6, window_seconds),
            cooldown_seconds = COALESCE($7, cooldown_seconds),
            channels = COALESCE($8, channels),
            enabled = COALESCE($9, enabled),
            updated_at = now()
         WHERE id = $10",
    )
    .bind(body.name.as_deref().map(str::trim))
    .bind(body.description.as_deref())
    .bind(body.metric.as_deref().map(str::trim))
    .bind(body.condition.as_deref())
    .bind(body.threshold)
    .bind(body.window_seconds)
    .bind(body.cooldown_seconds)
    .bind(&body.channels)
    .bind(body.enabled)
    .bind(rule_id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(ApiError::NotFound(format!("alert rule {id}")));
    }

    let rule = alert_engine::get_rule(&state.db, rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("alert rule {id}")))?;

    Ok(Json(AlertRuleResponse::from(rule)))
}

/// `DELETE /v1/alerts/{id}` — disable an alert rule (admin only, soft delete).
pub async fn delete_alert(
    State(state): State<AppState>,
    op: AuthenticatedOperator,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&op)?;

    let rule_id = id
        .parse::<uuid::Uuid>()
        .map_err(|e| ApiError::ValidationError(format!("invalid alert ID: {e}")))?;

    let rows = sqlx::query("UPDATE alert_rules SET enabled = false, updated_at = now() WHERE id = $1")
        .bind(rule_id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(ApiError::NotFound(format!("alert rule {id}")));
    }

    Ok(Json(serde_json::json!({ "status": "disabled" })))
}

/// `GET /v1/alerts/history` — list recently fired alerts.
pub async fn alert_history(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<AlertRuleResponse>>, ApiError> {
    let rules: Vec<alert_engine::AlertRule> = sqlx::query_as(
        "SELECT id, name, description, metric, condition, threshold, window_seconds,
                cooldown_seconds, channels, enabled, last_fired_at, created_at, updated_at
         FROM alert_rules WHERE last_fired_at IS NOT NULL
         ORDER BY last_fired_at DESC
         LIMIT 100",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rules.into_iter().map(AlertRuleResponse::from).collect()))
}
