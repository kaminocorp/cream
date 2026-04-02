use async_trait::async_trait;
use chrono::Utc;

use cream_models::prelude::{
    CardControls, CardStatus, CircuitState, ProviderHealth, ProviderId, VirtualCard, VirtualCardId,
};

use crate::error::ProviderError;
use crate::traits::PaymentProvider;
use crate::types::{
    CardConfig, NormalizedPaymentRequest, ProviderPaymentResponse, TransactionStatus,
};

/// Configuration for the mock provider's behavior.
#[derive(Debug, Clone)]
pub struct MockProviderConfig {
    /// Whether payment initiations should succeed.
    pub should_succeed: bool,
    /// Simulated latency in milliseconds (applied via tokio::time::sleep).
    pub latency_ms: u64,
    /// The settlement status returned on success.
    pub settlement_status: TransactionStatus,
    /// Whether the provider reports itself as healthy.
    pub is_healthy: bool,
}

impl Default for MockProviderConfig {
    fn default() -> Self {
        Self {
            should_succeed: true,
            latency_ms: 0,
            settlement_status: TransactionStatus::Settled,
            is_healthy: true,
        }
    }
}

/// A configurable mock payment provider for testing.
///
/// Returns predetermined responses based on `MockProviderConfig`. Enables
/// end-to-end pipeline testing without any external service dependencies.
pub struct MockProvider {
    id: ProviderId,
    config: MockProviderConfig,
}

impl MockProvider {
    pub fn new(id: impl Into<String>, config: MockProviderConfig) -> Self {
        Self {
            id: ProviderId::new(id),
            config,
        }
    }

    /// Create a mock that always succeeds with default config.
    pub fn success(id: impl Into<String>) -> Self {
        Self::new(id, MockProviderConfig::default())
    }

    /// Create a mock that always fails.
    pub fn failing(id: impl Into<String>) -> Self {
        Self::new(
            id,
            MockProviderConfig {
                should_succeed: false,
                ..Default::default()
            },
        )
    }
}

#[async_trait]
impl PaymentProvider for MockProvider {
    fn provider_id(&self) -> &ProviderId {
        &self.id
    }

    async fn initiate_payment(
        &self,
        req: &NormalizedPaymentRequest,
    ) -> Result<ProviderPaymentResponse, ProviderError> {
        if self.config.latency_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.config.latency_ms)).await;
        }

        if !self.config.should_succeed {
            return Err(ProviderError::RequestFailed(format!(
                "mock provider {} configured to fail",
                self.id
            )));
        }

        Ok(ProviderPaymentResponse {
            provider_transaction_id: format!("mock_tx_{}", uuid::Uuid::now_v7()),
            status: self.config.settlement_status,
            amount_settled: req.amount,
            currency: req.currency,
            created_at: Utc::now(),
        })
    }

    async fn issue_virtual_card(&self, config: &CardConfig) -> Result<VirtualCard, ProviderError> {
        if !self.config.should_succeed {
            return Err(ProviderError::CardError("mock configured to fail".into()));
        }

        Ok(VirtualCard {
            id: VirtualCardId::new(),
            agent_id: config.agent_id,
            provider: self.id.clone(),
            provider_card_id: format!("mock_card_{}", uuid::Uuid::now_v7()),
            card_type: config.card_type,
            controls: config.controls.clone(),
            status: CardStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: config.expires_at,
        })
    }

    async fn update_card_controls(
        &self,
        _card_id: &str,
        _controls: &CardControls,
    ) -> Result<(), ProviderError> {
        if !self.config.should_succeed {
            return Err(ProviderError::CardError("mock configured to fail".into()));
        }
        Ok(())
    }

    async fn cancel_card(&self, _card_id: &str) -> Result<(), ProviderError> {
        if !self.config.should_succeed {
            return Err(ProviderError::CardError("mock configured to fail".into()));
        }
        Ok(())
    }

    async fn get_transaction_status(
        &self,
        _tx_id: &str,
    ) -> Result<TransactionStatus, ProviderError> {
        if !self.config.should_succeed {
            return Err(ProviderError::RequestFailed(
                "mock configured to fail".into(),
            ));
        }
        Ok(self.config.settlement_status)
    }

    async fn health_check(&self) -> Result<ProviderHealth, ProviderError> {
        Ok(ProviderHealth {
            provider_id: self.id.clone(),
            is_healthy: self.config.is_healthy,
            error_rate_5m: if self.config.is_healthy { 0.0 } else { 1.0 },
            p50_latency_ms: self.config.latency_ms,
            p99_latency_ms: self.config.latency_ms * 2,
            last_checked_at: Utc::now(),
            circuit_state: if self.config.is_healthy {
                CircuitState::Closed
            } else {
                CircuitState::Open
            },
        })
    }
}
