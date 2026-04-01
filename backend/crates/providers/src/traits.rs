use async_trait::async_trait;

use cream_models::prelude::{CardControls, ProviderHealth, ProviderId, VirtualCard};

use crate::error::ProviderError;
use crate::types::{
    CardConfig, NormalizedPaymentRequest, ProviderPaymentResponse, TransactionStatus,
};

/// The core provider abstraction — every payment provider implements this trait.
///
/// Direct translation of Vision Section 7.1's `IPaymentProvider`. Adding a new
/// provider requires only implementing this trait and registering it in the
/// `ProviderRegistry`. Zero changes to core business logic.
///
/// All methods are async because provider operations involve network I/O.
#[async_trait]
pub trait PaymentProvider: Send + Sync {
    /// The unique identifier for this provider (e.g., "stripe_issuing").
    fn provider_id(&self) -> &ProviderId;

    /// Initiate a payment through this provider.
    async fn initiate_payment(
        &self,
        req: &NormalizedPaymentRequest,
    ) -> Result<ProviderPaymentResponse, ProviderError>;

    /// Issue a scoped virtual card for an agent.
    async fn issue_virtual_card(&self, config: &CardConfig) -> Result<VirtualCard, ProviderError>;

    /// Update spending controls on an existing card.
    async fn update_card_controls(
        &self,
        card_id: &str,
        controls: &CardControls,
    ) -> Result<(), ProviderError>;

    /// Cancel/revoke a virtual card immediately.
    async fn cancel_card(&self, card_id: &str) -> Result<(), ProviderError>;

    /// Query the status of a transaction at the provider.
    async fn get_transaction_status(&self, tx_id: &str)
        -> Result<TransactionStatus, ProviderError>;

    /// Check the provider's health and connectivity.
    async fn health_check(&self) -> Result<ProviderHealth, ProviderError>;
}
