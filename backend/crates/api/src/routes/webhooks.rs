use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::auth::{AuthenticatedAgent, AuthenticatedOperator};
use crate::extractors::json::ValidatedJson;
use crate::state::AppState;
use crate::webhook_worker::{enqueue_webhook, WebhookEvent};

#[derive(Debug, Deserialize)]
pub struct RegisterWebhookRequest {
    pub url: String,
    pub events: Option<Vec<String>>,
    pub secret: String,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub id: WebhookEndpointId,
    pub url: String,
    pub events: Vec<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct WebhookDeliveryResponse {
    pub id: String,
    pub webhook_endpoint_id: String,
    pub event_type: String,
    pub status: String,
    pub http_status: Option<i16>,
    pub attempt: i16,
    pub max_attempts: i16,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_attempted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `POST /v1/webhooks` — register a webhook endpoint for payment events.
///
/// The endpoint is scoped to the authenticated agent — only events for this
/// agent will be delivered. Operators can see all endpoints via `GET /v1/webhooks`.
pub async fn register(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    ValidatedJson(body): ValidatedJson<RegisterWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), ApiError> {
    if body.url.is_empty() {
        return Err(ApiError::ValidationError(
            "webhook URL cannot be empty".into(),
        ));
    }
    if body.url.len() > 2048 {
        return Err(ApiError::ValidationError(
            "webhook URL exceeds maximum length of 2048 characters".into(),
        ));
    }
    if !body.url.starts_with("https://") && !body.url.starts_with("http://") {
        return Err(ApiError::ValidationError(
            "webhook URL must start with https:// or http://".into(),
        ));
    }
    if body.url.starts_with("http://") {
        tracing::warn!(
            url = %body.url,
            "webhook URL uses plaintext HTTP — HTTPS is required in production; \
             event payloads (payment IDs, amounts, agent IDs) will be transmitted unencrypted"
        );
    }
    if body.secret.is_empty() {
        return Err(ApiError::ValidationError(
            "webhook secret cannot be empty".into(),
        ));
    }
    if body.secret.len() < 16 {
        return Err(ApiError::ValidationError(
            "webhook secret must be at least 16 characters".into(),
        ));
    }

    let id = WebhookEndpointId::new();
    // Store the raw secret — it is the HMAC signing key used during delivery.
    // Both Cream and the receiver need the same key for Stripe-compatible
    // HMAC-SHA256 verification. This mirrors Stripe's own webhook secret
    // storage model.
    let secret_hash = body.secret.clone();
    let events = body.events.unwrap_or_else(|| vec!["*".to_string()]);
    let events_json = serde_json::to_value(&events)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize events: {e}")))?;

    sqlx::query(
        "INSERT INTO webhook_endpoints (id, url, secret_hash, events, status, agent_id, created_at)
         VALUES ($1, $2, $3, $4, 'active', $5, now())",
    )
    .bind(id.as_uuid())
    .bind(&body.url)
    .bind(&secret_hash)
    .bind(&events_json)
    .bind(agent.agent.id.as_uuid())
    .execute(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(WebhookResponse {
            id,
            url: body.url,
            events,
            status: "active".to_string(),
        }),
    ))
}

/// `GET /v1/webhooks` — list all registered webhook endpoints (operator-only).
pub async fn list_webhooks(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<WebhookResponse>>, ApiError> {
    let rows: Vec<(Uuid, String, serde_json::Value, String)> = sqlx::query_as(
        "SELECT id, url, events, status
         FROM webhook_endpoints
         ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let webhooks: Vec<WebhookResponse> = rows
        .into_iter()
        .map(|(id, url, events, status)| {
            let events_vec = events
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(|| vec!["*".to_string()]);
            WebhookResponse {
                id: WebhookEndpointId::from_uuid(id),
                url,
                events: events_vec,
                status,
            }
        })
        .collect();

    Ok(Json(webhooks))
}

/// `DELETE /v1/webhooks/{id}` — deactivate a webhook endpoint (operator-only).
pub async fn delete_webhook(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE webhook_endpoints SET status = 'inactive', updated_at = now()
         WHERE id = $1 AND status = 'active'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "webhook endpoint {id} not found or already inactive"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/webhooks/{id}/deliveries` — delivery log for a webhook endpoint (operator-only).
pub async fn list_deliveries(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<WebhookDeliveryResponse>>, ApiError> {
    // Verify endpoint exists.
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM webhook_endpoints WHERE id = $1)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if !exists.0 {
        return Err(ApiError::NotFound(format!(
            "webhook endpoint {id} not found"
        )));
    }

    let rows: Vec<DeliveryRow> = sqlx::query_as(
        "SELECT id, webhook_endpoint_id, event_type, status, http_status,
                attempt, max_attempts, created_at, delivered_at,
                last_attempted_at, next_retry_at
         FROM webhook_delivery_log
         WHERE webhook_endpoint_id = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let deliveries: Vec<WebhookDeliveryResponse> = rows
        .into_iter()
        .map(|r| WebhookDeliveryResponse {
            id: r.id.to_string(),
            webhook_endpoint_id: r.webhook_endpoint_id.to_string(),
            event_type: r.event_type,
            status: r.status,
            http_status: r.http_status,
            attempt: r.attempt,
            max_attempts: r.max_attempts,
            created_at: r.created_at,
            delivered_at: r.delivered_at,
            last_attempted_at: r.last_attempted_at,
            next_retry_at: r.next_retry_at,
        })
        .collect();

    Ok(Json(deliveries))
}

/// `POST /v1/webhooks/{id}/test` — send a test event to a webhook endpoint (operator-only).
pub async fn test_webhook(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // Verify endpoint exists and is active.
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM webhook_endpoints WHERE id = $1 AND status = 'active')",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if !exists.0 {
        return Err(ApiError::NotFound(format!(
            "webhook endpoint {id} not found or inactive"
        )));
    }

    // Enqueue a test event.
    enqueue_webhook(
        &state.redis,
        WebhookEvent {
            event_type: "test.ping".to_string(),
            payload: serde_json::json!({
                "message": "This is a test event from Cream",
                "webhook_endpoint_id": id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
            agent_id: None, // Broadcast — matches any endpoint
        },
    )
    .await;

    Ok(StatusCode::ACCEPTED)
}

// ---------------------------------------------------------------------------
// Internal query types
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct DeliveryRow {
    id: Uuid,
    webhook_endpoint_id: Uuid,
    event_type: String,
    status: String,
    http_status: Option<i16>,
    attempt: i16,
    max_attempts: i16,
    created_at: chrono::DateTime<chrono::Utc>,
    delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    last_attempted_at: Option<chrono::DateTime<chrono::Utc>>,
    next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}
