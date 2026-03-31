use std::fmt;

use thiserror::Error;

use crate::ids::IdempotencyKey;
use crate::payment::PaymentStatus;

/// Domain-level errors that can occur across the cream platform.
///
/// These are *domain* errors — they represent business rule violations, not
/// infrastructure failures. Infrastructure errors (database timeouts, network
/// issues) are handled separately at the crate boundary.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid state transition: cannot move from {from} to {to}")]
    InvalidStateTransition {
        from: PaymentStatus,
        to: PaymentStatus,
    },

    #[error("invalid ID format: {0}")]
    InvalidIdFormat(String),

    #[error("justification too short: minimum {min_words} words required, got {actual}")]
    JustificationTooShort { min_words: usize, actual: usize },

    #[error("policy violation: {0}")]
    PolicyViolation(String),

    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("idempotency conflict: {0}")]
    IdempotencyConflict(IdempotencyKey),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,
}

// IdempotencyKey Display already has the "idem_" prefix, which is fine for
// error messages. But we also want the ProviderUnavailable variant to accept
// a ProviderId by string, so we keep it as String to avoid circular deps.

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Validating => "validating",
            PaymentStatus::PendingApproval => "pending_approval",
            PaymentStatus::Approved => "approved",
            PaymentStatus::Submitted => "submitted",
            PaymentStatus::Settled => "settled",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Blocked => "blocked",
            PaymentStatus::Rejected => "rejected",
            PaymentStatus::TimedOut => "timed_out",
        };
        write!(f, "{}", label)
    }
}
