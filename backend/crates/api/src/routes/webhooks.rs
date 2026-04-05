use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::extractors::json::ValidatedJson;
use crate::state::AppState;

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

/// `POST /v1/webhooks` — register a webhook endpoint for payment events.
pub async fn register(
    State(state): State<AppState>,
    _agent: AuthenticatedAgent,
    ValidatedJson(body): ValidatedJson<RegisterWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), ApiError> {
    if body.url.is_empty() {
        return Err(ApiError::ValidationError(
            "webhook URL cannot be empty".into(),
        ));
    }
    if body.secret.is_empty() {
        return Err(ApiError::ValidationError(
            "webhook secret cannot be empty".into(),
        ));
    }

    let id = WebhookEndpointId::new();
    let secret_hash = hex::encode(Sha256::digest(body.secret.as_bytes()));
    let events = body.events.unwrap_or_else(|| vec!["*".to_string()]);
    let events_json = serde_json::to_value(&events)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize events: {e}")))?;

    sqlx::query(
        "INSERT INTO webhook_endpoints (id, url, secret_hash, events, status, created_at)
         VALUES ($1, $2, $3, $4, 'active', now())",
    )
    .bind(id.as_uuid())
    .bind(&body.url)
    .bind(&secret_hash)
    .bind(&events_json)
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
