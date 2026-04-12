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

/// Resolved operator identity. In Phase 15.1 the operator is a single shared
/// secret without an internal identity — there is no operator table, no
/// multi-user tracking, no roles. Phase 16-A replaces this with a real
/// identity (`operator_id`, roles) and a proper sign-in flow; callers
/// pattern-matching on [`AuthenticatedPrincipal::Operator`] keep working
/// unchanged because the variant is an opaque struct.
///
/// Intentionally carries no fields: 16-A can add them without any handler
/// churn because all current handlers only care about the fact that the
/// caller is an operator, not who the operator is.
#[derive(Debug, Clone, Default)]
pub struct AuthenticatedOperator {
    // Phase 16-A: add operator_id, roles, email, etc.
}

/// Either an authenticated agent or an authenticated operator.
///
/// Handlers that should be callable by *both* principals accept this enum
/// and pattern-match on the variant to decide what data to expose (operators
/// see cross-agent data; agents are hard-scoped to themselves). Handlers
/// that should be callable by *only one* principal keep their original
/// extractor signature.
///
/// Note: `Agent` carries the full `AuthenticatedAgent` (profile + agent)
/// while `Operator` is currently empty. Clippy flags the size disparity
/// via `large_enum_variant`, but boxing would force every call site to
/// deref through a `Box` and add a heap allocation per authenticated
/// request. Since this type lives for one request only and is immediately
/// moved into the handler, the size difference is irrelevant.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum AuthenticatedPrincipal {
    Agent(AuthenticatedAgent),
    Operator(AuthenticatedOperator),
}

impl AuthenticatedPrincipal {
    /// If this principal is an agent, return a reference to it. Otherwise None.
    pub fn as_agent(&self) -> Option<&AuthenticatedAgent> {
        match self {
            Self::Agent(a) => Some(a),
            Self::Operator(_) => None,
        }
    }

    /// True if this principal is an operator (dashboard / admin caller).
    pub fn is_operator(&self) -> bool {
        matches!(self, Self::Operator(_))
    }

    /// Return the agent this principal is allowed to act on when a specific
    /// agent ID is named in the request path. Operators may target any agent;
    /// agents may only target themselves.
    ///
    /// Use this from handlers with a `{id}` path segment to avoid hand-rolling
    /// the check every time.
    pub fn authorize_target_agent(&self, target: &AgentId) -> Result<(), ApiError> {
        match self {
            Self::Operator(_) => Ok(()),
            Self::Agent(a) if &a.agent.id == target => Ok(()),
            Self::Agent(_) => Err(ApiError::NotFound(format!("agent {target}"))),
        }
    }
}

/// Constant-time comparison for the operator API key. Avoids timing-oracle
/// leakage of key bytes via response latency. Not strictly necessary at
/// current volumes, but cheap and correct.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Extract the bearer token from the Authorization header, or return Unauthorized.
fn bearer_token(parts: &Parts) -> Result<&str, ApiError> {
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

    Ok(token)
}

/// Check whether the token matches the configured operator key. Returns
/// `false` (not an error) when the operator key is unset — callers fall
/// through to agent lookup.
fn token_is_operator(state: &AppState, token: &str) -> bool {
    match state.config.operator_api_key.as_deref() {
        Some(expected) => constant_time_eq(expected.as_bytes(), token.as_bytes()),
        None => false,
    }
}

/// Axum extractor that authenticates an agent via `Authorization: Bearer <api_key>`.
///
/// 1. Extracts the bearer token from the header.
/// 2. SHA-256 hashes it and looks up the agent by `api_key_hash`.
/// 3. Verifies the agent is `active`.
/// 4. Loads the associated `AgentProfile`.
///
/// **Does not match the operator key** — this extractor is only for handlers
/// that must be agent-scoped (e.g., `POST /v1/payments`, anything called by
/// MCP). For handlers that accept either principal, use
/// [`AuthenticatedPrincipal`].
impl FromRequestParts<AppState> for AuthenticatedAgent {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;

        // Defence in depth: if someone configures the operator key and then
        // tries to call an agent-only endpoint with it, reject cleanly
        // rather than falling through to a DB lookup that would return
        // Unauthorized anyway.
        if token_is_operator(state, token) {
            return Err(ApiError::Unauthorized);
        }

        let key_hash = hex::encode(Sha256::digest(token.as_bytes()));

        let (agent, profile) = lookup_agent_by_key_hash(&state.db, &key_hash).await?;
        Ok(AuthenticatedAgent { agent, profile })
    }
}

/// Axum extractor that authenticates an operator via the shared
/// `OPERATOR_API_KEY`. Returns 401 if the header is missing, malformed,
/// or does not match the configured operator key. Also returns 401 when
/// the operator key is unset in config (safe default).
impl FromRequestParts<AppState> for AuthenticatedOperator {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;

        if token_is_operator(state, token) {
            Ok(AuthenticatedOperator::default())
        } else {
            Err(ApiError::Unauthorized)
        }
    }
}

/// Axum extractor that resolves the caller as either an operator (if the
/// token matches the shared operator key) or an agent (if the token matches
/// an `api_key_hash` in the `agents` table). Rejects with 401 otherwise.
///
/// Use this on handlers that should be callable by both principals — the
/// handler then branches on the variant to decide what data to expose.
impl FromRequestParts<AppState> for AuthenticatedPrincipal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;

        // Operator path first — constant-time check against the shared key.
        if token_is_operator(state, token) {
            return Ok(Self::Operator(AuthenticatedOperator::default()));
        }

        // Fall through to agent lookup.
        let key_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let (agent, profile) = lookup_agent_by_key_hash(&state.db, &key_hash).await?;
        Ok(Self::Agent(AuthenticatedAgent { agent, profile }))
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
