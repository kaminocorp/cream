use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use cream_models::prelude::*;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extractors::auth::{AuthenticatedAgent, AuthenticatedOperator, AuthenticatedPrincipal};
use crate::extractors::json::ValidatedJson;
use crate::orchestrator::PaymentOrchestrator;
use crate::state::AppState;

/// Validate reviewer_id and optional reason at the API boundary.
///
/// `HumanReviewRecord`'s custom `Deserialize` enforces these invariants, but
/// the approve/reject handlers construct the record via struct literal (not
/// deserialization), bypassing those guards. Since the data is written to the
/// append-only audit ledger, invalid values would become permanent.
fn validate_review_fields(reviewer_id: &str, reason: &Option<String>) -> Result<(), ApiError> {
    if reviewer_id.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "reviewer_id must not be empty — audit trail requires reviewer identity".to_string(),
        ));
    }
    if reviewer_id.len() > cream_models::prelude::MAX_REVIEWER_ID_LEN {
        return Err(ApiError::ValidationError(format!(
            "reviewer_id exceeds maximum length of {} characters (got {})",
            cream_models::prelude::MAX_REVIEWER_ID_LEN,
            reviewer_id.len()
        )));
    }
    if let Some(ref r) = reason {
        if r.trim().is_empty() {
            return Err(ApiError::ValidationError(
                "reason must not be empty or whitespace-only when provided — omit the field instead"
                    .to_string(),
            ));
        }
        if r.len() > cream_models::prelude::MAX_REVIEW_REASON_LEN {
            return Err(ApiError::ValidationError(format!(
                "reason exceeds maximum length of {} characters (got {})",
                cream_models::prelude::MAX_REVIEW_REASON_LEN,
                r.len()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Inbound request body for `POST /v1/payments`.
/// The `agent_id` is injected from the authenticated session, not from the body.
#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub amount: rust_decimal::Decimal,
    pub currency: Currency,
    pub recipient: Recipient,
    pub preferred_rail: Option<RailPreference>,
    pub justification: Justification,
    pub metadata: Option<PaymentMetadata>,
    pub idempotency_key: String,
}

/// Request body for `POST /v1/payments/{id}/approve`.
#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub reviewer_id: String,
    pub reason: Option<String>,
}

/// Request body for `POST /v1/payments/{id}/reject`.
#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reviewer_id: String,
    pub reason: Option<String>,
}

/// Response for payment detail endpoints.
#[derive(serde::Serialize)]
pub struct PaymentDetail {
    pub payment: PaymentResponse,
    pub audit_entries: Vec<AuditEntry>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /v1/payments` — initiate a payment with structured justification.
pub async fn initiate(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    ValidatedJson(body): ValidatedJson<CreatePaymentRequest>,
) -> Result<(StatusCode, Json<PaymentResponse>), ApiError> {
    // Validate amount at the API boundary. PaymentRequest's custom Deserialize
    // enforces this when loading from JSON/DB, but here we construct via struct
    // literal — so we must check explicitly to avoid a raw DB constraint error.
    if body.amount <= Decimal::ZERO {
        return Err(ApiError::ValidationError(format!(
            "amount must be positive, got {}",
            body.amount
        )));
    }

    let idempotency_key = IdempotencyKey::try_new(body.idempotency_key)
        .map_err(|e| ApiError::ValidationError(format!("invalid idempotency_key: {e}")))?;

    let request = PaymentRequest {
        agent_id: agent.agent.id,
        amount: body.amount,
        currency: body.currency,
        recipient: body.recipient,
        preferred_rail: body.preferred_rail.unwrap_or(RailPreference::Auto),
        justification: body.justification,
        metadata: body.metadata,
        idempotency_key,
    };

    let orchestrator = PaymentOrchestrator::new(state);
    let response = orchestrator.process(&agent, request).await?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /v1/payments/{id}` — retrieve payment status and audit trail.
///
/// Operators may view any payment. Agents may only view their own.
pub async fn get_status(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<String>,
) -> Result<Json<PaymentDetail>, ApiError> {
    let payment_id = id
        .parse::<PaymentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid payment ID: {e}")))?;

    // Operators see any payment; agents are scoped to their own.
    let payment = match &principal {
        AuthenticatedPrincipal::Operator(_) => {
            state
                .payment_repo
                .get_payment(&payment_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("payment {payment_id}")))?
        }
        AuthenticatedPrincipal::Agent(agent) => {
            state
                .payment_repo
                .get_payment_for_agent(&payment_id, &agent.agent.id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("payment {payment_id}")))?
        }
    };

    let audit_entries = state
        .audit_reader
        .get_by_payment(payment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(PaymentDetail {
        payment: PaymentResponse::from(&payment),
        audit_entries,
    }))
}

/// `POST /v1/payments/{id}/approve` — human-approve an escalated payment.
///
/// **Operator-only.** Phase 15.1 closes a pre-existing security gap where
/// this endpoint was entirely unauthenticated — anyone reaching the API
/// could approve any pending payment. The caller must present the shared
/// `OPERATOR_API_KEY` via `Authorization: Bearer …`. The `reviewer_id` in
/// the body is still free-text and stored verbatim in the audit ledger for
/// accountability; 16-A will replace it with the authenticated operator's
/// identity automatically.
pub async fn approve(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<ApproveRequest>,
) -> Result<Json<PaymentResponse>, ApiError> {
    let payment_id = id
        .parse::<PaymentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid payment ID: {e}")))?;

    let mut payment = state
        .payment_repo
        .get_payment(&payment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("payment {payment_id}")))?;

    if payment.status() != PaymentStatus::PendingApproval {
        return Err(ApiError::ValidationError(format!(
            "payment is in {:?} state, not PendingApproval",
            payment.status()
        )));
    }

    // Validate review fields before any state mutation — these go to the
    // append-only audit ledger, so invalid values would be permanent.
    validate_review_fields(&body.reviewer_id, &body.reason)?;

    // Transition to Approved with conditional update to prevent race with escalation monitor.
    payment.transition(PaymentStatus::Approved)?;
    let updated = state
        .payment_repo
        .update_payment_if_status(&payment, "pending_approval")
        .await?;
    if !updated {
        return Err(ApiError::ValidationError(
            "payment status changed concurrently (possibly timed out)".to_string(),
        ));
    }

    // Look up the agent + profile for audit and routing.
    let (agent, profile) = crate::extractors::auth::lookup_agent_by_id(
        &state.db,
        &payment.request.agent_id,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound("agent".to_string()))?;

    let auth_agent = crate::extractors::auth::AuthenticatedAgent { agent, profile };

    // Write the human review audit entry (after profile lookup so we have the correct profile_id).
    let review = HumanReviewRecord {
        reviewer_id: body.reviewer_id,
        decision: PolicyAction::Approve,
        reason: body.reason,
        decided_at: Utc::now(),
    };

    let request_json = serde_json::to_value(&payment.request)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize request: {e}")))?;
    let justification_json = serde_json::to_value(&payment.request.justification)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize justification: {e}")))?;

    let audit_entry = AuditEntry {
        id: AuditEntryId::new(),
        timestamp: Utc::now(),
        agent_id: payment.request.agent_id,
        agent_profile_id: auth_agent.profile.id,
        payment_id: Some(payment.id),
        request: request_json,
        justification: justification_json,
        policy_evaluation: PolicyEvaluationRecord {
            rules_evaluated: vec![],
            matching_rules: vec![],
            final_decision: PolicyAction::Approve,
            decision_latency_ms: 0,
        },
        routing_decision: None,
        provider_response: None,
        final_status: payment.status(),
        human_review: Some(review),
        on_chain_tx_hash: None,
    };
    state
        .audit_writer
        .append(&audit_entry, Some(payment.id))
        .await
        .map_err(ApiError::from)?;

    let idempotency_key = payment.request.idempotency_key.clone();
    let payment_id_for_idemp = payment.id;

    let orchestrator = PaymentOrchestrator::new(state.clone());
    let response = orchestrator
        .resume_after_approval(&auth_agent, payment)
        .await?;

    // Complete the idempotency key now that the payment lifecycle is finished.
    if let Err(e) = state
        .idempotency_guard
        .complete(&idempotency_key, &payment_id_for_idemp)
        .await
    {
        tracing::warn!(
            payment_id = %payment_id_for_idemp,
            error = %e,
            "idempotency guard completion failed after approval; payment already persisted"
        );
    }

    Ok(Json(response))
}

/// `POST /v1/payments/{id}/reject` — human-reject an escalated payment.
///
/// **Operator-only** — see the security note on [`approve`]. Same Phase 15.1
/// auth gate applies.
pub async fn reject(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<RejectRequest>,
) -> Result<Json<PaymentResponse>, ApiError> {
    let payment_id = id
        .parse::<PaymentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid payment ID: {e}")))?;

    let mut payment = state
        .payment_repo
        .get_payment(&payment_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("payment {payment_id}")))?;

    if payment.status() != PaymentStatus::PendingApproval {
        return Err(ApiError::ValidationError(format!(
            "payment is in {:?} state, not PendingApproval",
            payment.status()
        )));
    }

    // Validate review fields before any state mutation — these go to the
    // append-only audit ledger, so invalid values would be permanent.
    validate_review_fields(&body.reviewer_id, &body.reason)?;

    payment.transition(PaymentStatus::Rejected)?;
    let updated = state
        .payment_repo
        .update_payment_if_status(&payment, "pending_approval")
        .await?;
    if !updated {
        return Err(ApiError::ValidationError(
            "payment status changed concurrently (possibly timed out)".to_string(),
        ));
    }

    // Look up the agent's actual profile_id for a correct audit entry.
    let profile_id = crate::extractors::auth::lookup_profile_id_for_agent(
        &state.db,
        &payment.request.agent_id,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound("agent".to_string()))?;

    // Write audit entry with human review record.
    let review = HumanReviewRecord {
        reviewer_id: body.reviewer_id,
        decision: PolicyAction::Block,
        reason: body.reason,
        decided_at: Utc::now(),
    };

    let request_json = serde_json::to_value(&payment.request)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize request: {e}")))?;
    let justification_json = serde_json::to_value(&payment.request.justification)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize justification: {e}")))?;

    let audit_entry = AuditEntry {
        id: AuditEntryId::new(),
        timestamp: Utc::now(),
        agent_id: payment.request.agent_id,
        agent_profile_id: profile_id,
        payment_id: Some(payment.id),
        request: request_json,
        justification: justification_json,
        policy_evaluation: PolicyEvaluationRecord {
            rules_evaluated: vec![],
            matching_rules: vec![],
            final_decision: PolicyAction::Block,
            decision_latency_ms: 0,
        },
        routing_decision: None,
        provider_response: None,
        final_status: payment.status(),
        human_review: Some(review),
        on_chain_tx_hash: None,
    };
    state
        .audit_writer
        .append(&audit_entry, Some(payment.id))
        .await
        .map_err(ApiError::from)?;

    // Release the idempotency key — the payment is terminally rejected.
    if let Err(e) = state
        .idempotency_guard
        .release(&payment.request.idempotency_key, &payment.id)
        .await
    {
        tracing::warn!(
            payment_id = %payment.id,
            error = %e,
            "failed to release idempotency key after rejection"
        );
    }

    Ok(Json(PaymentResponse::from(&payment)))
}
