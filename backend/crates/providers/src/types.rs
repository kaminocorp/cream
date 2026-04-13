use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use cream_models::prelude::{AgentId, CardControls, CardType, Currency, PaymentId, RailPreference};

// ---------------------------------------------------------------------------
// Normalized Payment Request — provider-agnostic input
// ---------------------------------------------------------------------------

/// A provider-agnostic payment request, translated from the domain
/// `PaymentRequest` by the API layer before dispatching to a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPaymentRequest {
    pub payment_id: PaymentId,
    pub amount: Decimal,
    pub currency: Currency,
    pub recipient_identifier: String,
    pub recipient_country: Option<String>,
    pub rail: RailPreference,
    pub description: String,
    pub idempotency_key: String,
}

// ---------------------------------------------------------------------------
// Provider Payment Response
// ---------------------------------------------------------------------------

/// The normalized response from a provider after initiating a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPaymentResponse {
    /// The provider's own transaction/payment ID.
    pub provider_transaction_id: String,
    pub status: TransactionStatus,
    pub amount_settled: Decimal,
    pub currency: Currency,
    /// When the provider reports the transaction was created.
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Transaction Status
// ---------------------------------------------------------------------------

/// Normalized transaction status across all providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    /// Payment is processing.
    Pending,
    /// Payment has been successfully settled.
    Settled,
    /// Payment failed.
    Failed,
    /// Payment was declined by the provider.
    Declined,
    /// Payment is awaiting further action (e.g., 3DS).
    RequiresAction,
    /// Payment was refunded.
    Refunded,
}

// ---------------------------------------------------------------------------
// Card Config — input for issuing virtual cards
// ---------------------------------------------------------------------------

/// Configuration for issuing a virtual card to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardConfig {
    pub agent_id: AgentId,
    pub card_type: CardType,
    pub controls: CardControls,
    /// Optional expiry. If `None`, the card does not auto-expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

