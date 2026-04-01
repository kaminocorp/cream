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

/// An immutable record of a complete payment lifecycle event.
///
/// Every payment produces exactly one audit entry. The entry captures the
/// full decision trace: what was requested, why, what the policy engine
/// decided, how routing chose a provider, and what happened. These records
/// are append-only — the database physically prevents updates or deletes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: AuditEntryId,
    pub timestamp: DateTime<Utc>,
    pub agent_id: AgentId,
    pub agent_profile_id: AgentProfileId,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_chain_tx_hash: Option<String>,
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

/// Captures the result of dispatching a payment to a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Human Review Record
// ---------------------------------------------------------------------------

/// Captures the result of a human-in-the-loop review.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
