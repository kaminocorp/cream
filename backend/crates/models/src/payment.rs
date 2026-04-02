use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::audit::MAX_PROVIDER_TRANSACTION_ID_LEN;
use crate::error::DomainError;
use crate::ids::{AgentId, IdempotencyKey, PaymentId};
use crate::justification::Justification;
use crate::provider::ProviderId;
use crate::recipient::Recipient;

/// Maximum allowed length for `PaymentCategory::Other` values.
/// Prevents audit log bloat from unbounded category strings.
pub const MAX_CATEGORY_OTHER_LEN: usize = 500;

// ---------------------------------------------------------------------------
// Payment Status — the state machine
// ---------------------------------------------------------------------------

/// Every payment moves through a deterministic state machine.
///
/// Valid transitions:
/// ```text
/// Pending → Validating → Approved → Submitted → Settled
///                      → PendingApproval → Approved
///                                        → Rejected
///                                        → TimedOut → Blocked
///                      → Blocked
/// Submitted → Failed
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    /// Just created, awaiting validation.
    Pending,
    /// Schema + justification validated, entering policy evaluation.
    Validating,
    /// Policy engine returned ESCALATE; awaiting human decision.
    PendingApproval,
    /// Policy approved (auto or human); entering routing.
    Approved,
    /// Dispatched to provider; awaiting settlement.
    Submitted,
    /// Provider confirmed settlement.
    Settled,
    /// Payment failed at provider level.
    Failed,
    /// Blocked by policy engine.
    Blocked,
    /// Rejected by human reviewer.
    Rejected,
    /// Timed out waiting for human approval.
    TimedOut,
}

impl PaymentStatus {
    /// Returns `true` if transitioning from `self` to `next` is a valid
    /// state machine move.
    pub fn can_transition_to(&self, next: PaymentStatus) -> bool {
        matches!(
            (self, next),
            (PaymentStatus::Pending, PaymentStatus::Validating)
                | (PaymentStatus::Validating, PaymentStatus::Approved)
                | (PaymentStatus::Validating, PaymentStatus::PendingApproval)
                | (PaymentStatus::Validating, PaymentStatus::Blocked)
                | (PaymentStatus::PendingApproval, PaymentStatus::Approved)
                | (PaymentStatus::PendingApproval, PaymentStatus::Rejected)
                | (PaymentStatus::PendingApproval, PaymentStatus::TimedOut)
                | (PaymentStatus::TimedOut, PaymentStatus::Blocked)
                | (PaymentStatus::Approved, PaymentStatus::Submitted)
                | (PaymentStatus::Submitted, PaymentStatus::Settled)
                | (PaymentStatus::Submitted, PaymentStatus::Failed)
        )
    }

    /// Returns `true` if this is a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PaymentStatus::Settled
                | PaymentStatus::Failed
                | PaymentStatus::Blocked
                | PaymentStatus::Rejected
                | PaymentStatus::TimedOut
        )
    }

    /// Returns `true` if this payment counts toward spend/velocity limits.
    ///
    /// Includes both settled payments and in-flight payments (any state that
    /// represents real or intended money movement). Excludes only states where
    /// the payment was definitively cancelled: Failed, Blocked, Rejected.
    /// TimedOut is excluded because it always transitions to Blocked.
    pub fn counts_toward_spend(&self) -> bool {
        !matches!(
            self,
            PaymentStatus::Failed
                | PaymentStatus::Blocked
                | PaymentStatus::Rejected
                | PaymentStatus::TimedOut
        )
    }
}

// ---------------------------------------------------------------------------
// Currency — ISO 4217 fiat + major crypto
// ---------------------------------------------------------------------------

/// ISO 4217 fiat currency codes plus major cryptocurrency tickers.
///
/// Covers ~95% of global fiat transaction volume and the major stablecoins
/// / crypto assets relevant to agent payments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum Currency {
    // Major fiat
    USD,
    EUR,
    GBP,
    SGD,
    JPY,
    CNY,
    HKD,
    AUD,
    CAD,
    INR,
    KRW,
    TWD,
    THB,
    MYR,
    IDR,
    PHP,
    VND,
    BRL,
    MXN,
    CHF,
    SEK,
    NOK,
    DKK,
    NZD,
    AED,
    // Major crypto
    BTC,
    ETH,
    USDC,
    USDT,
    SOL,
    MATIC,
    AVAX,
    #[serde(rename = "BASE_ETH")]
    BaseEth,
}

impl Currency {
    /// Returns `true` if this is a cryptocurrency (not a fiat currency).
    pub fn is_crypto(&self) -> bool {
        matches!(
            self,
            Currency::BTC
                | Currency::ETH
                | Currency::USDC
                | Currency::USDT
                | Currency::SOL
                | Currency::MATIC
                | Currency::AVAX
                | Currency::BaseEth
        )
    }

    /// Returns `true` if this is a fiat currency.
    pub fn is_fiat(&self) -> bool {
        !self.is_crypto()
    }
}

// ---------------------------------------------------------------------------
// Rail Preference
// ---------------------------------------------------------------------------

/// The agent's preferred payment rail. `Auto` lets the routing engine decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailPreference {
    Auto,
    Card,
    Ach,
    Swift,
    Local,
    Stablecoin,
}

// ---------------------------------------------------------------------------
// Payment Metadata
// ---------------------------------------------------------------------------

/// Maximum allowed length for any single metadata field value.
/// Prevents audit log bloat from unbounded metadata strings.
pub const MAX_METADATA_FIELD_LEN: usize = 500;

/// Optional metadata the agent can attach to a payment for tracing/correlation.
///
/// All string fields are bounded to [`MAX_METADATA_FIELD_LEN`] characters on
/// deserialization to prevent audit log bloat from malicious or runaway values.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_ref: Option<String>,
}

impl<'de> Deserialize<'de> for PaymentMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            agent_session_id: Option<String>,
            workflow_id: Option<String>,
            operator_ref: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;

        fn validate_field<E: serde::de::Error>(
            field: &Option<String>,
            name: &str,
        ) -> Result<(), E> {
            if let Some(s) = field {
                if s.trim().is_empty() {
                    return Err(E::custom(format!(
                        "metadata.{name} must not be empty or whitespace-only when provided — \
                         use None instead of an empty string"
                    )));
                }
                if s.len() > MAX_METADATA_FIELD_LEN {
                    return Err(E::custom(format!(
                        "metadata.{name} exceeds maximum length of {MAX_METADATA_FIELD_LEN} characters (got {})",
                        s.len()
                    )));
                }
            }
            Ok(())
        }

        validate_field::<D::Error>(&raw.agent_session_id, "agent_session_id")?;
        validate_field::<D::Error>(&raw.workflow_id, "workflow_id")?;
        validate_field::<D::Error>(&raw.operator_ref, "operator_ref")?;

        Ok(PaymentMetadata {
            agent_session_id: raw.agent_session_id,
            workflow_id: raw.workflow_id,
            operator_ref: raw.operator_ref,
        })
    }
}

// ---------------------------------------------------------------------------
// Payment Request (inbound from agent)
// ---------------------------------------------------------------------------

/// The payload an agent submits to initiate a payment.
///
/// Maps directly to `POST /v1/payments` and the MCP `initiate_payment` tool.
///
/// Custom `Deserialize` validates that `amount` is strictly positive — zero
/// or negative amounts are nonsensical and would bypass the policy engine.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentRequest {
    pub agent_id: AgentId,
    pub amount: Decimal,
    pub currency: Currency,
    pub recipient: Recipient,
    pub preferred_rail: RailPreference,
    pub justification: Justification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PaymentMetadata>,
    pub idempotency_key: IdempotencyKey,
}

impl<'de> Deserialize<'de> for PaymentRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            agent_id: AgentId,
            amount: Decimal,
            currency: Currency,
            recipient: Recipient,
            preferred_rail: RailPreference,
            justification: Justification,
            metadata: Option<PaymentMetadata>,
            idempotency_key: IdempotencyKey,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.amount <= Decimal::ZERO {
            return Err(serde::de::Error::custom(format!(
                "amount must be positive, got {}",
                raw.amount
            )));
        }

        Ok(PaymentRequest {
            agent_id: raw.agent_id,
            amount: raw.amount,
            currency: raw.currency,
            recipient: raw.recipient,
            preferred_rail: raw.preferred_rail,
            justification: raw.justification,
            metadata: raw.metadata,
            idempotency_key: raw.idempotency_key,
        })
    }
}

// ---------------------------------------------------------------------------
// Payment (persisted entity with state machine)
// ---------------------------------------------------------------------------

/// The full payment entity as persisted in the database.
///
/// State transitions are enforced via `transition()` — the only way to
/// change status. The `status` field is private to prevent direct mutation
/// that would bypass the state machine. Use `status()` to read.
///
/// Custom `Deserialize` validates invariants on load:
/// - `created_at` must be <= `updated_at`
/// - `provider_id` and `provider_transaction_id` must not be set for
///   pre-submission statuses (Pending, Validating, PendingApproval)
///
/// `provider_id` and `provider_transaction_id` are private to enforce the
/// invariant that provider fields are only set for post-submission statuses.
/// Use `set_provider()` to set them — it validates the current status.
#[derive(Debug, Clone, Serialize)]
pub struct Payment {
    pub id: PaymentId,
    pub request: PaymentRequest,
    status: PaymentStatus,
    provider_id: Option<ProviderId>,
    provider_transaction_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Shadow struct used only for deserialization with validation.
#[derive(Deserialize)]
struct PaymentRaw {
    id: PaymentId,
    request: PaymentRequest,
    status: PaymentStatus,
    provider_id: Option<ProviderId>,
    provider_transaction_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for Payment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = PaymentRaw::deserialize(deserializer)?;

        // Invariant: created_at must not be after updated_at
        if raw.created_at > raw.updated_at {
            return Err(serde::de::Error::custom("created_at must be <= updated_at"));
        }

        // Invariant: provider fields should not be set before submission
        let pre_submission = matches!(
            raw.status,
            PaymentStatus::Pending | PaymentStatus::Validating | PaymentStatus::PendingApproval
        );
        if pre_submission && (raw.provider_id.is_some() || raw.provider_transaction_id.is_some()) {
            return Err(serde::de::Error::custom(format!(
                "provider_id and provider_transaction_id must not be set for status {:?}",
                raw.status
            )));
        }

        Ok(Payment {
            id: raw.id,
            request: raw.request,
            status: raw.status,
            provider_id: raw.provider_id,
            provider_transaction_id: raw.provider_transaction_id,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

impl Payment {
    /// Returns the current payment status.
    pub fn status(&self) -> PaymentStatus {
        self.status
    }

    /// Returns the provider ID, if set (only after submission).
    pub fn provider_id(&self) -> Option<&ProviderId> {
        self.provider_id.as_ref()
    }

    /// Returns the provider's transaction ID, if set (only after submission).
    pub fn provider_transaction_id(&self) -> Option<&str> {
        self.provider_transaction_id.as_deref()
    }

    /// Set the provider and provider transaction ID (write-once).
    ///
    /// Only valid when the payment is in `Approved` or `Submitted` status
    /// (i.e., at or past the point where a provider has been selected but
    /// before the payment reaches a terminal state). Returns an error for
    /// pre-submission statuses (not yet routed) and terminal statuses
    /// (Settled, Failed — immutable once reached).
    ///
    /// This is a write-once operation — calling it again after provider info
    /// is already set returns an error. During failover, the payment should
    /// transition through the state machine (creating a new audit trail entry)
    /// rather than silently overwriting the provider.
    pub fn set_provider(
        &mut self,
        provider_id: ProviderId,
        transaction_id: String,
    ) -> Result<(), DomainError> {
        if self.provider_id.is_some() {
            return Err(DomainError::PolicyViolation(
                "provider already set on this payment; create a new audit entry for failover"
                    .to_string(),
            ));
        }
        let valid = matches!(
            self.status,
            PaymentStatus::Approved | PaymentStatus::Submitted
        );
        if !valid {
            return Err(DomainError::PolicyViolation(format!(
                "cannot set provider on payment in status {}",
                self.status
            )));
        }
        if transaction_id.trim().is_empty() {
            return Err(DomainError::PolicyViolation(
                "provider transaction_id must not be empty".to_string(),
            ));
        }
        if transaction_id.len() > MAX_PROVIDER_TRANSACTION_ID_LEN {
            return Err(DomainError::PolicyViolation(format!(
                "provider transaction_id exceeds maximum length of {} characters (got {})",
                MAX_PROVIDER_TRANSACTION_ID_LEN,
                transaction_id.len()
            )));
        }
        self.provider_id = Some(provider_id);
        self.provider_transaction_id = Some(transaction_id);
        Ok(())
    }

    /// Create a new payment from a request. Starts in `Pending` status.
    pub fn new(request: PaymentRequest) -> Self {
        let now = Utc::now();
        Self {
            id: PaymentId::new(),
            request,
            status: PaymentStatus::Pending,
            provider_id: None,
            provider_transaction_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Attempt a state transition. Returns an error if the transition is invalid.
    pub fn transition(&mut self, next: PaymentStatus) -> Result<(), DomainError> {
        if self.status.can_transition_to(next) {
            self.status = next;
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(DomainError::InvalidStateTransition {
                from: self.status,
                to: next,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Payment Response (outbound to agent)
// ---------------------------------------------------------------------------

/// The response returned to the agent after a payment operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub payment_id: PaymentId,
    pub status: PaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_transaction_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Payment> for PaymentResponse {
    fn from(p: &Payment) -> Self {
        Self {
            payment_id: p.id,
            status: p.status(),
            provider: p.provider_id().cloned(),
            provider_transaction_id: p.provider_transaction_id().map(|s| s.to_owned()),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::CountryCode;
    use crate::justification::PaymentCategory;
    use crate::recipient::RecipientType;

    fn sample_request() -> PaymentRequest {
        PaymentRequest {
            agent_id: AgentId::new(),
            amount: Decimal::new(14999, 2), // 149.99
            currency: Currency::SGD,
            recipient: Recipient {
                recipient_type: RecipientType::Merchant,
                identifier: "stripe_merch_123".to_string(),
                name: None,
                country: Some(CountryCode::new("SG")),
            },
            preferred_rail: RailPreference::Auto,
            justification: Justification {
                summary: "Purchasing two API credit packs for batch processing job number 4421"
                    .to_string(),
                task_id: Some("task_8372".to_string()),
                category: PaymentCategory::ApiCredits,
                expected_value: None,
            },
            metadata: None,
            idempotency_key: IdempotencyKey::new("test-key-001"),
        }
    }

    #[test]
    fn valid_happy_path_transitions() {
        let mut p = Payment::new(sample_request());
        assert_eq!(p.status, PaymentStatus::Pending);

        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.transition(PaymentStatus::Settled).unwrap();

        assert!(p.status.is_terminal());
    }

    #[test]
    fn valid_escalation_path() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::PendingApproval).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.transition(PaymentStatus::Settled).unwrap();
    }

    #[test]
    fn valid_rejection_path() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::PendingApproval).unwrap();
        p.transition(PaymentStatus::Rejected).unwrap();
        assert!(p.status.is_terminal());
    }

    #[test]
    fn valid_timeout_path() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::PendingApproval).unwrap();
        p.transition(PaymentStatus::TimedOut).unwrap();
        p.transition(PaymentStatus::Blocked).unwrap();
        assert!(p.status.is_terminal());
    }

    #[test]
    fn invalid_transition_rejected() {
        let mut p = Payment::new(sample_request());
        let result = p.transition(PaymentStatus::Settled);
        assert!(result.is_err());
    }

    #[test]
    fn cannot_transition_from_terminal() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.transition(PaymentStatus::Settled).unwrap();

        let result = p.transition(PaymentStatus::Failed);
        assert!(result.is_err());
    }

    #[test]
    fn counts_toward_spend_includes_settled_and_inflight() {
        // Settled and in-flight statuses should count
        assert!(PaymentStatus::Pending.counts_toward_spend());
        assert!(PaymentStatus::Validating.counts_toward_spend());
        assert!(PaymentStatus::PendingApproval.counts_toward_spend());
        assert!(PaymentStatus::Approved.counts_toward_spend());
        assert!(PaymentStatus::Submitted.counts_toward_spend());
        assert!(PaymentStatus::Settled.counts_toward_spend());

        // Failed/cancelled statuses should NOT count
        assert!(!PaymentStatus::Failed.counts_toward_spend());
        assert!(!PaymentStatus::Blocked.counts_toward_spend());
        assert!(!PaymentStatus::Rejected.counts_toward_spend());
        assert!(!PaymentStatus::TimedOut.counts_toward_spend());
    }

    #[test]
    fn timed_out_is_terminal() {
        // TimedOut can only transition to Blocked — it is effectively terminal
        // and should be reported as such by is_terminal().
        assert!(PaymentStatus::TimedOut.is_terminal());
    }

    #[test]
    fn all_terminal_states_are_terminal() {
        assert!(PaymentStatus::Settled.is_terminal());
        assert!(PaymentStatus::Failed.is_terminal());
        assert!(PaymentStatus::Blocked.is_terminal());
        assert!(PaymentStatus::Rejected.is_terminal());
        assert!(PaymentStatus::TimedOut.is_terminal());

        // Non-terminal states
        assert!(!PaymentStatus::Pending.is_terminal());
        assert!(!PaymentStatus::Validating.is_terminal());
        assert!(!PaymentStatus::PendingApproval.is_terminal());
        assert!(!PaymentStatus::Approved.is_terminal());
        assert!(!PaymentStatus::Submitted.is_terminal());
    }

    #[test]
    fn currency_classification() {
        assert!(Currency::USDC.is_crypto());
        assert!(Currency::SGD.is_fiat());
        assert!(!Currency::BTC.is_fiat());
    }

    #[test]
    fn payment_status_serde() {
        let s = PaymentStatus::PendingApproval;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"pending_approval\"");
    }

    // -----------------------------------------------------------------------
    // Payment serde validation tests (v0.6.7)
    // -----------------------------------------------------------------------

    #[test]
    fn payment_serde_roundtrip_happy_path() {
        let p = Payment::new(sample_request());
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Payment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status(), PaymentStatus::Pending);
        assert_eq!(parsed.id, p.id);
    }

    #[test]
    fn payment_deserialize_rejects_created_after_updated() {
        let p = Payment::new(sample_request());
        let mut val = serde_json::to_value(&p).unwrap();
        // Set created_at far in the future, after updated_at
        val["created_at"] = serde_json::json!("2099-01-01T00:00:00Z");
        val["updated_at"] = serde_json::json!("2020-01-01T00:00:00Z");
        let result: Result<Payment, _> = serde_json::from_value(val);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("created_at must be <= updated_at"));
    }

    #[test]
    fn payment_deserialize_rejects_provider_id_on_pending() {
        let p = Payment::new(sample_request());
        let mut val = serde_json::to_value(&p).unwrap();
        val["provider_id"] = serde_json::json!("prov_stripe");
        let result: Result<Payment, _> = serde_json::from_value(val);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("provider_id"));
    }

    #[test]
    fn payment_deserialize_allows_provider_id_on_submitted() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.set_provider(ProviderId::new("stripe"), "ch_123".to_string())
            .unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Payment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status(), PaymentStatus::Submitted);
        assert!(parsed.provider_id().is_some());
    }

    #[test]
    fn set_provider_rejects_pre_submission_status() {
        let mut p = Payment::new(sample_request());
        // Pending — should reject
        let result = p.set_provider(ProviderId::new("stripe"), "ch_123".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn set_provider_accepts_approved_status() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        let result = p.set_provider(ProviderId::new("stripe"), "ch_123".to_string());
        assert!(result.is_ok());
        assert_eq!(p.provider_id().unwrap().as_str(), "stripe");
        assert_eq!(p.provider_transaction_id().unwrap(), "ch_123");
    }

    // -----------------------------------------------------------------------
    // Phase 6.9 tests
    // -----------------------------------------------------------------------

    #[test]
    fn set_provider_rejects_second_call() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.set_provider(ProviderId::new("stripe"), "ch_123".to_string())
            .unwrap();
        // Second call should fail — write-once semantics
        let result = p.set_provider(ProviderId::new("airwallex"), "aw_456".to_string());
        assert!(result.is_err());
        // Original provider should be unchanged
        assert_eq!(p.provider_id().unwrap().as_str(), "stripe");
    }

    #[test]
    fn set_provider_rejects_settled_status() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.transition(PaymentStatus::Settled).unwrap();
        // Terminal status — should not allow provider mutation
        let result = p.set_provider(ProviderId::new("stripe"), "ch_123".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn set_provider_rejects_failed_status() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        p.transition(PaymentStatus::Submitted).unwrap();
        p.transition(PaymentStatus::Failed).unwrap();
        let result = p.set_provider(ProviderId::new("stripe"), "ch_123".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn metadata_within_limits_deserializes() {
        let meta = PaymentMetadata {
            agent_session_id: Some("sess_123".to_string()),
            workflow_id: Some("wf_abc".to_string()),
            operator_ref: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: PaymentMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workflow_id.unwrap(), "wf_abc");
    }

    #[test]
    fn metadata_exceeding_limit_rejected() {
        let long = "x".repeat(MAX_METADATA_FIELD_LEN + 1);
        let json = serde_json::json!({
            "agent_session_id": long,
        });
        let result: Result<PaymentMetadata, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("maximum length"));
    }

    #[test]
    fn metadata_at_exact_limit_accepted() {
        let exact = "y".repeat(MAX_METADATA_FIELD_LEN);
        let json = serde_json::json!({
            "workflow_id": exact,
        });
        let parsed: PaymentMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.workflow_id.unwrap().len(), MAX_METADATA_FIELD_LEN);
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: PaymentRequest amount validation
    // -----------------------------------------------------------------------

    #[test]
    fn payment_request_positive_amount_accepted() {
        let req = sample_request();
        let json = serde_json::to_string(&req).unwrap();
        let parsed: PaymentRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.amount, req.amount);
    }

    #[test]
    fn payment_request_zero_amount_rejected() {
        let mut req = sample_request();
        req.amount = Decimal::ZERO;
        let json = serde_json::to_string(&req).unwrap();
        let result: Result<PaymentRequest, _> = serde_json::from_str(&json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("amount must be positive"));
    }

    #[test]
    fn payment_request_negative_amount_rejected() {
        let mut req = sample_request();
        req.amount = Decimal::new(-100, 0);
        let json = serde_json::to_string(&req).unwrap();
        let result: Result<PaymentRequest, _> = serde_json::from_str(&json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("amount must be positive"));
    }

    // -----------------------------------------------------------------------
    // Phase 6.14: set_provider transaction_id bounds
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 7.1: set_provider empty transaction_id guard
    // -----------------------------------------------------------------------

    #[test]
    fn set_provider_rejects_empty_transaction_id() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        let result = p.set_provider(ProviderId::new("stripe"), String::new());
        assert!(result.is_err());
        // Provider should NOT have been set
        assert!(p.provider_id().is_none());
    }

    #[test]
    fn set_provider_rejects_whitespace_only_transaction_id() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        let result = p.set_provider(ProviderId::new("stripe"), "   ".to_string());
        assert!(result.is_err());
        assert!(p.provider_id().is_none());
    }

    #[test]
    fn set_provider_rejects_oversized_transaction_id() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        let long_id = "x".repeat(MAX_PROVIDER_TRANSACTION_ID_LEN + 1);
        let result = p.set_provider(ProviderId::new("stripe"), long_id);
        assert!(result.is_err());
        // Provider should NOT have been set
        assert!(p.provider_id().is_none());
    }

    #[test]
    fn set_provider_accepts_transaction_id_at_limit() {
        let mut p = Payment::new(sample_request());
        p.transition(PaymentStatus::Validating).unwrap();
        p.transition(PaymentStatus::Approved).unwrap();
        let exact_id = "t".repeat(MAX_PROVIDER_TRANSACTION_ID_LEN);
        let result = p.set_provider(ProviderId::new("stripe"), exact_id);
        assert!(result.is_ok());
        assert_eq!(
            p.provider_transaction_id().unwrap().len(),
            MAX_PROVIDER_TRANSACTION_ID_LEN
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7.6: PaymentMetadata empty/whitespace guards
    // -----------------------------------------------------------------------

    #[test]
    fn metadata_rejects_empty_agent_session_id() {
        let json = serde_json::json!({
            "agent_session_id": ""
        });
        let result: Result<PaymentMetadata, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("agent_session_id"));
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn metadata_rejects_whitespace_only_workflow_id() {
        let json = serde_json::json!({
            "workflow_id": "   "
        });
        let result: Result<PaymentMetadata, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("workflow_id"));
    }

    #[test]
    fn metadata_rejects_empty_operator_ref() {
        let json = serde_json::json!({
            "operator_ref": ""
        });
        let result: Result<PaymentMetadata, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("operator_ref"));
    }

    #[test]
    fn metadata_accepts_none_fields() {
        let json = serde_json::json!({});
        let meta: PaymentMetadata = serde_json::from_value(json).unwrap();
        assert!(meta.agent_session_id.is_none());
        assert!(meta.workflow_id.is_none());
        assert!(meta.operator_ref.is_none());
    }

    #[test]
    fn metadata_accepts_valid_fields() {
        let json = serde_json::json!({
            "agent_session_id": "sess_123",
            "workflow_id": "wf_456",
            "operator_ref": "ref_789"
        });
        let meta: PaymentMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(meta.agent_session_id.unwrap(), "sess_123");
        assert_eq!(meta.workflow_id.unwrap(), "wf_456");
        assert_eq!(meta.operator_ref.unwrap(), "ref_789");
    }
}
