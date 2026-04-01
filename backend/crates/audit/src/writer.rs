use async_trait::async_trait;
use serde::ser::Error as _;
use sqlx::PgPool;
use tracing::instrument;

use cream_models::prelude::{AuditEntry, AuditEntryId, PaymentId};

use crate::error::AuditError;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Append-only write path for the audit ledger.
///
/// The trait intentionally has no `update` or `delete` method — audit records
/// are immutable by design. The database enforces this with triggers, and the
/// Rust type system enforces it here: you simply cannot express a mutation.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuditWriter: Send + Sync {
    /// Persist a single audit entry, optionally linking it to a payment.
    /// Returns the entry's ID on success.
    async fn append(
        &self,
        entry: &AuditEntry,
        payment_id: Option<PaymentId>,
    ) -> Result<AuditEntryId, AuditError>;
}

// ---------------------------------------------------------------------------
// PostgreSQL implementation
// ---------------------------------------------------------------------------

/// Writes audit entries to a PostgreSQL `audit_log` table.
pub struct PgAuditWriter {
    pool: PgPool,
}

impl PgAuditWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditWriter for PgAuditWriter {
    #[instrument(skip(self, entry, payment_id), fields(audit_id = %entry.id, agent_id = %entry.agent_id))]
    async fn append(
        &self,
        entry: &AuditEntry,
        payment_id: Option<PaymentId>,
    ) -> Result<AuditEntryId, AuditError> {
        let routing_decision = entry
            .routing_decision
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let provider_response = entry
            .provider_response
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let human_review = entry
            .human_review
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let final_status = serde_json::to_value(entry.final_status)?;
        let final_status_str = final_status
            .as_str()
            .ok_or_else(|| {
                serde_json::Error::custom(format!(
                    "payment status {:?} serialized to non-string JSON value",
                    entry.final_status
                ))
            })?
            .to_owned();

        sqlx::query(
            "INSERT INTO audit_log (
                id, timestamp, agent_id, agent_profile_id, payment_id,
                request, justification, policy_evaluation,
                routing_decision, provider_response,
                final_status, human_review
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(*entry.id.as_uuid())
        .bind(entry.timestamp)
        .bind(*entry.agent_id.as_uuid())
        .bind(*entry.agent_profile_id.as_uuid())
        .bind(payment_id.map(|id| *id.as_uuid()))
        .bind(&entry.request)
        .bind(&entry.justification)
        .bind(serde_json::to_value(&entry.policy_evaluation)?)
        .bind(routing_decision)
        .bind(provider_response)
        .bind(&final_status_str)
        .bind(human_review)
        .execute(&self.pool)
        .await?;

        tracing::info!("audit entry appended");
        Ok(entry.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cream_models::prelude::*;

    /// Helper to build a minimal valid AuditEntry for testing.
    fn make_test_entry() -> AuditEntry {
        AuditEntry {
            id: AuditEntryId::new(),
            timestamp: Utc::now(),
            agent_id: AgentId::new(),
            agent_profile_id: AgentProfileId::new(),
            request: serde_json::json!({"amount": "50.00", "currency": "usd"}),
            justification: serde_json::json!({
                "summary": "Purchasing API credits for testing purposes",
                "category": "api_credits"
            }),
            policy_evaluation: PolicyEvaluationRecord {
                rules_evaluated: vec![PolicyRuleId::new()],
                matching_rules: vec![],
                final_decision: PolicyAction::Approve,
                decision_latency_ms: 3,
            },
            routing_decision: None,
            provider_response: None,
            final_status: PaymentStatus::Approved,
            human_review: None,
        }
    }

    #[tokio::test]
    async fn mock_writer_append_returns_id() {
        let entry = make_test_entry();
        let expected_id = entry.id;

        let mut mock = MockAuditWriter::new();
        mock.expect_append().returning(move |e, _pid| Ok(e.id));

        let result = mock.append(&entry, None).await.unwrap();
        assert_eq!(result, expected_id);
    }

    #[tokio::test]
    async fn mock_writer_append_with_payment_id() {
        let entry = make_test_entry();
        let payment_id = PaymentId::new();

        let mut mock = MockAuditWriter::new();
        mock.expect_append().returning(move |e, _pid| Ok(e.id));

        let result = mock.append(&entry, Some(payment_id)).await.unwrap();
        assert_eq!(result, entry.id);
    }

    #[tokio::test]
    async fn mock_writer_called_exactly_once() {
        let mut mock = MockAuditWriter::new();
        mock.expect_append().times(1).returning(|e, _pid| Ok(e.id));

        let entry = make_test_entry();
        let _ = mock.append(&entry, None).await;
        // mockall verifies times(1) on drop
    }
}
