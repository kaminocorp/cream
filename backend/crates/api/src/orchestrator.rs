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

        // --- Step 3: Justification structural validation ---
        if let Err(e) = self.validate_justification(&request) {
            // Release the idempotency lock since the payment is abandoned.
            let _ = self
                .state
                .idempotency_guard
                .release(&request.idempotency_key)
                .await;
            return Err(e);
        }

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
                let _ = self
                    .state
                    .idempotency_guard
                    .release(&request.idempotency_key)
                    .await;
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
        let routing = self
            .state
            .route_selector
            .select(&request, &agent.profile)
            .await
            .map_err(ApiError::from)?;

        // --- Step 6: Provider execution with failover ---
        let provider_response = self
            .execute_with_failover(&mut payment, &request, &routing)
            .await?;

        // --- Step 7: Settlement confirmation ---
        match provider_response.status {
            TransactionStatus::Settled => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Settled)?;
            }
            TransactionStatus::Pending | TransactionStatus::RequiresAction => {
                payment.transition(PaymentStatus::Submitted)?;
            }
            TransactionStatus::Failed
            | TransactionStatus::Declined
            | TransactionStatus::Refunded => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
            }
        }
        self.state.payment_repo.update_payment(&payment).await?;

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
        let _ = self
            .state
            .idempotency_guard
            .complete(&request.idempotency_key, &payment.id)
            .await;

        Ok(PaymentResponse::from(&payment))
    }

    /// Resume the pipeline for a payment that was approved by a human reviewer.
    /// Picks up from Step 5 (routing).
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
        };

        // Step 5: Routing
        let routing = self
            .state
            .route_selector
            .select(&request, &agent.profile)
            .await
            .map_err(ApiError::from)?;

        // Step 6: Provider execution
        let provider_response = self
            .execute_with_failover(&mut payment, &request, &routing)
            .await?;

        // Step 7: Settlement
        match provider_response.status {
            TransactionStatus::Settled => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Settled)?;
            }
            TransactionStatus::Pending | TransactionStatus::RequiresAction => {
                payment.transition(PaymentStatus::Submitted)?;
            }
            TransactionStatus::Failed
            | TransactionStatus::Declined
            | TransactionStatus::Refunded => {
                payment.transition(PaymentStatus::Submitted)?;
                payment.transition(PaymentStatus::Failed)?;
            }
        }
        self.state.payment_repo.update_payment(&payment).await?;

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
                    let _ = self
                        .state
                        .circuit_breaker
                        .record_success(&candidate.provider_id)
                        .await;

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
                    let _ = self
                        .state
                        .circuit_breaker
                        .record_failure(&candidate.provider_id)
                        .await;
                    tracing::warn!(
                        provider = %candidate.provider_id,
                        error = %e,
                        "provider failed (retryable), trying next candidate"
                    );
                    continue;
                }
                Err(e) => {
                    let _ = self
                        .state
                        .circuit_breaker
                        .record_failure(&candidate.provider_id)
                        .await;
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

            if let Err(e) = state.payment_repo.update_payment(&payment).await {
                tracing::error!(payment_id = %payment_id, error = %e, "failed to update timed-out payment");
                continue;
            }

            tracing::warn!(payment_id = %payment_id, "escalation timed out, payment blocked");
        }
    }

    Ok(())
}
