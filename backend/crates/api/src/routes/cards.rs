use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use cream_models::prelude::*;
use cream_providers::CardConfig;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::extractors::json::ValidatedJson;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateCardRequest {
    pub card_type: CardType,
    pub controls: CardControls,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Which provider to use for card issuance. If omitted, uses the first
    /// registered provider that supports card issuance.
    pub provider_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCardRequest {
    pub controls: CardControls,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /v1/cards` — issue a scoped virtual card to the authenticated agent.
pub async fn create(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    ValidatedJson(body): ValidatedJson<CreateCardRequest>,
) -> Result<(StatusCode, Json<VirtualCard>), ApiError> {
    let provider_id = body
        .provider_id
        .map(|id| ProviderId::try_new(id).map_err(|e| ApiError::ValidationError(e.to_string())))
        .transpose()?;

    // Find the provider to use for card issuance.
    let provider = if let Some(ref id) = provider_id {
        state
            .provider_registry
            .get(id)
            .ok_or_else(|| ApiError::NotFound(format!("provider {id}")))?
    } else {
        // Use the first registered provider.
        let all = state.provider_registry.all();
        all.into_iter()
            .next()
            .ok_or(ApiError::AllProvidersUnavailable)?
    };

    let config = CardConfig {
        agent_id: agent.agent.id,
        card_type: body.card_type,
        controls: body.controls,
        expires_at: body.expires_at,
    };

    let card = provider
        .issue_virtual_card(&config)
        .await
        .map_err(ApiError::ProviderFailure)?;

    // Persist to database.
    let controls_json = serde_json::to_value(&card.controls)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize controls: {e}")))?;

    sqlx::query(
        "INSERT INTO virtual_cards
            (id, agent_id, provider_id, provider_card_id, card_type, controls,
             status, created_at, expires_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now(), $8, now())",
    )
    .bind(card.id.as_uuid())
    .bind(card.agent_id.as_uuid())
    .bind(card.provider.as_str())
    .bind(&card.provider_card_id)
    .bind(
        serde_json::to_value(card.card_type)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize card_type: {e}")))?
            .as_str()
            .ok_or_else(|| {
                ApiError::Internal(anyhow::anyhow!("card_type did not serialize to string"))
            })?
            .to_string(),
    )
    .bind(&controls_json)
    .bind(
        serde_json::to_value(card.status)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize card status: {e}")))?
            .as_str()
            .ok_or_else(|| {
                ApiError::Internal(anyhow::anyhow!("card status did not serialize to string"))
            })?
            .to_string(),
    )
    .bind(card.expires_at)
    .execute(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(card)))
}

/// `PATCH /v1/cards/{id}` — update spending controls on an existing card.
pub async fn update(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdateCardRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let card_id = id
        .parse::<VirtualCardId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid card ID: {e}")))?;

    // Load card and verify ownership.
    let card_row: Option<(String, String)> = sqlx::query_as(
        "SELECT provider_id, provider_card_id FROM virtual_cards
         WHERE id = $1 AND agent_id = $2 AND status = 'active'",
    )
    .bind(card_id.as_uuid())
    .bind(agent.agent.id.as_uuid())
    .fetch_optional(&state.db)
    .await?;

    let (provider_id_str, provider_card_id) =
        card_row.ok_or_else(|| ApiError::NotFound(format!("card {card_id}")))?;

    let provider_id = ProviderId::try_new(provider_id_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid provider_id: {e}")))?;

    let provider = state
        .provider_registry
        .get(&provider_id)
        .ok_or_else(|| ApiError::NotFound(format!("provider {provider_id}")))?;

    provider
        .update_card_controls(&provider_card_id, &body.controls)
        .await
        .map_err(ApiError::ProviderFailure)?;

    // Update controls in DB.
    let controls_json = serde_json::to_value(&body.controls)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize controls: {e}")))?;

    sqlx::query("UPDATE virtual_cards SET controls = $1, updated_at = now() WHERE id = $2")
        .bind(&controls_json)
        .bind(card_id.as_uuid())
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}

/// `DELETE /v1/cards/{id}` — cancel/revoke a virtual card immediately.
pub async fn cancel(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let card_id = id
        .parse::<VirtualCardId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid card ID: {e}")))?;

    let card_row: Option<(String, String)> = sqlx::query_as(
        "SELECT provider_id, provider_card_id FROM virtual_cards
         WHERE id = $1 AND agent_id = $2 AND status = 'active'",
    )
    .bind(card_id.as_uuid())
    .bind(agent.agent.id.as_uuid())
    .fetch_optional(&state.db)
    .await?;

    let (provider_id_str, provider_card_id) =
        card_row.ok_or_else(|| ApiError::NotFound(format!("card {card_id}")))?;

    let provider_id = ProviderId::try_new(provider_id_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid provider_id: {e}")))?;

    let provider = state
        .provider_registry
        .get(&provider_id)
        .ok_or_else(|| ApiError::NotFound(format!("provider {provider_id}")))?;

    provider
        .cancel_card(&provider_card_id)
        .await
        .map_err(ApiError::ProviderFailure)?;

    sqlx::query("UPDATE virtual_cards SET status = 'cancelled', updated_at = now() WHERE id = $1")
        .bind(card_id.as_uuid())
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}
