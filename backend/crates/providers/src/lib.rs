//! # cream-providers
//!
//! Payment provider abstraction layer for the Cream payment control plane.
//!
//! Defines the [`PaymentProvider`] trait that every provider (Stripe, Airwallex,
//! Coinbase, etc.) implements, plus a [`ProviderRegistry`] (factory pattern) for
//! runtime provider lookup. Adding a new provider = implement the trait + register it.
//!
//! Includes a [`MockProvider`] for end-to-end pipeline testing without external services.

pub mod error;
pub mod mock_provider;
pub mod registry;
pub mod traits;
pub mod types;

// Convenience re-exports
pub use error::ProviderError;
pub use mock_provider::{MockProvider, MockProviderConfig};
pub use registry::ProviderRegistry;
pub use traits::PaymentProvider;
pub use types::{CardConfig, NormalizedPaymentRequest, ProviderPaymentResponse, TransactionStatus};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cream_models::prelude::*;
    use rust_decimal::Decimal;

    use super::*;

    // -----------------------------------------------------------------------
    // Registry tests
    // -----------------------------------------------------------------------

    #[test]
    fn registry_starts_empty() {
        let reg = ProviderRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_register_and_lookup() {
        let mut reg = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::success("stripe_issuing"));
        reg.register(provider);

        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let found = reg.get(&ProviderId::new("stripe_issuing"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().provider_id().as_str(), "stripe_issuing");
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        let reg = ProviderRegistry::new();
        assert!(reg.get(&ProviderId::new("nonexistent")).is_none());
    }

    #[test]
    fn registry_overwrites_duplicate() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider::success("stripe")));
        reg.register(Arc::new(MockProvider::failing("stripe")));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_all_returns_all_providers() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider::success("stripe")));
        reg.register(Arc::new(MockProvider::success("airwallex")));
        reg.register(Arc::new(MockProvider::success("coinbase")));
        assert_eq!(reg.all().len(), 3);
    }

    #[test]
    fn registry_provider_ids() {
        let mut reg = ProviderRegistry::new();
        reg.register(Arc::new(MockProvider::success("stripe")));
        reg.register(Arc::new(MockProvider::success("airwallex")));
        let ids = reg.provider_ids();
        assert_eq!(ids.len(), 2);
    }

    // -----------------------------------------------------------------------
    // MockProvider tests
    // -----------------------------------------------------------------------

    fn test_payment_request() -> NormalizedPaymentRequest {
        NormalizedPaymentRequest {
            payment_id: PaymentId::new(),
            amount: Decimal::new(14999, 2), // 149.99
            currency: Currency::SGD,
            recipient_identifier: "stripe_merch_123".to_string(),
            recipient_country: Some("SG".to_string()),
            rail: RailPreference::Card,
            description: "Test payment".to_string(),
            idempotency_key: "idem_test_001".to_string(),
        }
    }

    fn test_card_config() -> CardConfig {
        CardConfig {
            agent_id: AgentId::new(),
            card_type: CardType::SingleUse,
            controls: CardControls {
                max_per_transaction: Some(Decimal::new(500, 0)),
                max_per_cycle: None,
                allowed_mcc_codes: vec!["5734".to_string()], // Computer software stores
                currency: Currency::SGD,
            },
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn mock_provider_succeeds() {
        let provider = MockProvider::success("test_provider");
        let req = test_payment_request();
        let result = provider.initiate_payment(&req).await;
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert_eq!(resp.status, TransactionStatus::Settled);
        assert_eq!(resp.amount_settled, req.amount);
        assert!(resp.provider_transaction_id.starts_with("mock_tx_"));
    }

    #[tokio::test]
    async fn mock_provider_fails_when_configured() {
        let provider = MockProvider::failing("test_provider");
        let req = test_payment_request();
        let result = provider.initiate_payment(&req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_provider_issues_card() {
        let provider = MockProvider::success("test_issuer");
        let config = test_card_config();
        let result = provider.issue_virtual_card(&config).await;
        assert!(result.is_ok());

        let card = result.unwrap();
        assert_eq!(card.provider.as_str(), "test_issuer");
        assert_eq!(card.card_type, CardType::SingleUse);
        assert_eq!(card.status, CardStatus::Active);
    }

    #[tokio::test]
    async fn mock_provider_card_fails_when_configured() {
        let provider = MockProvider::failing("test_issuer");
        let config = test_card_config();
        assert!(provider.issue_virtual_card(&config).await.is_err());
        assert!(provider.cancel_card("card_123").await.is_err());
        assert!(provider
            .update_card_controls("card_123", &config.controls)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn mock_provider_health_check_healthy() {
        let provider = MockProvider::success("test_provider");
        let health = provider.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.circuit_state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn mock_provider_health_check_unhealthy() {
        let provider = MockProvider::new(
            "sick_provider",
            MockProviderConfig {
                is_healthy: false,
                ..Default::default()
            },
        );
        let health = provider.health_check().await.unwrap();
        assert!(!health.is_healthy);
        assert_eq!(health.circuit_state, CircuitState::Open);
    }

    #[tokio::test]
    async fn mock_provider_transaction_status() {
        let provider = MockProvider::success("test_provider");
        let status = provider.get_transaction_status("tx_123").await.unwrap();
        assert_eq!(status, TransactionStatus::Settled);
    }

    #[tokio::test]
    async fn mock_provider_custom_settlement_status() {
        let provider = MockProvider::new(
            "pending_provider",
            MockProviderConfig {
                settlement_status: TransactionStatus::Pending,
                ..Default::default()
            },
        );
        let req = test_payment_request();
        let resp = provider.initiate_payment(&req).await.unwrap();
        assert_eq!(resp.status, TransactionStatus::Pending);
    }

    #[test]
    fn provider_id_accessible() {
        let provider = MockProvider::success("my_provider");
        assert_eq!(provider.provider_id().as_str(), "my_provider");
    }

    // -----------------------------------------------------------------------
    // ProviderError retryability tests (v0.6.7)
    // -----------------------------------------------------------------------

    #[test]
    fn transient_errors_are_retryable() {
        assert!(ProviderError::RequestFailed("oops".into()).is_retryable());
        assert!(ProviderError::UnexpectedResponse("bad".into()).is_retryable());
        assert!(ProviderError::Timeout(5000).is_retryable());
        assert!(ProviderError::Unavailable("down".into()).is_retryable());
        assert!(ProviderError::RateLimited {
            retry_after_ms: 1000
        }
        .is_retryable());
    }

    #[test]
    fn permanent_errors_are_not_retryable() {
        assert!(!ProviderError::NotFound("x".into()).is_retryable());
        assert!(!ProviderError::CardError("x".into()).is_retryable());
        assert!(!ProviderError::AuthenticationFailed("x".into()).is_retryable());
        assert!(!ProviderError::InvalidAmount("x".into()).is_retryable());
        assert!(!ProviderError::DuplicatePayment.is_retryable());
        assert!(!ProviderError::InsufficientFunds("x".into()).is_retryable());
        assert!(!ProviderError::ComplianceBlocked("x".into()).is_retryable());
        assert!(!ProviderError::UnsupportedCurrency("x".into()).is_retryable());
        assert!(!ProviderError::UnsupportedCountry("x".into()).is_retryable());
    }
}
