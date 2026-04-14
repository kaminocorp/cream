use axum::extract::{Query, State};
use axum::http::header::{self, HeaderMap};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use cream_audit::AuditQuery;
use cream_models::prelude::*;
use serde::Deserialize;

use crate::audit_export::flatten_entry;
use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedPrincipal;
use crate::state::AppState;

/// Maximum rows returned for synchronous CSV/NDJSON exports.
/// Larger exports must use the async `POST /v1/audit/export` endpoint.
const SYNC_EXPORT_ROW_CAP: u32 = 10_000;

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

// CSV flattening uses the shared `flatten_entry()` from `audit_export.rs`
// to avoid logic duplication between sync and async export paths.

/// Determines the requested export format from the `Accept` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Csv,
    Ndjson,
}

fn parse_accept(headers: &HeaderMap) -> ExportFormat {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    // Check for exact or prefix match. Multiple types with quality factors
    // are not supported — the first matching type wins.
    if accept.contains("text/csv") {
        ExportFormat::Csv
    } else if accept.contains("application/x-ndjson") {
        ExportFormat::Ndjson
    } else {
        ExportFormat::Json
    }
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
///
/// Content negotiation via `Accept` header:
/// - `application/json` (default) — standard JSON array
/// - `text/csv` — flattened CSV, capped at 10,000 rows
/// - `application/x-ndjson` — one JSON object per line, capped at 10,000 rows
pub async fn query(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    headers: HeaderMap,
    Query(params): Query<AuditQueryParams>,
) -> Result<Response, ApiError> {
    let format = parse_accept(&headers);

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

    // For CSV/NDJSON, enforce the sync row cap. Use export_mode() to bypass
    // the standard 1K pagination clamp so we can fetch up to 10,001 rows for
    // overflow detection.
    match format {
        ExportFormat::Csv | ExportFormat::Ndjson => {
            // Request one more than the cap to detect overflow.
            let effective_limit = params.limit.unwrap_or(SYNC_EXPORT_ROW_CAP).min(SYNC_EXPORT_ROW_CAP + 1);
            query = query.limit(effective_limit).export_mode();
        }
        ExportFormat::Json => {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
        }
    }

    if let Some(offset) = params.offset {
        query = query.offset(offset);
    }

    let entries = state
        .audit_reader
        .query(query)
        .await
        .map_err(ApiError::from)?;

    match format {
        ExportFormat::Json => Ok(Json(entries).into_response()),
        ExportFormat::Csv => {
            if entries.len() as u32 > SYNC_EXPORT_ROW_CAP {
                return Ok((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(serde_json::json!({
                        "error_code": "EXPORT_TOO_LARGE",
                        "message": format!(
                            "synchronous CSV export limited to {} rows; use POST /v1/audit/export for larger sets",
                            SYNC_EXPORT_ROW_CAP
                        ),
                        "row_cap": SYNC_EXPORT_ROW_CAP,
                    })),
                )
                    .into_response());
            }
            let csv_body = entries_to_csv(&entries)?;
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"audit_export.csv\"",
                    ),
                ],
                csv_body,
            )
                .into_response())
        }
        ExportFormat::Ndjson => {
            if entries.len() as u32 > SYNC_EXPORT_ROW_CAP {
                return Ok((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(serde_json::json!({
                        "error_code": "EXPORT_TOO_LARGE",
                        "message": format!(
                            "synchronous NDJSON export limited to {} rows; use POST /v1/audit/export for larger sets",
                            SYNC_EXPORT_ROW_CAP
                        ),
                        "row_cap": SYNC_EXPORT_ROW_CAP,
                    })),
                )
                    .into_response());
            }
            let ndjson_body = entries_to_ndjson(&entries)?;
            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/x-ndjson")],
                ndjson_body,
            )
                .into_response())
        }
    }
}

/// Serialize audit entries to CSV.
fn entries_to_csv(entries: &[AuditEntry]) -> Result<String, ApiError> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    // Header row.
    writer
        .write_record([
            "entry_id",
            "timestamp",
            "agent_id",
            "payment_id",
            "amount",
            "currency",
            "status",
            "decision",
            "provider",
            "justification_summary",
        ])
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("CSV header write: {e}")))?;

    for entry in entries {
        let row = flatten_entry(entry);
        writer
            .write_record(&row)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("CSV row write: {e}")))?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("CSV flush: {e}")))?;

    String::from_utf8(bytes)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("CSV UTF-8: {e}")))
}

/// Serialize audit entries to NDJSON (one JSON object per line).
/// Returns an error if any entry fails to serialize, rather than silently
/// dropping it — data integrity is critical for audit exports.
fn entries_to_ndjson(entries: &[AuditEntry]) -> Result<String, ApiError> {
    let mut buf = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("NDJSON serialize: {e}")))?;
        buf.push_str(&line);
        buf.push('\n');
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accept_defaults_to_json() {
        let headers = HeaderMap::new();
        assert_eq!(parse_accept(&headers), ExportFormat::Json);
    }

    #[test]
    fn parse_accept_csv() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, "text/csv".parse().unwrap());
        assert_eq!(parse_accept(&headers), ExportFormat::Csv);
    }

    #[test]
    fn parse_accept_ndjson() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            "application/x-ndjson".parse().unwrap(),
        );
        assert_eq!(parse_accept(&headers), ExportFormat::Ndjson);
    }

    #[test]
    fn parse_accept_json_explicit() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        assert_eq!(parse_accept(&headers), ExportFormat::Json);
    }

    #[test]
    fn csv_escapes_commas_and_quotes_in_justification() {
        use chrono::Utc;
        use cream_models::prelude::*;

        let entry = AuditEntry {
            id: AuditEntryId::new(),
            timestamp: Utc::now(),
            agent_id: AgentId::from_uuid(uuid::Uuid::nil()),
            agent_profile_id: AgentProfileId::from_uuid(uuid::Uuid::nil()),
            payment_id: None,
            request: serde_json::json!({ "amount": "99.99", "currency": "USD" }),
            justification: serde_json::json!({
                "summary": "Buying \"widgets, 50 pack\" from vendor"
            }),
            policy_evaluation: PolicyEvaluationRecord {
                rules_evaluated: vec![],
                matching_rules: vec![],
                final_decision: PolicyAction::Approve,
                decision_latency_ms: 1,
            },
            routing_decision: None,
            provider_response: None,
            final_status: PaymentStatus::Settled,
            human_review: None,
            on_chain_tx_hash: None,
        };

        let csv = entries_to_csv(&[entry]).expect("CSV should succeed");
        // The csv crate wraps fields containing commas or quotes in double-quotes
        // and escapes inner double-quotes by doubling them.
        assert!(csv.contains("\"Buying \"\"widgets, 50 pack\"\" from vendor\""));
    }

    #[test]
    fn ndjson_produces_one_line_per_entry() {
        use chrono::Utc;
        use cream_models::prelude::*;

        let entry = AuditEntry {
            id: AuditEntryId::new(),
            timestamp: Utc::now(),
            agent_id: AgentId::from_uuid(uuid::Uuid::nil()),
            agent_profile_id: AgentProfileId::from_uuid(uuid::Uuid::nil()),
            payment_id: None,
            request: serde_json::json!({}),
            justification: serde_json::json!({}),
            policy_evaluation: PolicyEvaluationRecord {
                rules_evaluated: vec![],
                matching_rules: vec![],
                final_decision: PolicyAction::Approve,
                decision_latency_ms: 1,
            },
            routing_decision: None,
            provider_response: None,
            final_status: PaymentStatus::Settled,
            human_review: None,
            on_chain_tx_hash: None,
        };

        let ndjson = entries_to_ndjson(&[entry.clone(), entry]).expect("NDJSON should succeed");
        let lines: Vec<&str> = ndjson.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        // Each line is valid JSON.
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line).expect("each NDJSON line must be valid JSON");
        }
    }

    #[test]
    fn csv_header_row_has_correct_columns() {
        let csv = entries_to_csv(&[]).expect("empty CSV should succeed");
        let first_line = csv.lines().next().expect("CSV must have header");
        assert_eq!(
            first_line,
            "entry_id,timestamp,agent_id,payment_id,amount,currency,status,decision,provider,justification_summary"
        );
    }

    #[test]
    fn sync_export_row_cap_is_10000() {
        assert_eq!(SYNC_EXPORT_ROW_CAP, 10_000);
    }

    #[test]
    fn csv_handles_numeric_amount_values() {
        use chrono::Utc;
        use cream_models::prelude::*;

        let entry = AuditEntry {
            id: AuditEntryId::new(),
            timestamp: Utc::now(),
            agent_id: AgentId::from_uuid(uuid::Uuid::nil()),
            agent_profile_id: AgentProfileId::from_uuid(uuid::Uuid::nil()),
            payment_id: None,
            // amount is a JSON number, not a string
            request: serde_json::json!({ "amount": 149.99, "currency": "SGD" }),
            justification: serde_json::json!({ "summary": "test" }),
            policy_evaluation: PolicyEvaluationRecord {
                rules_evaluated: vec![],
                matching_rules: vec![],
                final_decision: PolicyAction::Approve,
                decision_latency_ms: 1,
            },
            routing_decision: None,
            provider_response: None,
            final_status: PaymentStatus::Settled,
            human_review: None,
            on_chain_tx_hash: None,
        };

        let row = flatten_entry(&entry);
        // amount is at index 4 in the flattened row
        assert_eq!(row[4], "149.99", "numeric JSON amounts must not be empty");

        let csv = entries_to_csv(&[entry]).expect("CSV should succeed");
        assert!(csv.contains("149.99"), "CSV body must contain the numeric amount");
    }
}
