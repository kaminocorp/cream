use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cream_models::prelude::*;
use cream_policy::PaymentSummary;
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Trait — mockable boundary for orchestrator unit tests
// ---------------------------------------------------------------------------

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// Persist a newly created payment.
    async fn insert_payment(&self, payment: &Payment) -> Result<(), ApiError>;

    /// Load a payment by ID, regardless of agent ownership.
    async fn get_payment(&self, id: &PaymentId) -> Result<Option<Payment>, ApiError>;

    /// Load a payment by ID, scoped to a specific agent.
    async fn get_payment_for_agent(
        &self,
        id: &PaymentId,
        agent_id: &AgentId,
    ) -> Result<Option<Payment>, ApiError>;

    /// Persist the current state of a payment (status, provider fields, settlement).
    async fn update_payment(&self, payment: &Payment) -> Result<(), ApiError>;

    /// Load enabled policy rules for an agent profile, ordered by priority.
    async fn load_rules(&self, profile_id: &AgentProfileId) -> Result<Vec<PolicyRule>, ApiError>;

    /// Load recent payments (last 30 days, non-terminal) for velocity/spend checks.
    async fn load_recent_payments(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<PaymentSummary>, ApiError>;

    /// Load the set of merchant identifiers this agent has previously settled with.
    async fn load_known_merchants(&self, agent_id: &AgentId) -> Result<HashSet<String>, ApiError>;

    /// Persist the ID of the policy rule that triggered escalation.
    /// Called when the policy engine returns Escalate so the timeout monitor
    /// can use the correct rule's timeout_minutes.
    async fn persist_escalation_rule(
        &self,
        payment_id: &PaymentId,
        rule_id: &cream_models::prelude::PolicyRuleId,
    ) -> Result<(), ApiError>;

    /// Find payments stuck in `pending_approval` past their escalation timeout.
    async fn find_expired_escalations(&self) -> Result<Vec<PaymentId>, ApiError>;

    /// Conditionally update a payment only if its current DB status matches `expected_status`.
    /// Returns `true` if the row was updated, `false` if status had already changed (race lost).
    async fn update_payment_if_status(
        &self,
        payment: &Payment,
        expected_status: &str,
    ) -> Result<bool, ApiError>;

    /// Persist settlement data returned by the payment provider.
    ///
    /// Called after provider execution to write `amount_settled`, `settled_currency`,
    /// and optionally `failure_reason` to the payments table. These columns exist in
    /// the schema but are not part of the `Payment` domain model (which tracks status
    /// and provider attribution, not settlement details).
    async fn persist_settlement(
        &self,
        payment_id: &PaymentId,
        amount_settled: rust_decimal::Decimal,
        settled_currency: cream_models::prelude::Currency,
        failure_reason: Option<&str>,
    ) -> Result<(), ApiError>;
}

// ---------------------------------------------------------------------------
// PostgreSQL implementation
// ---------------------------------------------------------------------------

pub struct PgPaymentRepository {
    pool: PgPool,
}

impl PgPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// Row types for SQLx deserialization.
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PaymentRow {
    id: uuid::Uuid,
    agent_id: uuid::Uuid,
    idempotency_key: String,
    amount: Decimal,
    currency: String,
    recipient: serde_json::Value,
    preferred_rail: String,
    justification: serde_json::Value,
    metadata: Option<serde_json::Value>,
    status: String,
    provider_id: Option<String>,
    provider_tx_id: Option<String>,
    amount_settled: Option<Decimal>,
    settled_currency: Option<String>,
    failure_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PaymentRow {
    fn into_payment(self) -> Result<Payment, ApiError> {
        let request = PaymentRequest {
            agent_id: AgentId::from_uuid(self.agent_id),
            amount: self.amount,
            currency: serde_json::from_value(serde_json::json!(self.currency))
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid currency: {e}")))?,
            recipient: serde_json::from_value(self.recipient)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid recipient: {e}")))?,
            preferred_rail: serde_json::from_value(serde_json::json!(self.preferred_rail))
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid rail: {e}")))?,
            justification: serde_json::from_value(self.justification)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid justification: {e}")))?,
            metadata: self
                .metadata
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid metadata: {e}")))?,
            idempotency_key: IdempotencyKey::try_new(self.idempotency_key)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid idempotency key: {e}")))?,
        };

        // Reconstruct Payment via JSON round-trip to respect the custom Deserialize
        // that validates state machine invariants.
        let payment_json = serde_json::json!({
            "id": self.id.to_string(),
            "request": serde_json::to_value(&request)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize request: {e}")))?,
            "status": self.status,
            "provider_id": self.provider_id,
            "provider_transaction_id": self.provider_tx_id,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        });

        serde_json::from_value(payment_json)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize payment: {e}")))
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PolicyRuleRow {
    id: uuid::Uuid,
    profile_id: uuid::Uuid,
    rule_type: Option<String>,
    priority: i32,
    condition: serde_json::Value,
    action: String,
    escalation: Option<serde_json::Value>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PolicyRuleRow {
    fn into_rule(self) -> Result<PolicyRule, ApiError> {
        let rule_json = serde_json::json!({
            "id": format!("rule_{}", self.id),
            "profile_id": format!("prof_{}", self.profile_id),
            "rule_type": self.rule_type,
            "priority": self.priority,
            "condition": self.condition,
            "action": self.action,
            "escalation": self.escalation,
            "enabled": self.enabled,
        });

        serde_json::from_value(rule_json)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("deserialize policy rule: {e}")))
    }
}

#[derive(sqlx::FromRow)]
struct RecentPaymentRow {
    amount: Decimal,
    currency: String,
    recipient_identifier: Option<String>,
    category: Option<String>,
    status: String,
    preferred_rail: String,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl PaymentRepository for PgPaymentRepository {
    async fn insert_payment(&self, payment: &Payment) -> Result<(), ApiError> {
        let req = &payment.request;
        let recipient_json = serde_json::to_value(&req.recipient)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize recipient: {e}")))?;
        let justification_json = serde_json::to_value(&req.justification)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize justification: {e}")))?;
        let metadata_json = req
            .metadata
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize metadata: {e}")))?;
        let currency_str = serde_json::to_value(req.currency)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize currency: {e}")))?;
        let rail_str = serde_json::to_value(req.preferred_rail)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize rail: {e}")))?;

        sqlx::query(
            r#"INSERT INTO payments
                (id, agent_id, idempotency_key, amount, currency, recipient,
                 preferred_rail, justification, metadata, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        )
        .bind(payment.id.as_uuid())
        .bind(req.agent_id.as_uuid())
        .bind(req.idempotency_key.as_str())
        .bind(req.amount)
        .bind(currency_str.as_str().ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "currency serialized to non-string JSON value"
            ))
        })?)
        .bind(&recipient_json)
        .bind(rail_str.as_str().ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "preferred_rail serialized to non-string JSON value"
            ))
        })?)
        .bind(&justification_json)
        .bind(&metadata_json)
        .bind(payment.status().to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_payment(&self, id: &PaymentId) -> Result<Option<Payment>, ApiError> {
        let row: Option<PaymentRow> = sqlx::query_as(
            "SELECT id, agent_id, idempotency_key, amount, currency, recipient,
                    preferred_rail, justification, metadata, status, provider_id,
                    provider_tx_id, amount_settled, settled_currency, failure_reason,
                    created_at, updated_at
             FROM payments WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_payment()).transpose()
    }

    async fn get_payment_for_agent(
        &self,
        id: &PaymentId,
        agent_id: &AgentId,
    ) -> Result<Option<Payment>, ApiError> {
        let row: Option<PaymentRow> = sqlx::query_as(
            "SELECT id, agent_id, idempotency_key, amount, currency, recipient,
                    preferred_rail, justification, metadata, status, provider_id,
                    provider_tx_id, amount_settled, settled_currency, failure_reason,
                    created_at, updated_at
             FROM payments WHERE id = $1 AND agent_id = $2",
        )
        .bind(id.as_uuid())
        .bind(agent_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_payment()).transpose()
    }

    async fn update_payment(&self, payment: &Payment) -> Result<(), ApiError> {
        let status_str = payment.status().to_string();
        let provider_id_str = payment.provider_id().map(|p| p.as_str().to_string());
        let provider_tx_id = payment.provider_transaction_id().map(|s| s.to_string());

        sqlx::query(
            "UPDATE payments
             SET status = $1, provider_id = $2, provider_tx_id = $3, updated_at = now()
             WHERE id = $4",
        )
        .bind(&status_str)
        .bind(&provider_id_str)
        .bind(&provider_tx_id)
        .bind(payment.id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_rules(&self, profile_id: &AgentProfileId) -> Result<Vec<PolicyRule>, ApiError> {
        let rows: Vec<PolicyRuleRow> = sqlx::query_as(
            "SELECT id, profile_id, rule_type, priority, condition, action,
                    escalation, enabled, created_at, updated_at
             FROM policy_rules
             WHERE profile_id = $1 AND enabled = true
             ORDER BY priority ASC",
        )
        .bind(profile_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_rule()).collect()
    }

    async fn load_recent_payments(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<PaymentSummary>, ApiError> {
        let rows: Vec<RecentPaymentRow> = sqlx::query_as(
            r#"SELECT amount, currency, recipient->>'identifier' as recipient_identifier,
                      justification->>'category' as category,
                      status, preferred_rail, created_at
               FROM payments
               WHERE agent_id = $1
                 AND created_at > now() - interval '30 days'
                 AND status NOT IN ('failed', 'blocked', 'rejected', 'timed_out')"#,
        )
        .bind(agent_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let currency: Currency = serde_json::from_value(serde_json::json!(row.currency))
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!(
                        "corrupted currency '{}' in payments table: {e}",
                        row.currency
                    ))
                })?;
            let category: PaymentCategory = row
                .category
                .as_deref()
                .map(|c| serde_json::from_value(serde_json::json!(c)))
                .transpose()
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("corrupted category in payments table: {e}"))
                })?
                .unwrap_or(PaymentCategory::Other("unknown".to_string()));
            let status: PaymentStatus = serde_json::from_value(serde_json::json!(row.status))
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!(
                        "corrupted status '{}' in payments table: {e}",
                        row.status
                    ))
                })?;
            let rail: RailPreference =
                serde_json::from_value(serde_json::json!(row.preferred_rail)).map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!(
                        "corrupted preferred_rail '{}' in payments table: {e}",
                        row.preferred_rail
                    ))
                })?;

            summaries.push(PaymentSummary {
                amount: row.amount,
                currency,
                recipient_identifier: row.recipient_identifier.unwrap_or_default(),
                category,
                status,
                rail,
                created_at: row.created_at,
            });
        }

        Ok(summaries)
    }

    async fn load_known_merchants(&self, agent_id: &AgentId) -> Result<HashSet<String>, ApiError> {
        let rows: Vec<(Option<String>,)> = sqlx::query_as(
            r#"SELECT DISTINCT recipient->>'identifier' as identifier
               FROM payments
               WHERE agent_id = $1 AND status = 'settled'"#,
        )
        .bind(agent_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|(id,)| id)
            .map(|s| s.to_ascii_lowercase())
            .collect())
    }

    async fn persist_escalation_rule(
        &self,
        payment_id: &PaymentId,
        rule_id: &cream_models::prelude::PolicyRuleId,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE payments SET escalation_rule_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(rule_id.as_uuid())
        .bind(payment_id.as_uuid())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_expired_escalations(&self) -> Result<Vec<PaymentId>, ApiError> {
        // When escalation_rule_id is set, use that specific rule's timeout.
        // Fallback to MIN timeout across all profile escalation rules for
        // legacy payments that don't have escalation_rule_id set.
        // Final fallback: 60 minutes default if all escalation rules are
        // disabled/deleted — prevents payments stuck in pending_approval forever.
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            r#"SELECT DISTINCT p.id
               FROM payments p
               JOIN agents a ON a.id = p.agent_id
               WHERE p.status = 'pending_approval'
                 AND p.updated_at + make_interval(
                     mins := COALESCE(
                         (SELECT (pr.escalation->>'timeout_minutes')::int
                          FROM policy_rules pr
                          WHERE pr.id = p.escalation_rule_id
                            AND pr.escalation IS NOT NULL),
                         (SELECT MIN((pr2.escalation->>'timeout_minutes')::int)
                          FROM policy_rules pr2
                          WHERE pr2.profile_id = a.profile_id
                            AND pr2.escalation IS NOT NULL
                            AND pr2.enabled = true),
                         60
                     )
                 ) < now()"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id,)| PaymentId::from_uuid(id))
            .collect())
    }

    async fn update_payment_if_status(
        &self,
        payment: &Payment,
        expected_status: &str,
    ) -> Result<bool, ApiError> {
        let status_str = payment.status().to_string();
        let provider_id_str = payment.provider_id().map(|p| p.as_str().to_string());
        let provider_tx_id = payment.provider_transaction_id().map(|s| s.to_string());

        let result = sqlx::query(
            "UPDATE payments
             SET status = $1, provider_id = $2, provider_tx_id = $3, updated_at = now()
             WHERE id = $4 AND status = $5",
        )
        .bind(&status_str)
        .bind(&provider_id_str)
        .bind(&provider_tx_id)
        .bind(payment.id.as_uuid())
        .bind(expected_status)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn persist_settlement(
        &self,
        payment_id: &PaymentId,
        amount_settled: rust_decimal::Decimal,
        settled_currency: cream_models::prelude::Currency,
        failure_reason: Option<&str>,
    ) -> Result<(), ApiError> {
        let currency_str = serde_json::to_value(settled_currency)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize settled_currency: {e}")))?;
        let currency_text = currency_str.as_str().ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "settled_currency serialized to non-string JSON value"
            ))
        })?;

        sqlx::query(
            "UPDATE payments
             SET amount_settled = $1, settled_currency = $2, failure_reason = $3, updated_at = now()
             WHERE id = $4",
        )
        .bind(amount_settled)
        .bind(currency_text)
        .bind(failure_reason)
        .bind(payment_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
