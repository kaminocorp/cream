use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::Utc;
use cream_models::prelude::*;
use cream_policy::{EvaluationContext, PaymentSummary};
use cream_providers::{NormalizedPaymentRequest, ProviderPaymentResponse, TransactionStatus};
use cream_router::IdempotencyOutcome;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::state::AppState;

/// Orchestrates the deterministic 8-step payment lifecycle.
///
/// Steps 1-2 (schema validation, agent identity) are handled by Axum extractors
/// before `process()` is called. Steps 3-8 are implemented here.
pub struct PaymentOrchestrator {
    state: AppState,
}

impl PaymentOrchestrator {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Execute the full payment lifecycle for an authenticated agent request.
    pub async fn process(
        &self,
        agent: &AuthenticatedAgent,
        mut request: PaymentRequest,
    ) -> Result<PaymentResponse, ApiError> {
        // Inject agent_id from the authenticated session.
        request.agent_id = agent.agent.id;

        // --- Step 3: Justification structural validation (before any state) ---
        self.validate_justification(&request)?;

        // Create the payment entity in Pending state.
        let mut payment = Payment::new(request.clone());

        // --- Idempotency check ---
        let outcome = self
            .state
            .idempotency_guard
            .acquire(&request.idempotency_key, &payment.id)
            .await
            .map_err(ApiError::from)?;

        match outcome {
            IdempotencyOutcome::Acquired => { /* proceed */ }
            IdempotencyOutcome::Existing(existing_id) => {
                return Err(ApiError::IdempotencyConflict(existing_id));
            }
        }

        // Persist the new payment.
        self.state.payment_repo.insert_payment(&payment).await?;

        // --- Step 4: Policy engine evaluation ---
        payment.transition(PaymentStatus::Validating)?;
        self.state.payment_repo.update_payment(&payment).await?;

        let decision = self.evaluate_policy(agent, &request).await?;

        match decision.action {
            PolicyAction::Block => {
                payment.transition(PaymentStatus::Blocked)?;
                self.state.payment_repo.update_payment(&payment).await?;
                self.write_audit(&payment, agent, &decision, None, None)
                    .await?;
                if let Err(e) = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key, &payment.id)
                    .await
                {
                    tracing::warn!(
                        payment_id = %payment.id,
                        error = %e,
                        "failed to release idempotency key after policy block"
                    );
                }
                return Err(ApiError::PolicyBlocked {
                    rule_ids: decision.matching_rules.clone(),
                    reason: format!(
                        "payment blocked by {} policy rule(s)",
                        decision.matching_rules.len()
                    ),
                });
            }
            PolicyAction::Escalate => {
                payment.transition(PaymentStatus::PendingApproval)?;
                self.state.payment_repo.update_payment(&payment).await?;
                // Persist the specific rule that triggered escalation so the
                // timeout monitor can use the correct timeout_minutes.
                if let Some(ref rule_id) = decision.escalation_rule_id {
                    if let Err(e) = self
                        .state
                        .payment_repo
                        .persist_escalation_rule(&payment.id, rule_id)
                        .await
                    {
                        tracing::warn!(
                            payment_id = %payment.id,
                            rule_id = %rule_id,
                            error = %e,
                            "failed to persist escalation_rule_id; timeout monitor will use fallback"
                        );
                    }
                }
                self.write_audit(&payment, agent, &decision, None, None)
                    .await?;
                // Don't release idempotency — the payment is still in progress.
                return Ok(PaymentResponse::from(&payment));
            }
            PolicyAction::Approve => {
                payment.transition(PaymentStatus::Approved)?;
                self.state.payment_repo.update_payment(&payment).await?;
            }
        }

        // --- Step 5: Routing engine selection ---
        // --- Step 6: Provider execution with failover ---
        // If routing or provider execution fails after policy approval, we
        // transition directly Approved → Failed (not via Submitted, since no
        // provider was contacted for routing failures). Write an audit entry
        // and release the idempotency key so the agent can retry.
        let routing = match self
            .state
            .route_selector
            .select(&request, &agent.profile)
            .await
            .map_err(ApiError::from)
        {
            Ok(r) => r,
            Err(e) => {
                payment.transition(PaymentStatus::Failed).ok();
                self.state.payment_repo.update_payment(&payment).await.ok();
                self.write_audit(&payment, agent, &decision, None, None)
                    .await
                    .ok();
                if let Err(rel_err) = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key, &payment.id)
                    .await
                {
                    tracing::warn!(
                        payment_id = %payment.id,
                        error = %rel_err,
                        "failed to release idempotency key after routing failure"
                    );
                }
                return Err(e);
            }
        };

        let provider_response = match self
            .execute_with_failover(&mut payment, &request, &routing)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if !payment.status().is_terminal() {
                    payment.transition(PaymentStatus::Failed).ok();
                    self.state.payment_repo.update_payment(&payment).await.ok();
                }
                self.write_audit(&payment, agent, &decision, Some(&routing), None)
                    .await
                    .ok();
                if let Err(rel_err) = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key, &payment.id)
                    .await
                {
                    tracing::warn!(
                        payment_id = %payment.id,
                        error = %rel_err,
                        "failed to release idempotency key after provider failure"
                    );
                }
                return Err(e);
            }
        };

        // --- Step 7: Settlement confirmation ---
        let failure_reason = match provider_response.status {
            TransactionStatus::Settled => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Settled)?;
                None
            }
            TransactionStatus::Pending | TransactionStatus::RequiresAction => {
                payment.transition(PaymentStatus::Submitted)?;
                None
            }
            TransactionStatus::Failed => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider returned failed")
            }
            TransactionStatus::Declined => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider declined the transaction")
            }
            TransactionStatus::Refunded => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider returned refunded")
            }
        };
        self.state.payment_repo.update_payment(&payment).await?;

        // Persist settlement data (amount_settled, settled_currency, failure_reason)
        // to the payments table. These fields are not part of the Payment domain
        // model but are critical for reconciliation and financial reporting.
        self.state
            .payment_repo
            .persist_settlement(
                &payment.id,
                provider_response.amount_settled,
                provider_response.currency,
                failure_reason,
            )
            .await?;

        // --- Step 8: Audit write ---
        self.write_audit(
            &payment,
            agent,
            &decision,
            Some(&routing),
            Some(&provider_response),
        )
        .await?;

        // Mark idempotency as completed.
        if let Err(e) = self
            .state
            .idempotency_guard
            .complete(&request.idempotency_key, &payment.id)
            .await
        {
            tracing::warn!(
                payment_id = %payment.id,
                error = %e,
                "idempotency guard completion failed; payment already persisted"
            );
        }

        Ok(PaymentResponse::from(&payment))
    }

    /// Resume the pipeline for a payment that was approved by a human reviewer.
    /// Picks up from Step 5 (routing).
    ///
    /// On failure, the idempotency key is released so the agent can retry.
    /// On success, the caller (approve handler) is responsible for completing it.
    pub async fn resume_after_approval(
        &self,
        agent: &AuthenticatedAgent,
        mut payment: Payment,
    ) -> Result<PaymentResponse, ApiError> {
        let request = payment.request.clone();

        // Build a minimal policy decision for the audit record.
        let decision = cream_policy::PolicyDecision {
            action: PolicyAction::Approve,
            rules_evaluated: vec![],
            matching_rules: vec![],
            latency_ms: 0,
            escalation_rule_id: None,
        };

        // Step 5: Routing
        // Step 6: Provider execution
        // Same error-recovery as process(): Approved → Failed (direct, not via
        // Submitted) for pre-provider failures, with audit write.
        let routing = match self
            .state
            .route_selector
            .select(&request, &agent.profile)
            .await
            .map_err(ApiError::from)
        {
            Ok(r) => r,
            Err(e) => {
                payment.transition(PaymentStatus::Failed).ok();
                self.state.payment_repo.update_payment(&payment).await.ok();
                self.write_audit(&payment, agent, &decision, None, None)
                    .await
                    .ok();
                // Release idempotency key so the agent can retry.
                if let Err(rel_err) = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key, &payment.id)
                    .await
                {
                    tracing::warn!(
                        payment_id = %payment.id,
                        error = %rel_err,
                        "failed to release idempotency key after routing failure in resume_after_approval"
                    );
                }
                return Err(e);
            }
        };

        let provider_response = match self
            .execute_with_failover(&mut payment, &request, &routing)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if !payment.status().is_terminal() {
                    payment.transition(PaymentStatus::Failed).ok();
                    self.state.payment_repo.update_payment(&payment).await.ok();
                }
                self.write_audit(&payment, agent, &decision, Some(&routing), None)
                    .await
                    .ok();
                // Release idempotency key so the agent can retry.
                if let Err(rel_err) = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key, &payment.id)
                    .await
                {
                    tracing::warn!(
                        payment_id = %payment.id,
                        error = %rel_err,
                        "failed to release idempotency key after provider failure in resume_after_approval"
                    );
                }
                return Err(e);
            }
        };

        // Step 7: Settlement
        let failure_reason = match provider_response.status {
            TransactionStatus::Settled => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Settled)?;
                None
            }
            TransactionStatus::Pending | TransactionStatus::RequiresAction => {
                payment.transition(PaymentStatus::Submitted)?;
                None
            }
            TransactionStatus::Failed => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider returned failed")
            }
            TransactionStatus::Declined => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider declined the transaction")
            }
            TransactionStatus::Refunded => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
                Some("provider returned refunded")
            }
        };
        self.state.payment_repo.update_payment(&payment).await?;

        // Persist settlement data from the provider response.
        self.state
            .payment_repo
            .persist_settlement(
                &payment.id,
                provider_response.amount_settled,
                provider_response.currency,
                failure_reason,
            )
            .await?;

        // Step 8: Audit
        self.write_audit(
            &payment,
            agent,
            &decision,
            Some(&routing),
            Some(&provider_response),
        )
        .await?;

        Ok(PaymentResponse::from(&payment))
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn validate_justification(&self, request: &PaymentRequest) -> Result<(), ApiError> {
        let word_count = request.justification.summary.split_whitespace().count();
        if word_count < 5 {
            return Err(ApiError::JustificationInvalid(format!(
                "justification must be at least 5 words, got {word_count}"
            )));
        }
        Ok(())
    }

    async fn evaluate_policy(
        &self,
        agent: &AuthenticatedAgent,
        request: &PaymentRequest,
    ) -> Result<cream_policy::PolicyDecision, ApiError> {
        let rules = self
            .state
            .payment_repo
            .load_rules(&agent.profile.id)
            .await?;

        let recent_payments: Vec<PaymentSummary> = self
            .state
            .payment_repo
            .load_recent_payments(&agent.agent.id)
            .await?;

        let known_merchants: HashSet<String> = self
            .state
            .payment_repo
            .load_known_merchants(&agent.agent.id)
            .await?;

        let ctx = EvaluationContext {
            request: request.clone(),
            agent: agent.agent.clone(),
            profile: agent.profile.clone(),
            recent_payments,
            known_merchants,
            current_time: Utc::now(),
        };

        let decision = self.state.policy_engine.evaluate(&rules, &ctx)?;
        Ok(decision)
    }

    async fn execute_with_failover(
        &self,
        payment: &mut Payment,
        request: &PaymentRequest,
        routing: &RoutingDecision,
    ) -> Result<ProviderPaymentResponse, ApiError> {
        let normalized = NormalizedPaymentRequest {
            payment_id: payment.id,
            amount: request.amount,
            currency: request.currency,
            recipient_identifier: request.recipient.identifier.clone(),
            recipient_country: request
                .recipient
                .country
                .as_ref()
                .map(|c| c.as_str().to_string()),
            rail: request.preferred_rail,
            description: request.justification.summary.clone(),
            idempotency_key: request.idempotency_key.as_str().to_string(),
        };

        let start = Instant::now();

        for candidate in &routing.candidates {
            // Check circuit breaker.
            match self
                .state
                .circuit_breaker
                .is_allowed(&candidate.provider_id)
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    tracing::debug!(
                        provider = %candidate.provider_id,
                        "skipping: circuit breaker open"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        provider = %candidate.provider_id,
                        error = %e,
                        "circuit breaker check failed, skipping"
                    );
                    continue;
                }
            }

            // Look up provider from registry.
            let provider = match self.state.provider_registry.get(&candidate.provider_id) {
                Some(p) => p,
                None => {
                    tracing::warn!(
                        provider = %candidate.provider_id,
                        "skipping: not in registry"
                    );
                    continue;
                }
            };

            // Attempt payment.
            match provider.initiate_payment(&normalized).await {
                Ok(response) => {
                    if let Err(e) = self
                        .state
                        .circuit_breaker
                        .record_success(&candidate.provider_id)
                        .await
                    {
                        tracing::warn!(
                            provider = %candidate.provider_id,
                            error = %e,
                            "circuit breaker record_success failed; routing may use stale health data"
                        );
                    }

                    payment.set_provider(
                        candidate.provider_id.clone(),
                        response.provider_transaction_id.clone(),
                    )?;

                    tracing::info!(
                        provider = %candidate.provider_id,
                        elapsed_ms = start.elapsed().as_millis() as u64,
                        "payment executed successfully"
                    );

                    return Ok(response);
                }
                Err(e) if e.is_retryable() => {
                    if let Err(cb_err) = self
                        .state
                        .circuit_breaker
                        .record_failure(&candidate.provider_id)
                        .await
                    {
                        tracing::warn!(
                            provider = %candidate.provider_id,
                            error = %cb_err,
                            "circuit breaker record_failure failed; routing may use stale health data"
                        );
                    }
                    tracing::warn!(
                        provider = %candidate.provider_id,
                        error = %e,
                        "provider failed (retryable), trying next candidate"
                    );
                    continue;
                }
                Err(e) => {
                    if let Err(cb_err) = self
                        .state
                        .circuit_breaker
                        .record_failure(&candidate.provider_id)
                        .await
                    {
                        tracing::warn!(
                            provider = %candidate.provider_id,
                            error = %cb_err,
                            "circuit breaker record_failure failed; routing may use stale health data"
                        );
                    }
                    tracing::error!(
                        provider = %candidate.provider_id,
                        error = %e,
                        "provider failed (non-retryable)"
                    );
                    return Err(ApiError::ProviderFailure(e));
                }
            }
        }

        Err(ApiError::AllProvidersUnavailable)
    }

    async fn write_audit(
        &self,
        payment: &Payment,
        agent: &AuthenticatedAgent,
        decision: &cream_policy::PolicyDecision,
        routing: Option<&RoutingDecision>,
        provider_response: Option<&ProviderPaymentResponse>,
    ) -> Result<(), ApiError> {
        let request_json = serde_json::to_value(&payment.request)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize request: {e}")))?;
        let justification_json = serde_json::to_value(&payment.request.justification)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize justification: {e}")))?;

        let entry = AuditEntry {
            id: AuditEntryId::new(),
            timestamp: Utc::now(),
            agent_id: agent.agent.id,
            agent_profile_id: agent.profile.id,
            payment_id: Some(payment.id),
            request: request_json,
            justification: justification_json,
            policy_evaluation: PolicyEvaluationRecord {
                rules_evaluated: decision.rules_evaluated.clone(),
                matching_rules: decision.matching_rules.clone(),
                final_decision: decision.action,
                decision_latency_ms: decision.latency_ms,
            },
            routing_decision: routing.cloned(),
            provider_response: provider_response.map(|r| ProviderResponseRecord {
                provider: payment
                    .provider_id()
                    .cloned()
                    .unwrap_or_else(|| ProviderId::new("unknown")),
                transaction_id: r.provider_transaction_id.clone(),
                status: format!("{:?}", r.status),
                amount_settled: r.amount_settled,
                currency: r.currency,
                latency_ms: 0,
            }),
            final_status: payment.status(),
            human_review: None,
            on_chain_tx_hash: None,
        };

        self.state
            .audit_writer
            .append(&entry, Some(payment.id))
            .await
            .map_err(ApiError::from)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Escalation timeout monitor
// ---------------------------------------------------------------------------

/// Background task that periodically checks for payments stuck in
/// `PendingApproval` past their escalation timeout. Transitions them to
/// `TimedOut` → `Blocked` and writes an audit entry.
pub async fn escalation_timeout_monitor(state: AppState) {
    let interval_secs = state.config.escalation_check_interval_secs;
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    tracing::info!(interval_secs, "escalation timeout monitor started");

    loop {
        interval.tick().await;

        if let Err(e) = check_escalation_timeouts(&state).await {
            tracing::error!(error = %e, "escalation timeout check failed");
        }
    }
}

async fn check_escalation_timeouts(state: &AppState) -> Result<(), ApiError> {
    let expired_ids = state.payment_repo.find_expired_escalations().await?;

    if expired_ids.is_empty() {
        return Ok(());
    }

    tracing::info!(count = expired_ids.len(), "found expired escalations");

    for payment_id in expired_ids {
        if let Some(mut payment) = state.payment_repo.get_payment(&payment_id).await? {
            if payment.status() != PaymentStatus::PendingApproval {
                continue;
            }

            if let Err(e) = payment.transition(PaymentStatus::TimedOut) {
                tracing::warn!(payment_id = %payment_id, error = %e, "timeout transition failed");
                continue;
            }

            // TimedOut is terminal in itself, but per the state machine
            // TimedOut → Blocked is a valid transition for policy enforcement.
            if let Err(e) = payment.transition(PaymentStatus::Blocked) {
                tracing::warn!(payment_id = %payment_id, error = %e, "blocked transition failed");
                // Still persist the TimedOut state.
            }

            // Conditional update: only write if DB status is still pending_approval.
            // If a human approved/rejected concurrently, this is a no-op (race lost).
            let updated = state
                .payment_repo
                .update_payment_if_status(&payment, "pending_approval")
                .await;
            match updated {
                Ok(true) => {
                    tracing::warn!(payment_id = %payment_id, "escalation timed out, payment blocked");

                    // Look up the agent's actual profile_id for a correct audit entry.
                    let profile_id = match sqlx::query_as::<_, (uuid::Uuid,)>(
                        "SELECT profile_id FROM agents WHERE id = $1",
                    )
                    .bind(payment.request.agent_id.as_uuid())
                    .fetch_optional(&state.db)
                    .await
                    {
                        Ok(Some(row)) => AgentProfileId::from_uuid(row.0),
                        Ok(None) => {
                            tracing::warn!(
                                agent_id = %payment.request.agent_id,
                                "agent not found for escalation timeout audit entry, using nil profile_id"
                            );
                            AgentProfileId::from_uuid(uuid::Uuid::nil())
                        }
                        Err(e) => {
                            tracing::warn!(
                                agent_id = %payment.request.agent_id,
                                error = %e,
                                "failed to look up agent profile for escalation timeout audit entry, using nil profile_id"
                            );
                            AgentProfileId::from_uuid(uuid::Uuid::nil())
                        }
                    };

                    // Write audit entry for the timeout/block transition.
                    let request_json = serde_json::to_value(&payment.request)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let justification_json = serde_json::to_value(&payment.request.justification)
                        .unwrap_or_else(|_| serde_json::json!({}));

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
                        human_review: Some(HumanReviewRecord {
                            reviewer_id: "system:escalation_timeout".to_string(),
                            decision: PolicyAction::Block,
                            reason: Some("escalation timed out without human approval".to_string()),
                            decided_at: Utc::now(),
                        }),
                        on_chain_tx_hash: None,
                    };

                    if let Err(e) = state
                        .audit_writer
                        .append(&audit_entry, Some(payment.id))
                        .await
                    {
                        // Retry once after a short delay — transient DB errors are
                        // the most common cause, and the audit trail is a core
                        // compliance invariant for this payment control plane.
                        tracing::warn!(
                            payment_id = %payment_id,
                            error = %e,
                            "escalation timeout audit write failed, retrying once"
                        );
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        if let Err(e2) = state
                            .audit_writer
                            .append(&audit_entry, Some(payment.id))
                            .await
                        {
                            tracing::error!(
                                payment_id = %payment_id,
                                error = %e2,
                                "CRITICAL: escalation timeout audit write failed after retry — \
                                 payment {} was blocked by timeout but has no audit record. \
                                 Manual reconciliation required.",
                                payment_id
                            );
                        }
                    }

                    // Release the idempotency key — the payment is terminally blocked.
                    if let Err(e) = state
                        .idempotency_guard
                        .release(&payment.request.idempotency_key, &payment.id)
                        .await
                    {
                        tracing::warn!(
                            payment_id = %payment_id,
                            error = %e,
                            "failed to release idempotency key after escalation timeout"
                        );
                    }
                }
                Ok(false) => {
                    tracing::info!(
                        payment_id = %payment_id,
                        "escalation timeout skipped: payment status changed concurrently"
                    );
                }
                Err(e) => {
                    tracing::error!(payment_id = %payment_id, error = %e, "failed to update timed-out payment");
                    continue;
                }
            }
        }
    }

    Ok(())
}
