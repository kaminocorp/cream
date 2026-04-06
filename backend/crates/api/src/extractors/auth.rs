use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{DateTime, Utc};
use cream_models::prelude::*;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::ApiError;
use crate::state::AppState;

/// Resolved agent identity, injected by the extractor into handlers that
/// require authentication.
#[derive(Debug, Clone)]
pub struct AuthenticatedAgent {
    pub agent: Agent,
    pub profile: AgentProfile,
}

/// Axum extractor that authenticates an agent via `Authorization: Bearer <api_key>`.
///
/// 1. Extracts the bearer token from the header.
/// 2. SHA-256 hashes it and looks up the agent by `api_key_hash`.
/// 3. Verifies the agent is `active`.
/// 4. Loads the associated `AgentProfile`.
///
/// Handlers that include `AuthenticatedAgent` as a parameter automatically
/// require authentication; handlers that omit it are public.
impl FromRequestParts<AppState> for AuthenticatedAgent {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        if token.is_empty() {
            return Err(ApiError::Unauthorized);
        }

        let key_hash = hex::encode(Sha256::digest(token.as_bytes()));

        let (agent, profile) = lookup_agent_by_key_hash(&state.db, &key_hash).await?;
        Ok(AuthenticatedAgent { agent, profile })
    }
}

// ---------------------------------------------------------------------------
// DB helpers (intentionally not on the PaymentRepository trait — auth is a
// cross-cutting concern, not a payment domain operation)
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: uuid::Uuid,
    profile_id: uuid::Uuid,
    name: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AgentProfileRow {
    id: uuid::Uuid,
    name: String,
    version: i32,
    max_per_transaction: Option<rust_decimal::Decimal>,
    max_daily_spend: Option<rust_decimal::Decimal>,
    max_weekly_spend: Option<rust_decimal::Decimal>,
    max_monthly_spend: Option<rust_decimal::Decimal>,
    allowed_categories: serde_json::Value,
    allowed_rails: serde_json::Value,
    geographic_restrictions: serde_json::Value,
    escalation_threshold: Option<rust_decimal::Decimal>,
    timezone: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Look up an agent and its profile by agent ID. Used by the approve flow and any
/// path that needs full agent context without an API key (e.g. after human review).
pub(crate) async fn lookup_agent_by_id(
    pool: &PgPool,
    agent_id: &AgentId,
) -> Result<Option<(Agent, AgentProfile)>, ApiError> {
    let agent_row: Option<AgentRow> = sqlx::query_as(
        "SELECT id, profile_id, name, status, created_at, updated_at
         FROM agents WHERE id = $1",
    )
    .bind(agent_id.as_uuid())
    .fetch_optional(pool)
    .await?;

    let agent_row = match agent_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let profile_row: AgentProfileRow = sqlx::query_as(
        "SELECT id, name, version, max_per_transaction, max_daily_spend,
                max_weekly_spend, max_monthly_spend, allowed_categories,
                allowed_rails, geographic_restrictions, escalation_threshold,
                timezone, created_at, updated_at
         FROM agent_profiles WHERE id = $1",
    )
    .bind(agent_row.profile_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!(
            "agent profile {} not found for agent {}",
            agent_row.profile_id,
            agent_row.id
        ))
    })?;

    let agent_json = serde_json::json!({
        "id": format!("agt_{}", agent_row.id),
        "profile_id": format!("prof_{}", agent_row.profile_id),
        "name": agent_row.name,
        "status": agent_row.status,
        "created_at": agent_row.created_at,
        "updated_at": agent_row.updated_at,
    });
    let agent: Agent = serde_json::from_value(agent_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize agent: {e}")))?;

    let profile_json = serde_json::json!({
        "id": format!("prof_{}", profile_row.id),
        "name": profile_row.name,
        "version": profile_row.version,
        "max_per_transaction": profile_row.max_per_transaction,
        "max_daily_spend": profile_row.max_daily_spend,
        "max_weekly_spend": profile_row.max_weekly_spend,
        "max_monthly_spend": profile_row.max_monthly_spend,
        "allowed_categories": profile_row.allowed_categories,
        "allowed_rails": profile_row.allowed_rails,
        "geographic_restrictions": profile_row.geographic_restrictions,
        "escalation_threshold": profile_row.escalation_threshold,
        "timezone": profile_row.timezone,
        "created_at": profile_row.created_at,
        "updated_at": profile_row.updated_at,
    });
    let profile: AgentProfile = serde_json::from_value(profile_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize profile: {e}")))?;

    Ok(Some((agent, profile)))
}

/// Look up only the AgentProfileId for a given agent. Lightweight alternative when
/// only the profile ID is needed (reject flow, escalation timeout audit entries).
pub(crate) async fn lookup_profile_id_for_agent(
    pool: &PgPool,
    agent_id: &AgentId,
) -> Result<Option<AgentProfileId>, ApiError> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT profile_id FROM agents WHERE id = $1",
    )
    .bind(agent_id.as_uuid())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id,)| AgentProfileId::from_uuid(id)))
}

async fn lookup_agent_by_key_hash(
    pool: &PgPool,
    key_hash: &str,
) -> Result<(Agent, AgentProfile), ApiError> {
    let agent_row: AgentRow = sqlx::query_as(
        "SELECT id, profile_id, name, status, created_at, updated_at
         FROM agents
         WHERE api_key_hash = $1 AND status = 'active'",
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    let profile_row: AgentProfileRow = sqlx::query_as(
        "SELECT id, name, version, max_per_transaction, max_daily_spend,
                max_weekly_spend, max_monthly_spend, allowed_categories,
                allowed_rails, geographic_restrictions, escalation_threshold,
                timezone, created_at, updated_at
         FROM agent_profiles
         WHERE id = $1",
    )
    .bind(agent_row.profile_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Internal(anyhow::anyhow!(
        "agent profile {} not found for agent {}",
        agent_row.profile_id,
        agent_row.id
    )))?;

    let agent_json = serde_json::json!({
        "id": format!("agt_{}", agent_row.id),
        "profile_id": format!("prof_{}", agent_row.profile_id),
        "name": agent_row.name,
        "status": agent_row.status,
        "created_at": agent_row.created_at,
        "updated_at": agent_row.updated_at,
    });
    let agent: Agent = serde_json::from_value(agent_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize agent: {e}")))?;

    let profile_json = serde_json::json!({
        "id": format!("prof_{}", profile_row.id),
        "name": profile_row.name,
        "version": profile_row.version,
        "max_per_transaction": profile_row.max_per_transaction,
        "max_daily_spend": profile_row.max_daily_spend,
        "max_weekly_spend": profile_row.max_weekly_spend,
        "max_monthly_spend": profile_row.max_monthly_spend,
        "allowed_categories": profile_row.allowed_categories,
        "allowed_rails": profile_row.allowed_rails,
        "geographic_restrictions": profile_row.geographic_restrictions,
        "escalation_threshold": profile_row.escalation_threshold,
        "timezone": profile_row.timezone,
        "created_at": profile_row.created_at,
        "updated_at": profile_row.updated_at,
    });
    let profile: AgentProfile = serde_json::from_value(profile_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize profile: {e}")))?;

    Ok((agent, profile))
}
