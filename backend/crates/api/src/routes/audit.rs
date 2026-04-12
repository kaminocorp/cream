use axum::extract::{Query, State};
use axum::Json;
use cream_audit::AuditQuery;
use cream_models::prelude::*;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    pub from: Option<String>,
    pub to: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    /// Free-text case-insensitive search against `justification.summary`.
    /// Trimmed, truncated to the reader's max length, ILIKE-escaped.
    pub q: Option<String>,
    /// Operator-only: scope results to a specific agent. Ignored (and a
    /// validation error surfaced) when the caller is an agent — agents are
    /// always hard-scoped to themselves.
    pub agent_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// `GET /v1/audit` — query the audit log with filters.
///
/// Visibility rules:
///
/// - **Agent caller** — hard-scoped to their own `agent_id`. Passing an
///   `agent_id` query param different from the caller's is a 400 (rather
///   than silently ignoring it, which would leak whether filtering "worked").
/// - **Operator caller** — sees every agent's entries by default. Passing
///   `agent_id` scopes to that one agent; omitting it returns everything
///   subject to `limit`/`offset`.
pub async fn query(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let mut query = AuditQuery::new();

    // Resolve agent_id scoping based on principal.
    match &principal {
        AuthenticatedPrincipal::Agent(agent) => {
            // Agents are hard-scoped to themselves. If they passed an
            // `agent_id` param, verify it matches — otherwise 400.
            if let Some(ref requested) = params.agent_id {
                let requested_id = requested.parse::<AgentId>().map_err(|e| {
                    ApiError::ValidationError(format!("invalid agent_id: {e}"))
                })?;
                if requested_id != agent.agent.id {
                    return Err(ApiError::ValidationError(
                        "agents may only query their own audit entries".to_string(),
                    ));
                }
            }
            query = query.agent_id(agent.agent.id);
        }
        AuthenticatedPrincipal::Operator(_) => {
            // Operators see all agents by default; optional narrow-scope.
            if let Some(ref requested) = params.agent_id {
                let requested_id = requested.parse::<AgentId>().map_err(|e| {
                    ApiError::ValidationError(format!("invalid agent_id: {e}"))
                })?;
                query = query.agent_id(requested_id);
            }
        }
    }

    if let Some(ref from) = params.from {
        let ts = from
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|e| ApiError::ValidationError(format!("invalid 'from' timestamp: {e}")))?;
        query = query.from(ts);
    }

    if let Some(ref to) = params.to {
        let ts = to
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|e| ApiError::ValidationError(format!("invalid 'to' timestamp: {e}")))?;
        query = query.to(ts);
    }

    if let Some(ref status) = params.status {
        let s: PaymentStatus = serde_json::from_value(serde_json::json!(status))
            .map_err(|e| ApiError::ValidationError(format!("invalid status: {e}")))?;
        query = query.status(s);
    }

    if let Some(ref category) = params.category {
        let c: PaymentCategory = serde_json::from_value(serde_json::json!(category))
            .map_err(|e| ApiError::ValidationError(format!("invalid category: {e}")))?;
        query = query.category(c);
    }

    if let Some(ref min) = params.min_amount {
        let amt: rust_decimal::Decimal = min
            .parse()
            .map_err(|e| ApiError::ValidationError(format!("invalid min_amount: {e}")))?;
        query = query.min_amount(amt);
    }

    if let Some(ref max) = params.max_amount {
        let amt: rust_decimal::Decimal = max
            .parse()
            .map_err(|e| ApiError::ValidationError(format!("invalid max_amount: {e}")))?;
        query = query.max_amount(amt);
    }

    if let Some(q) = params.q {
        query = query.q(q);
    }

    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    if let Some(offset) = params.offset {
        query = query.offset(offset);
    }

    let entries = state
        .audit_reader
        .query(query)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(entries))
}
