//! # cream-models
//!
//! Pure domain types for the Cream payment control plane.
//! Zero business logic — this crate defines the shared vocabulary that
//! every other crate in the workspace depends on.

// Note: `error` must come before `ids` because `ids` imports DomainError,
// and `payment` must come before `error` because `error` imports PaymentStatus.
// Rust modules can have circular references within the same crate, but we
// declare them in dependency order for clarity.

pub mod agent;
pub mod audit;
pub mod card;
pub mod error;
pub mod ids;
pub mod justification;
pub mod payment;
pub mod policy;
pub mod provider;
pub mod recipient;

/// Convenience re-exports for the most commonly used types.
///
/// Downstream crates can `use cream_models::prelude::*` to get all the
/// core types without individual imports.
pub mod prelude {
    pub use crate::agent::{Agent, AgentProfile, AgentStatus, CountryCode};
    pub use crate::audit::{
        AuditEntry, HumanReviewRecord, PolicyEvaluationRecord, ProviderResponseRecord,
        MAX_REVIEWER_ID_LEN, MAX_REVIEW_REASON_LEN,
    };
    pub use crate::card::{CardControls, CardStatus, CardType, VirtualCard};
    pub use crate::error::DomainError;
    pub use crate::ids::{
        AgentId, AgentProfileId, AuditEntryId, IdempotencyKey, PaymentId, PolicyRuleId,
        VirtualCardId, WebhookEndpointId,
    };
    pub use crate::justification::{Justification, PaymentCategory};
    pub use crate::payment::{
        Currency, Payment, PaymentMetadata, PaymentRequest, PaymentResponse, PaymentStatus,
        RailPreference,
    };
    pub use crate::policy::{
        ComparisonOp, EscalationChannel, EscalationConfig, FieldCheck, PolicyAction,
        PolicyCondition, PolicyRule,
    };
    pub use crate::provider::{
        CircuitState, ProviderHealth, ProviderId, RoutingCandidate, RoutingDecision,
    };
    pub use crate::recipient::{Recipient, RecipientType};
}
