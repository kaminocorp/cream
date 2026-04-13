use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::Utc;
use cream_models::prelude::*;
use cream_policy::{EvaluationContext, PaymentSummary};
use cream_providers::{NormalizedPaymentRequest, ProviderPaymentResponse, TransactionStatus};
use cream_router::{CircuitBreaker, IdempotencyOutcome};

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::metrics;
use crate::notifications::EscalationNotification;
use crate::state::AppState;
use crate::webhook_worker::{enqueue_webhook, WebhookEvent};

/// Apply settlement status transitions based on the provider response.
/// Returns the failure reason (if any) for persistence in the payments table.
///
/// Used by both `process()` and `resume_after_approval()` — extracted to
/// avoid duplicating the same match block in both paths.
fn apply_settlement_transitions(
    payment: &mut Payment,
    response: &ProviderPaymentResponse,
) -> Result<Option<&'static str>, ApiError> {
    match response.status {
        TransactionStatus::Settled => {
            payment.transition(PaymentStatus::Submitted)?;
            payment.transition(PaymentStatus::Settled)?;
            Ok(None)
        }
        TransactionStatus::Pending | TransactionStatus::RequiresAction => {
            payment.transition(PaymentStatus::Submitted)?;
            Ok(None)
        }
        TransactionStatus::Failed => {
            payment.transition(PaymentStatus::Submitted)?;
            payment.transition(PaymentStatus::Failed)?;
            Ok(Some("provider returned failed"))
        }
        TransactionStatus::Declined => {
            payment.transition(PaymentStatus::Submitted)?;
            payment.transition(PaymentStatus::Failed)?;
            Ok(Some("provider declined the transaction"))
        }
        TransactionStatus::Refunded => {
            payment.transition(PaymentStatus::Submitted)?;
            payment.transition(PaymentStatus::Failed)?;
            Ok(Some("provider returned refunded"))
        }
    }
}

/// Read the current circuit breaker state for a provider and emit it as a
/// Prometheus gauge. The gauge uses numeric encoding: 0 = closed, 1 = open,
/// 2 = half_open. Called after every `record_success` / `record_failure` so
/// the metric tracks state transitions in near-real-time.
async fn emit_circuit_breaker_gauge(
    circuit_breaker: &CircuitBreaker,
    provider_id: &cream_models::prelude::ProviderId,
) {
    if let Ok(state) = circuit_breaker.state(provider_id).await {
        let state_label = match state {
            cream_models::prelude::CircuitState::Closed => "closed",
            cream_models::prelude::CircuitState::Open => "open",
            cream_models::prelude::CircuitState::HalfOpen => "half_open",
        };
        ::metrics::gauge!(
            metrics::CIRCUIT_BREAKER_STATE,
            "provider" => provider_id.as_str().to_string(),
            "state" => state_label,
        )
        .set(1.0);
    }
}

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
    #[tracing::instrument(
        skip_all,
        fields(
            payment_id = tracing::field::Empty,
            agent_id = %agent.agent.id,
            agent_name = %agent.agent.name,
            amount = %request.amount,
            currency = ?request.currency,
        )
    )]
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

        // Record payment_id into the span now that we have it.
        tracing::Span::current().record("payment_id", tracing::field::display(&payment.id));

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

        // Record policy metrics.
        ::metrics::histogram!(metrics::POLICY_EVALUATION_DURATION_SECONDS)
            .record(decision.latency_ms as f64 / 1000.0);
        let action_label = match decision.action {
            PolicyAction::Approve => "approve",
            PolicyAction::Block => "block",
            PolicyAction::Escalate => "escalate",
        };
        ::metrics::counter!(metrics::POLICY_DECISION_TOTAL, "action" => action_label).increment(1);

        match decision.action {
            PolicyAction::Block => {
                payment.transition(PaymentStatus::Blocked)?;
                self.state.payment_repo.update_payment(&payment).await?;
                self.write_audit(&payment, agent, &decision, None, None, 0)
                    .await?;
                self.fire_webhook(&payment).await;
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
                self.write_audit(&payment, agent, &decision, None, None, 0)
                    .await?;

                // Fire escalation notification (Slack, email, etc.) — best-effort.
                self.fire_escalation_notification(&payment, agent).await;

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
                self.write_audit(&payment, agent, &decision, None, None, 0)
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

        let (provider_response, provider_latency_ms) = match self
            .execute_with_failover(&mut payment, &request, &routing)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if !payment.status().is_terminal() {
                    payment.transition(PaymentStatus::Failed).ok();
                    self.state.payment_repo.update_payment(&payment).await.ok();
                }
                self.write_audit(&payment, agent, &decision, Some(&routing), None, 0)
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
        let failure_reason = apply_settlement_transitions(&mut payment, &provider_response)?;
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
            provider_latency_ms,
        )
        .await?;

        // Fire webhook for terminal status (settled or failed).
        self.fire_webhook(&payment).await;

        // Record payment outcome metrics.
        let status_label = match payment.status() {
            PaymentStatus::Settled => "settled",
            PaymentStatus::Failed => "failed",
            _ => "other",
        };
        let provider_label = payment
            .provider_id()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let rail_label = format!("{:?}", request.preferred_rail);
        ::metrics::counter!(
            metrics::PAYMENTS_TOTAL,
            "status" => status_label,
            "provider" => provider_label.clone(),
            "rail" => rail_label,
        )
        .increment(1);
        ::metrics::histogram!(
            metrics::PAYMENT_DURATION_SECONDS,
            "provider" => provider_label.clone(),
        )
        .record(provider_latency_ms as f64 / 1000.0);
        ::metrics::histogram!(
            metrics::PROVIDER_REQUEST_DURATION_SECONDS,
            "provider" => provider_label,
        )
        .record(provider_latency_ms as f64 / 1000.0);

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
    #[tracing::instrument(
        skip_all,
        fields(
            payment_id = %payment.id,
            agent_id = %agent.agent.id,
        )
    )]
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
                self.write_audit(&payment, agent, &decision, None, None, 0)
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

        let (provider_response, provider_latency_ms) = match self
            .execute_with_failover(&mut payment, &request, &routing)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if !payment.status().is_terminal() {
                    payment.transition(PaymentStatus::Failed).ok();
                    self.state.payment_repo.update_payment(&payment).await.ok();
                }
                self.write_audit(&payment, agent, &decision, Some(&routing), None, 0)
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
        let failure_reason = apply_settlement_transitions(&mut payment, &provider_response)?;
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
            provider_latency_ms,
        )
        .await?;

        // Fire webhook for terminal status (settled or failed).
        self.fire_webhook(&payment).await;

        Ok(PaymentResponse::from(&payment))
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    #[tracing::instrument(skip_all)]
    fn validate_justification(&self, request: &PaymentRequest) -> Result<(), ApiError> {
        let word_count = request.justification.summary.split_whitespace().count();
        if word_count < 5 {
            return Err(ApiError::JustificationInvalid(format!(
                "justification must be at least 5 words, got {word_count}"
            )));
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(profile_id = %agent.profile.id))]
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

    #[tracing::instrument(skip_all, fields(candidates = routing.candidates.len()))]
    async fn execute_with_failover(
        &self,
        payment: &mut Payment,
        request: &PaymentRequest,
        routing: &RoutingDecision,
    ) -> Result<(ProviderPaymentResponse, u64), ApiError> {
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
                    let elapsed_ms = start.elapsed().as_millis() as u64;

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
                    emit_circuit_breaker_gauge(
                        &self.state.circuit_breaker,
                        &candidate.provider_id,
                    )
                    .await;

                    payment.set_provider(
                        candidate.provider_id.clone(),
                        response.provider_transaction_id.clone(),
                    )?;

                    tracing::info!(
                        provider = %candidate.provider_id,
                        elapsed_ms,
                        "payment executed successfully"
                    );

                    return Ok((response, elapsed_ms));
                }
                Err(e) if e.is_retryable() => {
                    ::metrics::counter!(
                        metrics::PROVIDER_ERRORS_TOTAL,
                        "provider" => candidate.provider_id.as_str().to_string(),
                        "retryable" => "true",
                    )
                    .increment(1);
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
                    emit_circuit_breaker_gauge(
                        &self.state.circuit_breaker,
                        &candidate.provider_id,
                    )
                    .await;
                    tracing::warn!(
                        provider = %candidate.provider_id,
                        error = %e,
                        "provider failed (retryable), trying next candidate"
                    );
                    continue;
                }
                Err(e) => {
                    ::metrics::counter!(
                        metrics::PROVIDER_ERRORS_TOTAL,
                        "provider" => candidate.provider_id.as_str().to_string(),
                        "retryable" => "false",
                    )
                    .increment(1);
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
                    emit_circuit_breaker_gauge(
                        &self.state.circuit_breaker,
                        &candidate.provider_id,
                    )
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

    /// Send an escalation notification via the configured channel (Slack, email,
    /// etc.). Best-effort — never blocks or errors the payment pipeline.
    async fn fire_escalation_notification(
        &self,
        payment: &Payment,
        agent: &AuthenticatedAgent,
    ) {
        // Read the actual escalation timeout from the DB (rule-specific → profile
        // minimum → 60-minute default). The escalation_rule_id was already
        // persisted by the caller before we get here.
        let timeout_minutes = self
            .state
            .payment_repo
            .get_escalation_timeout_minutes(&payment.id)
            .await
            .unwrap_or(60) as u32;

        // Build dashboard deep link if DASHBOARD_BASE_URL is configured.
        let dashboard_url = self
            .state
            .config
            .dashboard_base_url
            .as_ref()
            .map(|base| format!("{}/escalations?payment_id={}", base.trim_end_matches('/'), payment.id));

        let notification = EscalationNotification {
            payment_id: payment.id,
            amount: payment.request.amount,
            currency: payment.request.currency,
            recipient: payment.request.recipient.identifier.clone(),
            agent_name: agent.agent.name.clone(),
            agent_id: agent.agent.id,
            justification_summary: payment.request.justification.summary.clone(),
            timeout_minutes,
            dashboard_url,
        };

        if let Err(e) = self
            .state
            .notification_sender
            .send_escalation(&notification)
            .await
        {
            tracing::warn!(
                payment_id = %payment.id,
                error = %e,
                "escalation notification dispatch failed (non-blocking)"
            );
        }
    }

    /// Fire a webhook event for a terminal payment status. Best-effort — never
    /// blocks or errors the payment pipeline.
    async fn fire_webhook(&self, payment: &Payment) {
        let event_type = match payment.status() {
            PaymentStatus::Settled => "payment.settled",
            PaymentStatus::Failed => "payment.failed",
            PaymentStatus::Blocked => "payment.blocked",
            PaymentStatus::TimedOut => "payment.timed_out",
            _ => return, // non-terminal — no webhook
        };

        let payload = serde_json::json!({
            "payment_id": payment.id.to_string(),
            "status": event_type.split('.').next_back().unwrap_or("unknown"),
            "amount": payment.request.amount.to_string(),
            "currency": payment.request.currency,
            "agent_id": payment.request.agent_id.to_string(),
            "recipient": payment.request.recipient.identifier,
        });

        enqueue_webhook(
            &self.state.redis,
            WebhookEvent {
                event_type: event_type.to_string(),
                payload,
                agent_id: Some(*payment.request.agent_id.as_uuid()),
            },
        )
        .await;
    }

    #[tracing::instrument(skip_all, fields(payment_id = %payment.id, agent_id = %agent.agent.id))]
    async fn write_audit(
        &self,
        payment: &Payment,
        agent: &AuthenticatedAgent,
        decision: &cream_policy::PolicyDecision,
        routing: Option<&RoutingDecision>,
        provider_response: Option<&ProviderPaymentResponse>,
        provider_latency_ms: u64,
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
                // Use serde serialization (snake_case) rather than Debug (PascalCase).
                // TransactionStatus has #[serde(rename_all = "snake_case")], so
                // to_value produces "settled", "requires_action", etc.
                status: serde_json::to_value(r.status)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| format!("{:?}", r.status)),
                amount_settled: r.amount_settled,
                currency: r.currency,
                latency_ms: provider_latency_ms,
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
///
/// Also sends reminder notifications at 50% of the timeout duration.
pub async fn escalation_timeout_monitor(state: AppState) {
    let interval_secs = state.config.escalation_check_interval_secs;
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    tracing::info!(interval_secs, "escalation timeout monitor started");

    loop {
        interval.tick().await;

        // Update the escalation pending gauge on every tick so Prometheus
        // always reflects the current queue depth.
        match update_escalation_pending_gauge(&state).await {
            Ok(()) => {}
            Err(e) => tracing::warn!(error = %e, "escalation pending gauge update failed"),
        }

        // Check for reminders first (50% timeout).
        if let Err(e) = check_escalation_reminders(&state).await {
            tracing::error!(error = %e, "escalation reminder check failed");
        }

        // Then check for full timeouts.
        if let Err(e) = check_escalation_timeouts(&state).await {
            tracing::error!(error = %e, "escalation timeout check failed");
        }
    }
}

/// Query the current count of payments in `pending_approval` status and set
/// the `cream_escalation_pending_count` gauge. This runs on every monitor tick
/// so the metric always reflects the real queue depth.
async fn update_escalation_pending_gauge(state: &AppState) -> Result<(), ApiError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM payments WHERE status = 'pending_approval'",
    )
    .fetch_one(&state.db)
    .await?;

    ::metrics::gauge!(metrics::ESCALATION_PENDING_COUNT).set(count as f64);
    Ok(())
}

/// Send reminder notifications for payments that have passed 50% of their
/// escalation timeout and haven't been reminded yet.
#[tracing::instrument(skip_all)]
async fn check_escalation_reminders(state: &AppState) -> Result<(), ApiError> {
    let reminder_ids = state.payment_repo.find_reminder_due_escalations().await?;

    if reminder_ids.is_empty() {
        return Ok(());
    }

    tracing::info!(count = reminder_ids.len(), "sending escalation reminders");

    for payment_id in reminder_ids {
        if let Some(payment) = state.payment_repo.get_payment(&payment_id).await? {
            if payment.status() != PaymentStatus::PendingApproval {
                continue;
            }

            // Mark reminder as sent BEFORE sending to prevent duplicates on crash/retry.
            if let Err(e) = state.payment_repo.set_reminder_sent(&payment_id).await {
                tracing::warn!(
                    payment_id = %payment_id,
                    error = %e,
                    "failed to set reminder_sent_at, skipping to prevent duplicate"
                );
                continue;
            }

            // Compute real minutes remaining from the escalation timeout config.
            let timeout_minutes = state
                .payment_repo
                .get_escalation_timeout_minutes(&payment_id)
                .await
                .unwrap_or(60) as i64;
            let elapsed_secs = (Utc::now() - payment.updated_at)
                .num_seconds()
                .max(0);
            let remaining = ((timeout_minutes * 60 - elapsed_secs) / 60).max(0) as u32;

            // Resolve real agent name for human-readable notifications.
            let agent_name = crate::extractors::auth::lookup_agent_name(
                &state.db,
                &payment.request.agent_id,
            )
            .await
            .unwrap_or_else(|| format!("agent:{}", payment.request.agent_id));

            let notification = crate::notifications::ReminderNotification {
                payment_id: payment.id,
                amount: payment.request.amount,
                currency: payment.request.currency,
                recipient: payment.request.recipient.identifier.clone(),
                agent_name,
                minutes_remaining: remaining,
                kind: crate::notifications::ReminderKind::Reminder,
            };

            if let Err(e) = state.notification_sender.send_reminder(&notification).await {
                tracing::warn!(
                    payment_id = %payment_id,
                    error = %e,
                    "escalation reminder notification failed (non-blocking)"
                );
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
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
                    let profile_id = match crate::extractors::auth::lookup_profile_id_for_agent(
                        &state.db,
                        &payment.request.agent_id,
                    )
                    .await
                    {
                        Ok(Some(pid)) => pid,
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

                    // Fire webhook for the timed-out/blocked payment.
                    let timeout_event_type = match payment.status() {
                        PaymentStatus::Blocked => "payment.blocked",
                        PaymentStatus::TimedOut => "payment.timed_out",
                        _ => "payment.timed_out",
                    };
                    enqueue_webhook(
                        &state.redis,
                        WebhookEvent {
                            event_type: timeout_event_type.to_string(),
                            payload: serde_json::json!({
                                "payment_id": payment.id.to_string(),
                                "status": "timed_out",
                                "reason": "escalation_timeout",
                                "agent_id": payment.request.agent_id.to_string(),
                                "amount": payment.request.amount.to_string(),
                                "currency": payment.request.currency,
                            }),
                            agent_id: Some(*payment.request.agent_id.as_uuid()),
                        },
                    )
                    .await;

                    // Send timeout notification via configured channels.
                    let timeout_agent_name = crate::extractors::auth::lookup_agent_name(
                        &state.db,
                        &payment.request.agent_id,
                    )
                    .await
                    .unwrap_or_else(|| format!("agent:{}", payment.request.agent_id));

                    let timeout_notification = crate::notifications::ReminderNotification {
                        payment_id: payment.id,
                        amount: payment.request.amount,
                        currency: payment.request.currency,
                        recipient: payment.request.recipient.identifier.clone(),
                        agent_name: timeout_agent_name,
                        minutes_remaining: 0,
                        kind: crate::notifications::ReminderKind::Timeout,
                    };
                    if let Err(e) = state
                        .notification_sender
                        .send_reminder(&timeout_notification)
                        .await
                    {
                        tracing::warn!(
                            payment_id = %payment_id,
                            error = %e,
                            "timeout notification failed (non-blocking)"
                        );
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

// ---------------------------------------------------------------------------
// Credential age monitor (Phase 17-D)
// ---------------------------------------------------------------------------

/// Background task that scans for agents whose API keys haven't been rotated
/// within `CREDENTIAL_ROTATION_WARN_DAYS`. Runs on the same interval as the
/// escalation timeout monitor. Logs a warning per stale key and bumps the
/// `cream_credential_age_warning` counter.
pub async fn credential_age_monitor(state: AppState) {
    let interval_secs = state.config.escalation_check_interval_secs;
    let warn_days = state.config.credential_rotation_warn_days as i64;
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    tracing::info!(
        interval_secs,
        warn_days,
        "credential age monitor started"
    );

    loop {
        interval.tick().await;

        if let Err(e) = check_credential_ages(&state, warn_days).await {
            tracing::error!(error = %e, "credential age check failed");
        }
    }
}

#[tracing::instrument(skip_all)]
async fn check_credential_ages(state: &AppState, warn_days: i64) -> Result<(), ApiError> {
    #[derive(sqlx::FromRow)]
    struct StaleAgent {
        id: uuid::Uuid,
        name: String,
        key_rotated_at: chrono::DateTime<Utc>,
    }

    let stale_agents: Vec<StaleAgent> = sqlx::query_as(
        "SELECT id, name, key_rotated_at FROM agents
         WHERE status = 'active'
           AND key_rotated_at < now() - make_interval(days => $1::int)
         ORDER BY key_rotated_at ASC
         LIMIT 100",
    )
    .bind(warn_days as i32)
    .fetch_all(&state.db)
    .await?;

    if stale_agents.is_empty() {
        return Ok(());
    }

    for agent in &stale_agents {
        let age_days = (Utc::now() - agent.key_rotated_at).num_days();
        tracing::warn!(
            agent_id = %agent.id,
            agent_name = %agent.name,
            key_age_days = age_days,
            threshold_days = warn_days,
            "agent API key has not been rotated within the configured threshold"
        );
        ::metrics::counter!(crate::metrics::CREDENTIAL_AGE_WARNING, "agent_id" => agent.id.to_string())
            .increment(1);
    }

    tracing::info!(
        count = stale_agents.len(),
        "credential age warnings emitted"
    );

    Ok(())
}
