use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{AgentId, IdempotencyKey, PaymentId};
use crate::justification::Justification;
use crate::provider::ProviderId;
use crate::recipient::Recipient;

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

/// Optional metadata the agent can attach to a payment for tracing/correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_ref: Option<String>,
}

// ---------------------------------------------------------------------------
// Payment Request (inbound from agent)
// ---------------------------------------------------------------------------

/// The payload an agent submits to initiate a payment.
///
/// Maps directly to `POST /v1/payments` and the MCP `initiate_payment` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Payment (persisted entity with state machine)
// ---------------------------------------------------------------------------

/// The full payment entity as persisted in the database.
///
/// State transitions are enforced via `transition()` — the only way to
/// change status. The `status` field is private to prevent direct mutation
/// that would bypass the state machine. Use `status()` to read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: PaymentId,
    pub request: PaymentRequest,
    status: PaymentStatus,
    pub provider_id: Option<ProviderId>,
    pub provider_transaction_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Payment {
    /// Returns the current payment status.
    pub fn status(&self) -> PaymentStatus {
        self.status
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
            provider: p.provider_id.clone(),
            provider_transaction_id: p.provider_transaction_id.clone(),
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
}
