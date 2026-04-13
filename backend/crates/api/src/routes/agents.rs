//! Agent management and policy endpoints.
//!
//! Phase 15.1 introduces operator-level agent lifecycle management. Two
//! classes of handler live here:
//!
//! 1. **Policy read** (`get_policy`) — callable by either an agent acting on
//!    itself, or an operator acting on any agent. Accepts
//!    [`AuthenticatedPrincipal`] and branches on the variant.
//!    **Policy write** (`update_policy`) — operator-only. Agents must not be
//!    able to self-elevate their own spending limits.
//!
//! 2. **Agent lifecycle** (`list_agents`, `create_agent`, `update_agent`,
//!    `rotate_agent_key`) — callable only by operators. These accept
//!    [`AuthenticatedOperator`] and return 401 for any other caller.
//!
//! API key generation: 256 bits of entropy sourced from two UUID v4 values
//! concatenated (~244 effective random bits — above every practical
//! threshold and zero new crate deps). The plaintext key is formatted as
//! `cream_<64 hex chars>` and returned in the response body **exactly once**;
//! the database persists only its SHA-256 hash in `agents.api_key_hash`.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::extractors::auth::{
    lookup_agent_by_id, AuthenticatedOperator, AuthenticatedPrincipal,
};
use crate::extractors::json::ValidatedJson;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response for `GET /v1/agents/{id}/policy`.
///
/// Carries the full agent (so the dashboard detail page has name + status
/// without a second round-trip), the profile (spending limits + allowed
/// categories/rails/geo), and the list of policy rules. The frontend
/// `AgentPolicyResponse` type mirrors this shape exactly.
#[derive(Serialize)]
pub struct AgentPolicyResponse {
    pub agent: Agent,
    pub profile: AgentProfile,
    pub rules: Vec<PolicyRule>,
}

/// Lightweight summary returned by `GET /v1/agents`. Excludes `api_key_hash`
/// (never exposed) and the full profile (too heavy for a list view).
#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub id: AgentId,
    pub profile_id: AgentProfileId,
    pub profile_name: String,
    pub name: String,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for agent create. The plaintext `api_key` is returned ONCE —
/// the backend stores only its SHA-256 hash, so there is no way to retrieve
/// it again. Callers must surface a "copy once" UX to operators.
#[derive(Debug, Serialize)]
pub struct CreateAgentResponse {
    pub agent: AgentSummary,
    /// Plaintext API key. Prefix `cream_` for identifiability in logs; 64 hex
    /// chars after the prefix (256 bits of entropy). Persisted only as
    /// SHA-256 hash.
    pub api_key: String,
}

/// Response for key rotation. Same contract as create: plaintext returned
/// once, old key invalidated.
#[derive(Debug, Serialize)]
pub struct RotateKeyResponse {
    pub agent_id: AgentId,
    pub api_key: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for `POST /v1/agents`.
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub profile_id: AgentProfileId,
}

/// Request body for `PATCH /v1/agents/{id}`. All fields optional; only those
/// present are updated. Policy (spending limits, allowed categories, etc.)
/// lives on the profile and is changed via the separate policy endpoints.
#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub status: Option<AgentStatus>,
    pub profile_id: Option<AgentProfileId>,
}

/// Request body for `PUT /v1/agents/{id}/policy`.
///
/// Spending limit fields use `Option<Option<Decimal>>` with a custom
/// deserializer so the API can distinguish three states:
///   - **Key absent** from JSON → `None` → don't change the column
///   - **Key present as `null`** → `Some(None)` → clear (set column to NULL)
///   - **Key present with value** → `Some(Some(v))` → set column to `v`
///
/// The standard `Option<T>` + `COALESCE` pattern cannot distinguish absent
/// from null, making it impossible to clear a spending limit once set.
#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    #[serde(default, deserialize_with = "deserialize_clearable")]
    pub max_per_transaction: Option<Option<rust_decimal::Decimal>>,
    #[serde(default, deserialize_with = "deserialize_clearable")]
    pub max_daily_spend: Option<Option<rust_decimal::Decimal>>,
    #[serde(default, deserialize_with = "deserialize_clearable")]
    pub max_weekly_spend: Option<Option<rust_decimal::Decimal>>,
    #[serde(default, deserialize_with = "deserialize_clearable")]
    pub max_monthly_spend: Option<Option<rust_decimal::Decimal>>,
    pub allowed_categories: Option<Vec<PaymentCategory>>,
    pub allowed_rails: Option<Vec<RailPreference>>,
    pub geographic_restrictions: Option<Vec<CountryCode>>,
    #[serde(default, deserialize_with = "deserialize_clearable")]
    pub escalation_threshold: Option<Option<rust_decimal::Decimal>>,
}

/// Deserialize a field that supports three states: absent, explicit null, or
/// a value. When the key is present (even as `null`), serde calls this
/// function; the `#[serde(default)]` attribute ensures absent keys produce
/// `None` without calling this function at all.
fn deserialize_clearable<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    // Key is present — deserialize the inner value (which may be null).
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

// ---------------------------------------------------------------------------
// Policy handlers — accept either principal
// ---------------------------------------------------------------------------

/// `GET /v1/agents/{id}/policy` — get an agent's policy profile and rules.
///
/// Operators may read any agent's policy. Agents may only read their own.
/// Returns `{ agent, profile, rules }` — the agent struct is included so
/// the dashboard can render name and status without a second round-trip.
pub async fn get_policy(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<String>,
) -> Result<Json<AgentPolicyResponse>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    // Authorisation: agents can only view their own policy; operators any.
    principal.authorize_target_agent(&agent_id)?;

    // If the caller is the agent itself we already have both the agent and
    // the profile loaded from auth — avoid the extra DB round-trip.
    let (agent, profile) = match &principal {
        AuthenticatedPrincipal::Agent(a) if a.agent.id == agent_id => {
            (a.agent.clone(), a.profile.clone())
        }
        _ => lookup_agent_by_id(&state.db, &agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id}")))?,
    };

    let rules = state.payment_repo.load_rules(&profile.id).await?;

    Ok(Json(AgentPolicyResponse {
        agent,
        profile,
        rules,
    }))
}

/// `PUT /v1/agents/{id}/policy` — update an agent's policy profile.
///
/// **Operator-only.** Agents must not be able to modify their own spending
/// limits, allowed categories, or escalation thresholds — that would
/// undermine the entire control-plane premise. Updates are applied directly
/// to the profile row without version bumping (the full version history is
/// a Phase 16-A concern).
pub async fn update_policy(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdatePolicyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    let (_, profile) = lookup_agent_by_id(&state.db, &agent_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id}")))?;
    let profile_id = profile.id;

    // Validate spending limits are strictly positive when provided.
    // `Some(Some(v))` = setting a value → must be positive.
    // `Some(None)` = clearing → allowed (no validation needed).
    // `None` = absent → no change.
    for (name, value) in [
        ("max_per_transaction", &body.max_per_transaction),
        ("max_daily_spend", &body.max_daily_spend),
        ("max_weekly_spend", &body.max_weekly_spend),
        ("max_monthly_spend", &body.max_monthly_spend),
        ("escalation_threshold", &body.escalation_threshold),
    ] {
        if let Some(Some(v)) = value {
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

    // Spending limits use CASE WHEN instead of COALESCE so that explicit
    // null clears the column. The boolean flag ($1, $3, ...) is true when
    // the key was present in JSON; the value ($2, $4, ...) is NULL for
    // clear or the Decimal for set. COALESCE is still correct for the
    // array fields (categories, rails, geo) since clearing those means
    // sending `[]`, not `null`.
    sqlx::query(
        "UPDATE agent_profiles SET
            max_per_transaction   = CASE WHEN $1::boolean THEN $2  ELSE max_per_transaction   END,
            max_daily_spend       = CASE WHEN $3::boolean THEN $4  ELSE max_daily_spend       END,
            max_weekly_spend      = CASE WHEN $5::boolean THEN $6  ELSE max_weekly_spend      END,
            max_monthly_spend     = CASE WHEN $7::boolean THEN $8  ELSE max_monthly_spend     END,
            allowed_categories    = COALESCE($9,  allowed_categories),
            allowed_rails         = COALESCE($10, allowed_rails),
            geographic_restrictions = COALESCE($11, geographic_restrictions),
            escalation_threshold  = CASE WHEN $12::boolean THEN $13 ELSE escalation_threshold END,
            updated_at = now()
         WHERE id = $14",
    )
    // max_per_transaction
    .bind(body.max_per_transaction.is_some())
    .bind(body.max_per_transaction.flatten())
    // max_daily_spend
    .bind(body.max_daily_spend.is_some())
    .bind(body.max_daily_spend.flatten())
    // max_weekly_spend
    .bind(body.max_weekly_spend.is_some())
    .bind(body.max_weekly_spend.flatten())
    // max_monthly_spend
    .bind(body.max_monthly_spend.is_some())
    .bind(body.max_monthly_spend.flatten())
    // array fields (COALESCE)
    .bind(&categories_json)
    .bind(&rails_json)
    .bind(&geo_json)
    // escalation_threshold
    .bind(body.escalation_threshold.is_some())
    .bind(body.escalation_threshold.flatten())
    // WHERE
    .bind(profile_id.as_uuid())
    .execute(&state.db)
    .await?;

    log_operator_event(
        &state.db,
        "policy_updated",
        &agent_id,
        serde_json::json!({ "profile_id": profile_id.to_string() }),
    )
    .await;

    Ok(Json(serde_json::json!({ "status": "updated" })))
}

// ---------------------------------------------------------------------------
// Agent lifecycle handlers — operator-only
// ---------------------------------------------------------------------------

/// Record an operator event in the `operator_events` append-only log.
///
/// Best-effort: if the INSERT fails (e.g. DB issue), the mutation itself
/// has already committed, so we log the failure rather than unwinding the
/// business operation. Audit writes should never block successful mutations.
async fn log_operator_event(
    db: &sqlx::PgPool,
    event_type: &str,
    target_agent_id: &AgentId,
    details: serde_json::Value,
) {
    let result = sqlx::query(
        "INSERT INTO operator_events (event_type, target_agent_id, details)
         VALUES ($1, $2, $3)",
    )
    .bind(event_type)
    .bind(target_agent_id.as_uuid())
    .bind(&details)
    .execute(db)
    .await;

    if let Err(e) = result {
        tracing::error!(
            event_type = event_type,
            agent_id = %target_agent_id,
            error = %e,
            "failed to write operator event — mutation succeeded but audit write failed"
        );
    }
}

/// `GET /v1/agents` — list all agents. Operator-only.
///
/// Returns lightweight [`AgentSummary`] rows joined with the profile name.
/// Hard-capped at 500 rows to bound response size; pagination is a
/// post-15.1 concern once operators start running into the ceiling.
pub async fn list_agents(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<AgentSummary>>, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        profile_id: uuid::Uuid,
        profile_name: String,
        name: String,
        status: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT a.id, a.profile_id, p.name AS profile_name, a.name, a.status,
                a.created_at, a.updated_at
         FROM agents a
         JOIN agent_profiles p ON p.id = a.profile_id
         ORDER BY a.created_at DESC
         LIMIT 500",
    )
    .fetch_all(&state.db)
    .await?;

    let summaries = rows
        .into_iter()
        .map(|r| {
            let status: AgentStatus =
                serde_json::from_value(serde_json::json!(r.status)).map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("deserialize status {}: {e}", r.status))
                })?;
            Ok(AgentSummary {
                id: AgentId::from_uuid(r.id),
                profile_id: AgentProfileId::from_uuid(r.profile_id),
                profile_name: r.profile_name,
                name: r.name,
                status,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(summaries))
}

/// `POST /v1/agents` — create a new agent. Operator-only.
///
/// Validates the target profile exists, then inserts a new row with a
/// freshly generated API key. Returns the plaintext key in the response —
/// the caller MUST display it once and then drop it; the backend stores
/// only the SHA-256 hash.
pub async fn create_agent(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    ValidatedJson(body): ValidatedJson<CreateAgentRequest>,
) -> Result<Json<CreateAgentResponse>, ApiError> {
    let trimmed_name = body.name.trim();
    if trimmed_name.is_empty() {
        return Err(ApiError::ValidationError(
            "agent name must not be empty".to_string(),
        ));
    }
    if trimmed_name.len() > 255 {
        return Err(ApiError::ValidationError(format!(
            "agent name exceeds maximum length of 255 characters (got {})",
            trimmed_name.len()
        )));
    }

    // Verify target profile exists before inserting — a FK violation would
    // produce a less descriptive error.
    let profile_exists: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM agent_profiles WHERE id = $1")
            .bind(body.profile_id.as_uuid())
            .fetch_optional(&state.db)
            .await?;
    if profile_exists.is_none() {
        return Err(ApiError::NotFound(format!(
            "agent_profile {}",
            body.profile_id
        )));
    }

    let agent_id = uuid::Uuid::now_v7();
    let (plaintext, hash) = generate_api_key();

    sqlx::query(
        "INSERT INTO agents (id, profile_id, name, api_key_hash, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', now(), now())",
    )
    .bind(agent_id)
    .bind(body.profile_id.as_uuid())
    .bind(trimmed_name)
    .bind(&hash)
    .execute(&state.db)
    .await?;

    // Fetch the just-inserted row to populate the response with DB timestamps.
    let created_id = AgentId::from_uuid(agent_id);
    let summary = load_agent_summary(&state, created_id).await?;

    log_operator_event(
        &state.db,
        "agent_created",
        &created_id,
        serde_json::json!({
            "name": trimmed_name,
            "profile_id": body.profile_id.to_string(),
        }),
    )
    .await;

    Ok(Json(CreateAgentResponse {
        agent: summary,
        api_key: plaintext,
    }))
}

/// `PATCH /v1/agents/{id}` — update an agent's name, status, or profile
/// assignment. Operator-only.
pub async fn update_agent(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdateAgentRequest>,
) -> Result<Json<AgentSummary>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    if body.name.is_none() && body.status.is_none() && body.profile_id.is_none() {
        return Err(ApiError::ValidationError(
            "PATCH body must include at least one of: name, status, profile_id".to_string(),
        ));
    }

    if let Some(ref name) = body.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ApiError::ValidationError(
                "agent name must not be empty".to_string(),
            ));
        }
        if trimmed.len() > 255 {
            return Err(ApiError::ValidationError(format!(
                "agent name exceeds maximum length of 255 characters (got {})",
                trimmed.len()
            )));
        }
    }

    if let Some(profile_id) = body.profile_id {
        let exists: Option<(uuid::Uuid,)> =
            sqlx::query_as("SELECT id FROM agent_profiles WHERE id = $1")
                .bind(profile_id.as_uuid())
                .fetch_optional(&state.db)
                .await?;
        if exists.is_none() {
            return Err(ApiError::NotFound(format!("agent_profile {profile_id}")));
        }
    }

    // Serialize status to its serde string for the CHECK-constrained column.
    let status_str = body
        .status
        .map(|s| {
            serde_json::to_value(s)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(|| {
                    ApiError::Internal(anyhow::anyhow!(
                        "failed to serialize status {s:?} to string"
                    ))
                })
        })
        .transpose()?;

    // COALESCE pattern: only fields present in the request body are updated.
    let rows_affected = sqlx::query(
        "UPDATE agents SET
            name = COALESCE($1, name),
            status = COALESCE($2, status),
            profile_id = COALESCE($3, profile_id),
            updated_at = now()
         WHERE id = $4",
    )
    .bind(body.name.as_deref().map(str::trim))
    .bind(status_str.as_deref())
    .bind(body.profile_id.map(|p| *p.as_uuid()))
    .bind(agent_id.as_uuid())
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ApiError::NotFound(format!("agent {agent_id}")));
    }

    let mut details = serde_json::Map::new();
    if let Some(ref name) = body.name {
        details.insert("name".into(), serde_json::json!(name.trim()));
    }
    if let Some(status) = body.status {
        details.insert("status".into(), serde_json::json!(status));
    }
    if let Some(profile_id) = body.profile_id {
        details.insert("profile_id".into(), serde_json::json!(profile_id.to_string()));
    }
    log_operator_event(
        &state.db,
        "agent_updated",
        &agent_id,
        serde_json::Value::Object(details),
    )
    .await;

    let summary = load_agent_summary(&state, agent_id).await?;
    Ok(Json(summary))
}

/// `POST /v1/agents/{id}/rotate-key` — generate a new API key for an agent,
/// invalidating the old one. Operator-only.
///
/// Returns the plaintext key in the response body exactly once. The old
/// key stops working as soon as this SQL UPDATE commits.
pub async fn rotate_agent_key(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
) -> Result<Json<RotateKeyResponse>, ApiError> {
    let agent_id = id
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    let (plaintext, hash) = generate_api_key();

    let rows_affected = sqlx::query(
        "UPDATE agents SET api_key_hash = $1, key_rotated_at = now(), updated_at = now() WHERE id = $2",
    )
    .bind(&hash)
    .bind(agent_id.as_uuid())
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(ApiError::NotFound(format!("agent {agent_id}")));
    }

    log_operator_event(
        &state.db,
        "agent_key_rotated",
        &agent_id,
        serde_json::json!({}),
    )
    .await;

    Ok(Json(RotateKeyResponse {
        agent_id,
        api_key: plaintext,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a new API key: `cream_<64 hex chars>`. Returns `(plaintext, hash)`
/// where `hash` is the SHA-256 hex digest that gets persisted.
///
/// Entropy source: two UUID v4 values concatenated → 32 bytes → 64 hex chars.
/// Each UUID v4 contributes 122 random bits, so the total effective entropy
/// is ~244 bits — well above every practical threshold, with zero new crate
/// dependencies.
fn generate_api_key() -> (String, String) {
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(a.as_bytes());
    bytes[16..].copy_from_slice(b.as_bytes());
    let plaintext = format!("cream_{}", hex::encode(bytes));
    let hash = hex::encode(Sha256::digest(plaintext.as_bytes()));
    (plaintext, hash)
}

/// Fetch a single agent's summary. Used by create/update responses to return
/// the authoritative post-write state (including DB-generated timestamps).
async fn load_agent_summary(
    state: &AppState,
    agent_id: AgentId,
) -> Result<AgentSummary, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        profile_id: uuid::Uuid,
        profile_name: String,
        name: String,
        status: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let row: Row = sqlx::query_as(
        "SELECT a.id, a.profile_id, p.name AS profile_name, a.name, a.status,
                a.created_at, a.updated_at
         FROM agents a
         JOIN agent_profiles p ON p.id = a.profile_id
         WHERE a.id = $1",
    )
    .bind(agent_id.as_uuid())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id}")))?;

    let status: AgentStatus = serde_json::from_value(serde_json::json!(row.status))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize status: {e}")))?;

    Ok(AgentSummary {
        id: AgentId::from_uuid(row.id),
        profile_id: AgentProfileId::from_uuid(row.profile_id),
        profile_name: row.profile_name,
        name: row.name,
        status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_api_key_is_correct_shape() {
        let (plaintext, hash) = generate_api_key();
        assert!(plaintext.starts_with("cream_"));
        // "cream_" (6) + 64 hex = 70
        assert_eq!(plaintext.len(), 70);
        // Everything after the prefix is lowercase hex.
        let suffix = &plaintext[6..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "expected lowercase hex suffix, got {suffix}"
        );
        // SHA-256 digest is 64 hex chars.
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generated_api_key_has_sha256_hash_matching_plaintext() {
        let (plaintext, hash) = generate_api_key();
        let recomputed = hex::encode(Sha256::digest(plaintext.as_bytes()));
        assert_eq!(hash, recomputed);
    }

    #[test]
    fn two_generated_keys_differ() {
        let (a, _) = generate_api_key();
        let (b, _) = generate_api_key();
        assert_ne!(a, b, "keys must not collide");
    }
}
