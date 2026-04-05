use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, AgentProfileId, AuditEntryId, PolicyRuleId};
use crate::payment::{Currency, PaymentStatus};
use crate::policy::PolicyAction;
use crate::provider::{ProviderId, RoutingDecision};

// ---------------------------------------------------------------------------
// Audit Entry
// ---------------------------------------------------------------------------

/// Maximum allowed length for `AuditEntry.on_chain_tx_hash`.
/// On-chain transaction hashes are external input persisted to the immutable
/// audit ledger — must be bounded to prevent permanent bloat.
/// Ethereum/Base hashes are 66 chars (`0x` + 64 hex); 256 provides headroom
/// for future chain formats.
pub const MAX_ON_CHAIN_TX_HASH_LEN: usize = 256;

/// An immutable record of a complete payment lifecycle event.
///
/// Every payment produces exactly one audit entry. The entry captures the
/// full decision trace: what was requested, why, what the policy engine
/// decided, how routing chose a provider, and what happened. These records
/// are append-only — the database physically prevents updates or deletes.
///
/// Custom `Deserialize` validates `on_chain_tx_hash` length bounds.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub id: AuditEntryId,
    pub timestamp: DateTime<Utc>,
    pub agent_id: AgentId,
    pub agent_profile_id: AgentProfileId,
    /// The payment this audit entry is linked to (written to the DB but not part
    /// of the entry's own identity — populated by the reader from the join column).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<crate::ids::PaymentId>,
    /// The full normalized payment request, stored as JSON for schema flexibility.
    pub request: serde_json::Value,
    /// The verbatim agent-provided justification, stored as JSON.
    pub justification: serde_json::Value,
    /// Record of the policy engine's evaluation.
    pub policy_evaluation: PolicyEvaluationRecord,
    /// Routing decision (absent if payment was blocked before routing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_decision: Option<RoutingDecision>,
    /// Provider execution result (absent if payment never reached a provider).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_response: Option<ProviderResponseRecord>,
    /// The terminal status of the payment.
    pub final_status: PaymentStatus,
    /// Human review record (absent if no escalation occurred).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_review: Option<HumanReviewRecord>,
    /// On-chain transaction hash for crypto-rail payments.
    /// Serves as an independently verifiable cryptographic receipt.
    /// Bounded to [`MAX_ON_CHAIN_TX_HASH_LEN`] to prevent audit ledger bloat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_chain_tx_hash: Option<String>,
}

impl<'de> Deserialize<'de> for AuditEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            id: AuditEntryId,
            timestamp: DateTime<Utc>,
            agent_id: AgentId,
            agent_profile_id: AgentProfileId,
            payment_id: Option<crate::ids::PaymentId>,
            request: serde_json::Value,
            justification: serde_json::Value,
            policy_evaluation: PolicyEvaluationRecord,
            routing_decision: Option<RoutingDecision>,
            provider_response: Option<ProviderResponseRecord>,
            final_status: PaymentStatus,
            human_review: Option<HumanReviewRecord>,
            on_chain_tx_hash: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if let Some(ref hash) = raw.on_chain_tx_hash {
            if hash.trim().is_empty() {
                return Err(serde::de::Error::custom(
                    "on_chain_tx_hash must not be empty or whitespace-only when provided — \
                     use None instead of an empty string",
                ));
            }
            if hash.len() > MAX_ON_CHAIN_TX_HASH_LEN {
                return Err(serde::de::Error::custom(format!(
                    "on_chain_tx_hash exceeds maximum length of {} characters (got {})",
                    MAX_ON_CHAIN_TX_HASH_LEN,
                    hash.len()
                )));
            }
        }

        Ok(AuditEntry {
            id: raw.id,
            timestamp: raw.timestamp,
            agent_id: raw.agent_id,
            agent_profile_id: raw.agent_profile_id,
            payment_id: raw.payment_id,
            request: raw.request,
            justification: raw.justification,
            policy_evaluation: raw.policy_evaluation,
            routing_decision: raw.routing_decision,
            provider_response: raw.provider_response,
            final_status: raw.final_status,
            human_review: raw.human_review,
            on_chain_tx_hash: raw.on_chain_tx_hash,
        })
    }
}

// ---------------------------------------------------------------------------
// Policy Evaluation Record
// ---------------------------------------------------------------------------

/// Captures the policy engine's evaluation for a single payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationRecord {
    /// All rules that were evaluated (by ID).
    pub rules_evaluated: Vec<PolicyRuleId>,
    /// Rules whose conditions matched (subset of rules_evaluated).
    pub matching_rules: Vec<PolicyRuleId>,
    /// The final verdict: Approve, Block, or Escalate.
    pub final_decision: PolicyAction,
    /// How long the policy evaluation took, in milliseconds.
    pub decision_latency_ms: u64,
}

// ---------------------------------------------------------------------------
// Provider Response Record
// ---------------------------------------------------------------------------

/// Maximum allowed length for `ProviderResponseRecord.transaction_id`.
/// Provider transaction IDs are external input persisted to the immutable
/// audit ledger — must be bounded to prevent permanent bloat.
pub const MAX_PROVIDER_TRANSACTION_ID_LEN: usize = 500;

/// Maximum allowed length for `ProviderResponseRecord.status`.
pub const MAX_PROVIDER_STATUS_LEN: usize = 255;

/// Captures the result of dispatching a payment to a provider.
///
/// Custom `Deserialize` enforces length bounds on string fields sourced from
/// external provider APIs. These are written to the append-only audit ledger,
/// so unbounded values would persist forever.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponseRecord {
    pub provider: ProviderId,
    pub transaction_id: String,
    /// Provider-reported status string (varies by provider).
    pub status: String,
    pub amount_settled: Decimal,
    pub currency: Currency,
    /// How long the provider call took, in milliseconds.
    pub latency_ms: u64,
}

impl<'de> Deserialize<'de> for ProviderResponseRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            provider: ProviderId,
            transaction_id: String,
            status: String,
            amount_settled: Decimal,
            currency: Currency,
            latency_ms: u64,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.transaction_id.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "transaction_id must not be empty or whitespace-only",
            ));
        }
        if raw.transaction_id.len() > MAX_PROVIDER_TRANSACTION_ID_LEN {
            return Err(serde::de::Error::custom(format!(
                "transaction_id exceeds maximum length of {} characters (got {})",
                MAX_PROVIDER_TRANSACTION_ID_LEN,
                raw.transaction_id.len()
            )));
        }
        if raw.status.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "status must not be empty or whitespace-only",
            ));
        }
        if raw.status.len() > MAX_PROVIDER_STATUS_LEN {
            return Err(serde::de::Error::custom(format!(
                "status exceeds maximum length of {} characters (got {})",
                MAX_PROVIDER_STATUS_LEN,
                raw.status.len()
            )));
        }
        if raw.amount_settled <= Decimal::ZERO {
            return Err(serde::de::Error::custom(format!(
                "amount_settled must be positive, got {}",
                raw.amount_settled
            )));
        }

        Ok(ProviderResponseRecord {
            provider: raw.provider,
            transaction_id: raw.transaction_id,
            status: raw.status,
            amount_settled: raw.amount_settled,
            currency: raw.currency,
            latency_ms: raw.latency_ms,
        })
    }
}

// ---------------------------------------------------------------------------
// Human Review Record
// ---------------------------------------------------------------------------

/// Maximum allowed length for `HumanReviewRecord.reviewer_id`.
pub const MAX_REVIEWER_ID_LEN: usize = 255;

/// Maximum allowed length for `HumanReviewRecord.reason`.
pub const MAX_REVIEW_REASON_LEN: usize = 2000;

/// Captures the result of a human-in-the-loop review.
///
/// Custom `Deserialize` enforces length bounds on string fields to prevent
/// audit log bloat (the audit ledger is append-only).
#[derive(Debug, Clone, Serialize)]
pub struct HumanReviewRecord {
    /// Identifier of the human reviewer (email or user ID).
    pub reviewer_id: String,
    /// The human's decision.
    pub decision: PolicyAction,
    /// Optional explanation for the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When the decision was made.
    pub decided_at: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for HumanReviewRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            reviewer_id: String,
            decision: PolicyAction,
            reason: Option<String>,
            decided_at: DateTime<Utc>,
        }

        let raw = Raw::deserialize(deserializer)?;

        // A human review decision must be Approve or Block — never Escalate.
        // Escalating an already-escalated payment would create an infinite
        // escalation loop, matching the invariant enforced on
        // EscalationConfig::on_timeout (see policy.rs).
        if raw.decision == PolicyAction::Escalate {
            return Err(serde::de::Error::custom(
                "human review decision must not be ESCALATE — \
                 re-escalating an already-escalated payment would create an infinite loop",
            ));
        }

        if raw.reviewer_id.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "reviewer_id must not be empty — audit trail requires reviewer identity",
            ));
        }
        if raw.reviewer_id.len() > MAX_REVIEWER_ID_LEN {
            return Err(serde::de::Error::custom(format!(
                "reviewer_id exceeds maximum length of {} characters (got {})",
                MAX_REVIEWER_ID_LEN,
                raw.reviewer_id.len()
            )));
        }
        if let Some(ref reason) = raw.reason {
            if reason.trim().is_empty() {
                return Err(serde::de::Error::custom(
                    "reason must not be empty or whitespace-only when provided — \
                     use None instead of an empty string",
                ));
            }
            if reason.len() > MAX_REVIEW_REASON_LEN {
                return Err(serde::de::Error::custom(format!(
                    "reason exceeds maximum length of {} characters (got {})",
                    MAX_REVIEW_REASON_LEN,
                    reason.len()
                )));
            }
        }

        Ok(HumanReviewRecord {
            reviewer_id: raw.reviewer_id,
            decision: raw.decision,
            reason: raw.reason,
            decided_at: raw.decided_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_evaluation_record_serde() {
        let record = PolicyEvaluationRecord {
            rules_evaluated: vec![PolicyRuleId::new(), PolicyRuleId::new()],
            matching_rules: vec![],
            final_decision: PolicyAction::Approve,
            decision_latency_ms: 12,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["final_decision"], "APPROVE");
        assert_eq!(json["decision_latency_ms"], 12);
    }

    // -----------------------------------------------------------------------
    // Phase 6.14: ProviderResponseRecord bounds
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 6.15: HumanReviewRecord rejects Escalate decision
    // -----------------------------------------------------------------------

    #[test]
    fn human_review_rejects_escalate_decision() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "ESCALATE",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let result: Result<HumanReviewRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ESCALATE"));
        assert!(err.contains("infinite loop"));
    }

    #[test]
    fn human_review_accepts_approve_decision() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "APPROVE",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let record: HumanReviewRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.decision, PolicyAction::Approve);
    }

    #[test]
    fn human_review_accepts_block_decision() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "BLOCK",
            "reason": "Suspicious transaction",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let record: HumanReviewRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.decision, PolicyAction::Block);
        assert_eq!(record.reason.unwrap(), "Suspicious transaction");
    }

    // -----------------------------------------------------------------------
    // Phase 6.14: ProviderResponseRecord bounds
    // -----------------------------------------------------------------------

    #[test]
    fn provider_response_record_within_limits() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123abc",
            "status": "succeeded",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let record: ProviderResponseRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.transaction_id, "ch_123abc");
        assert_eq!(record.status, "succeeded");
    }

    #[test]
    fn provider_response_record_rejects_oversized_transaction_id() {
        let long_id = "x".repeat(MAX_PROVIDER_TRANSACTION_ID_LEN + 1);
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": long_id,
            "status": "succeeded",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("transaction_id"));
    }

    #[test]
    fn provider_response_record_rejects_oversized_status() {
        let long_status = "x".repeat(MAX_PROVIDER_STATUS_LEN + 1);
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123",
            "status": long_status,
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("status"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.1: HumanReviewRecord empty reviewer_id guard
    // -----------------------------------------------------------------------

    #[test]
    fn human_review_rejects_empty_reviewer_id() {
        let json = serde_json::json!({
            "reviewer_id": "",
            "decision": "APPROVE",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let result: Result<HumanReviewRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reviewer_id"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn human_review_rejects_whitespace_only_reviewer_id() {
        let json = serde_json::json!({
            "reviewer_id": "   ",
            "decision": "APPROVE",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let result: Result<HumanReviewRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reviewer_id"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.6: HumanReviewRecord reason empty/whitespace guard
    // -----------------------------------------------------------------------

    #[test]
    fn human_review_rejects_empty_reason() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "APPROVE",
            "reason": "",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let result: Result<HumanReviewRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reason"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn human_review_rejects_whitespace_only_reason() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "BLOCK",
            "reason": "   ",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let result: Result<HumanReviewRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("reason"));
    }

    #[test]
    fn human_review_accepts_none_reason() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "APPROVE",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let record: HumanReviewRecord = serde_json::from_value(json).unwrap();
        assert!(record.reason.is_none());
    }

    #[test]
    fn human_review_accepts_valid_reason() {
        let json = serde_json::json!({
            "reviewer_id": "admin@example.com",
            "decision": "APPROVE",
            "reason": "Verified with vendor directly",
            "decided_at": "2026-04-01T12:00:00Z"
        });
        let record: HumanReviewRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.reason.unwrap(), "Verified with vendor directly");
    }

    // -----------------------------------------------------------------------
    // Phase 7.2: ProviderResponseRecord empty/whitespace guards
    // -----------------------------------------------------------------------

    #[test]
    fn provider_response_record_rejects_empty_transaction_id() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "",
            "status": "succeeded",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("transaction_id"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn provider_response_record_rejects_whitespace_transaction_id() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "   ",
            "status": "succeeded",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("transaction_id"));
    }

    #[test]
    fn provider_response_record_rejects_empty_status() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123abc",
            "status": "",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("status"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn provider_response_record_rejects_whitespace_status() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123abc",
            "status": "   ",
            "amount_settled": "149.99",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("status"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.3: ProviderResponseRecord amount_settled positive validation
    // -----------------------------------------------------------------------

    #[test]
    fn provider_response_record_rejects_zero_amount_settled() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123abc",
            "status": "succeeded",
            "amount_settled": "0.00",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("amount_settled"));
        assert!(err.contains("positive"));
    }

    #[test]
    fn provider_response_record_rejects_negative_amount_settled() {
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": "ch_123abc",
            "status": "succeeded",
            "amount_settled": "-5.00",
            "currency": "SGD",
            "latency_ms": 187
        });
        let result: Result<ProviderResponseRecord, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("amount_settled"));
        assert!(err.contains("positive"));
    }

    #[test]
    fn provider_response_record_at_exact_limit() {
        let exact_id = "t".repeat(MAX_PROVIDER_TRANSACTION_ID_LEN);
        let exact_status = "s".repeat(MAX_PROVIDER_STATUS_LEN);
        let json = serde_json::json!({
            "provider": "stripe_issuing",
            "transaction_id": exact_id,
            "status": exact_status,
            "amount_settled": "100.00",
            "currency": "USD",
            "latency_ms": 100
        });
        let record: ProviderResponseRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.transaction_id.len(), MAX_PROVIDER_TRANSACTION_ID_LEN);
        assert_eq!(record.status.len(), MAX_PROVIDER_STATUS_LEN);
    }

    // -----------------------------------------------------------------------
    // Phase 7.9: AuditEntry on_chain_tx_hash bounds
    // -----------------------------------------------------------------------

    /// Helper: build a minimal valid AuditEntry JSON for deserialization tests.
    fn sample_audit_entry_json(on_chain_tx_hash: Option<&str>) -> serde_json::Value {
        let mut entry = serde_json::json!({
            "id": AuditEntryId::new(),
            "timestamp": "2026-04-05T12:00:00Z",
            "agent_id": AgentId::new(),
            "agent_profile_id": AgentProfileId::new(),
            "request": {},
            "justification": {},
            "policy_evaluation": {
                "rules_evaluated": [],
                "matching_rules": [],
                "final_decision": "APPROVE",
                "decision_latency_ms": 5
            },
            "final_status": "settled"
        });
        if let Some(hash) = on_chain_tx_hash {
            entry["on_chain_tx_hash"] = serde_json::json!(hash);
        }
        entry
    }

    #[test]
    fn audit_entry_accepts_valid_on_chain_tx_hash() {
        let hash = "0x".to_string() + &"a".repeat(64); // standard Ethereum hash
        let json = sample_audit_entry_json(Some(&hash));
        let entry: AuditEntry = serde_json::from_value(json).unwrap();
        assert_eq!(entry.on_chain_tx_hash.unwrap(), hash);
    }

    #[test]
    fn audit_entry_accepts_none_on_chain_tx_hash() {
        let json = sample_audit_entry_json(None);
        let entry: AuditEntry = serde_json::from_value(json).unwrap();
        assert!(entry.on_chain_tx_hash.is_none());
    }

    #[test]
    fn audit_entry_rejects_empty_on_chain_tx_hash() {
        let json = sample_audit_entry_json(Some(""));
        let result: Result<AuditEntry, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("on_chain_tx_hash"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn audit_entry_rejects_whitespace_only_on_chain_tx_hash() {
        let json = sample_audit_entry_json(Some("   "));
        let result: Result<AuditEntry, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("on_chain_tx_hash"));
    }

    #[test]
    fn audit_entry_rejects_oversized_on_chain_tx_hash() {
        let long_hash = "x".repeat(MAX_ON_CHAIN_TX_HASH_LEN + 1);
        let json = sample_audit_entry_json(Some(&long_hash));
        let result: Result<AuditEntry, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("on_chain_tx_hash"));
        assert!(err.contains("maximum length"));
    }

    #[test]
    fn audit_entry_accepts_on_chain_tx_hash_at_limit() {
        let exact_hash = "h".repeat(MAX_ON_CHAIN_TX_HASH_LEN);
        let json = sample_audit_entry_json(Some(&exact_hash));
        let entry: AuditEntry = serde_json::from_value(json).unwrap();
        assert_eq!(
            entry.on_chain_tx_hash.unwrap().len(),
            MAX_ON_CHAIN_TX_HASH_LEN
        );
    }
}
