use axum::extract::{Path, State};
use axum::Json;
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::extractors::json::ValidatedJson;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AgentPolicyResponse {
    pub profile: AgentProfile,
    pub rules: Vec<PolicyRule>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub max_per_transaction: Option<rust_decimal::Decimal>,
    pub max_daily_spend: Option<rust_decimal::Decimal>,
    pub max_weekly_spend: Option<rust_decimal::Decimal>,
    pub max_monthly_spend: Option<rust_decimal::Decimal>,
    pub allowed_categories: Option<Vec<PaymentCategory>>,
    pub allowed_rails: Option<Vec<RailPreference>>,
    pub geographic_restrictions: Option<Vec<CountryCode>>,
    pub escalation_threshold: Option<rust_decimal::Decimal>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /v1/agents/{id}/policy` — get the agent's policy profile and rules.
///
/// Agents can only view their own policy.
pub async fn get_policy(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(id): Path<String>,
) -> Result<Json<AgentPolicyResponse>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    // Agents can only view their own policy.
    if agent_id != agent.agent.id {
        return Err(ApiError::NotFound(format!("agent {agent_id}")));
    }

    let rules = state.payment_repo.load_rules(&agent.profile.id).await?;

    Ok(Json(AgentPolicyResponse {
        profile: agent.profile,
        rules,
    }))
}

/// `PUT /v1/agents/{id}/policy` — update the agent's policy profile.
///
/// For the scaffold, this updates profile fields directly without version bumping.
/// Agents can only update their own policy.
pub async fn update_policy(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdatePolicyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    if agent_id != agent.agent.id {
        return Err(ApiError::NotFound(format!("agent {agent_id}")));
    }

    // Validate spending limits are strictly positive when provided.
    // AgentProfile's custom Deserialize enforces > 0, but this handler
    // bypasses deserialization (struct literal → SQL). Without this check,
    // a zero value would pass the DB CHECK (>= 0), be persisted, and then
    // fail deserialization on the next auth attempt — permanently locking
    // the agent out.
    for (name, value) in [
        ("max_per_transaction", &body.max_per_transaction),
        ("max_daily_spend", &body.max_daily_spend),
        ("max_weekly_spend", &body.max_weekly_spend),
        ("max_monthly_spend", &body.max_monthly_spend),
        ("escalation_threshold", &body.escalation_threshold),
    ] {
        if let Some(v) = value {
            if *v <= rust_decimal::Decimal::ZERO {
                return Err(ApiError::ValidationError(format!(
                    "{name} must be positive, got {v}"
                )));
            }
        }
    }

    let categories_json = body
        .allowed_categories
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize categories: {e}")))?;

    let rails_json = body
        .allowed_rails
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize rails: {e}")))?;

    let geo_json = body
        .geographic_restrictions
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize geo: {e}")))?;

    // Use COALESCE to only update fields that are provided.
    sqlx::query(
        "UPDATE agent_profiles SET
            max_per_transaction = COALESCE($1, max_per_transaction),
            max_daily_spend = COALESCE($2, max_daily_spend),
            max_weekly_spend = COALESCE($3, max_weekly_spend),
            max_monthly_spend = COALESCE($4, max_monthly_spend),
            allowed_categories = COALESCE($5, allowed_categories),
            allowed_rails = COALESCE($6, allowed_rails),
            geographic_restrictions = COALESCE($7, geographic_restrictions),
            escalation_threshold = COALESCE($8, escalation_threshold),
            updated_at = now()
         WHERE id = $9",
    )
    .bind(body.max_per_transaction)
    .bind(body.max_daily_spend)
    .bind(body.max_weekly_spend)
    .bind(body.max_monthly_spend)
    .bind(&categories_json)
    .bind(&rails_json)
    .bind(&geo_json)
    .bind(body.escalation_threshold)
    .bind(agent.profile.id.as_uuid())
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}
