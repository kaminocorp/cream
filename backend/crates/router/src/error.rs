use cream_models::prelude::IdempotencyKey;
use cream_providers::ProviderError;
use thiserror::Error;

/// Errors that can occur during routing and provider selection.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// No provider passed the viability filters (circuit breaker, currency,
    /// rail policy). The payment cannot be dispatched.
    #[error("no viable provider found for this payment")]
    NoViableProvider,

    /// Providers were available but all failed during execution.
    #[error("all providers exhausted after failover attempts")]
    AllProvidersExhausted,

    /// A payment already exists for this idempotency key.
    #[error("idempotency conflict: payment already exists for key {0}")]
    IdempotencyConflict(IdempotencyKey),

    /// Redis lock acquisition failed (Redis unavailable or network error).
    #[error("idempotency lock failed: {0}")]
    IdempotencyLockFailed(String),

    /// Redis connectivity or command error.
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Error from a payment provider during execution.
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Invalid configuration (weights, thresholds, etc.).
    #[error("configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_viable_provider_display() {
        let err = RoutingError::NoViableProvider;
        assert_eq!(err.to_string(), "no viable provider found for this payment");
    }

    #[test]
    fn idempotency_conflict_display() {
        let key = IdempotencyKey::new("idem_test_123");
        let err = RoutingError::IdempotencyConflict(key);
        assert!(err.to_string().contains("idem_test_123"));
    }
}
