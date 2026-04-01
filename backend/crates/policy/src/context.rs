use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use cream_models::prelude::{
    Agent, AgentProfile, Currency, PaymentRequest, PaymentStatus, RailPreference,
};

/// The complete data bag passed to every rule during evaluation.
///
/// Pre-loaded by the API crate before calling the policy engine. This keeps
/// the policy crate free of database dependencies — it's purely computational.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// The payment request being evaluated.
    pub request: PaymentRequest,
    /// The agent initiating the payment.
    pub agent: Agent,
    /// The agent's policy profile (spending authority and constraints).
    pub profile: AgentProfile,
    /// Recent payments by this agent (for velocity/spend rate checks).
    pub recent_payments: Vec<PaymentSummary>,
    /// Merchant identifiers this agent has transacted with before.
    pub known_merchants: HashSet<String>,
    /// The current time (injectable for testing).
    pub current_time: DateTime<Utc>,
}

/// A lightweight summary of a recent payment, used for velocity and spend rate
/// checks without loading the full payment entity.
#[derive(Debug, Clone)]
pub struct PaymentSummary {
    pub amount: Decimal,
    pub currency: Currency,
    pub recipient_identifier: String,
    pub status: PaymentStatus,
    pub rail: RailPreference,
    pub created_at: DateTime<Utc>,
}
