use thiserror::Error;

/// Errors that can occur during provider operations.
///
/// Variants are split into *transient* (retryable) and *permanent* (not retryable)
/// categories. The [`is_retryable`](ProviderError::is_retryable) method exposes
/// this classification for use by circuit breakers and failover logic.
#[derive(Debug, Error)]
pub enum ProviderError {
    // -- Transient (retryable) errors ------------------------------------------
    #[error("provider request failed: {0}")]
    RequestFailed(String),

    #[error("provider returned unexpected response: {0}")]
    UnexpectedResponse(String),

    #[error("provider timeout after {0}ms")]
    Timeout(u64),

    #[error("provider unavailable: {0}")]
    Unavailable(String),

    #[error("rate limited by provider, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    // -- Permanent (non-retryable) errors --------------------------------------
    #[error("provider not found: {0}")]
    NotFound(String),

    #[error("card operation failed: {0}")]
    CardError(String),

    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("invalid payment amount: {0}")]
    InvalidAmount(String),

    #[error("duplicate payment detected")]
    DuplicatePayment,

    #[error("insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("blocked by compliance: {0}")]
    ComplianceBlocked(String),

    #[error("unsupported currency: {0}")]
    UnsupportedCurrency(String),

    #[error("unsupported country: {0}")]
    UnsupportedCountry(String),
}

impl ProviderError {
    /// Returns `true` if this error is transient and the operation should be
    /// retried (possibly on a different provider). Used by the circuit breaker
    /// and failover logic in the routing engine.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RequestFailed(_)
                | Self::UnexpectedResponse(_)
                | Self::Timeout(_)
                | Self::Unavailable(_)
                | Self::RateLimited { .. }
        )
    }
}
