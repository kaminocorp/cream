use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use cream_models::prelude::*;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::extractors::json::ValidatedJson;
use crate::orchestrator::PaymentOrchestrator;
use crate::state::AppState;

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
pub async fn get_status(
    State(state): State<AppState>,
    agent: AuthenticatedAgent,
    Path(id): Path<String>,
) -> Result<Json<PaymentDetail>, ApiError> {
    let payment_id = id
        .parse::<PaymentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid payment ID: {e}")))?;

    let payment = state
        .payment_repo
        .get_payment_for_agent(&payment_id, &agent.agent.id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("payment {payment_id}")))?;

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
/// No agent authentication — this endpoint is for human reviewers
/// (dashboard auth in Phase 10). The `reviewer_id` is passed in the body.
pub async fn approve(
    State(state): State<AppState>,
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

    // Transition to Approved.
    payment.transition(PaymentStatus::Approved)?;
    state.payment_repo.update_payment(&payment).await?;

    // Write the human review audit entry.
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
        agent_profile_id: AgentProfileId::from_uuid(*payment.request.agent_id.as_uuid()),
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

    // Resume the pipeline from routing onwards.
    // Look up the agent + profile for routing.
    let agent_row: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT profile_id FROM agents WHERE id = $1")
            .bind(payment.request.agent_id.as_uuid())
            .fetch_optional(&state.db)
            .await?;

    let profile_id = agent_row
        .ok_or_else(|| ApiError::NotFound("agent".to_string()))?
        .0;

    let agent_json = serde_json::json!({
        "id": payment.request.agent_id.to_string(),
        "profile_id": format!("prof_{}", profile_id),
        "name": "system-approved",
        "status": "active",
        "created_at": Utc::now(),
        "updated_at": Utc::now(),
    });
    let agent: Agent = serde_json::from_value(agent_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize agent: {e}")))?;

    let profile_row = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            i32,
            Option<rust_decimal::Decimal>,
            Option<rust_decimal::Decimal>,
            Option<rust_decimal::Decimal>,
            Option<rust_decimal::Decimal>,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            Option<rust_decimal::Decimal>,
            Option<String>,
            chrono::DateTime<Utc>,
            chrono::DateTime<Utc>,
        ),
    >(
        "SELECT id, name, version, max_per_transaction, max_daily_spend,
                max_weekly_spend, max_monthly_spend, allowed_categories,
                allowed_rails, geographic_restrictions, escalation_threshold,
                timezone, created_at, updated_at
         FROM agent_profiles WHERE id = $1",
    )
    .bind(profile_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("agent profile".to_string()))?;

    let profile_json = serde_json::json!({
        "id": format!("prof_{}", profile_row.0),
        "name": profile_row.1,
        "version": profile_row.2,
        "max_per_transaction": profile_row.3,
        "max_daily_spend": profile_row.4,
        "max_weekly_spend": profile_row.5,
        "max_monthly_spend": profile_row.6,
        "allowed_categories": profile_row.7,
        "allowed_rails": profile_row.8,
        "geographic_restrictions": profile_row.9,
        "escalation_threshold": profile_row.10,
        "timezone": profile_row.11,
        "created_at": profile_row.12,
        "updated_at": profile_row.13,
    });
    let profile: AgentProfile = serde_json::from_value(profile_json)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize profile: {e}")))?;

    let auth_agent = crate::extractors::auth::AuthenticatedAgent { agent, profile };
    let orchestrator = PaymentOrchestrator::new(state);
    let response = orchestrator
        .resume_after_approval(&auth_agent, payment)
        .await?;
    Ok(Json(response))
}

/// `POST /v1/payments/{id}/reject` — human-reject an escalated payment.
pub async fn reject(
    State(state): State<AppState>,
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

    payment.transition(PaymentStatus::Rejected)?;
    state.payment_repo.update_payment(&payment).await?;

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
        agent_profile_id: AgentProfileId::from_uuid(*payment.request.agent_id.as_uuid()),
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

    Ok(Json(PaymentResponse::from(&payment)))
}
