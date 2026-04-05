use axum::extract::{Query, State};
use axum::Json;
use cream_audit::AuditQuery;
use cream_models::prelude::*;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    pub from: Option<String>,
    pub to: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// `GET /v1/audit` — query the audit log with filters.
///
/// The agent can only see their own audit entries (scoped by authenticated agent_id).
pub async fn query(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let mut query = AuditQuery::new().agent_id(agent.agent.id);

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
