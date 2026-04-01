use std::collections::HashMap;
use std::sync::Arc;

use cream_models::prelude::ProviderId;

use crate::traits::PaymentProvider;

/// A factory-pattern registry of payment providers.
///
/// The routing engine and API layer query this registry to find providers.
/// Each provider is stored behind `Arc` for concurrent access across
/// Axum request handlers.
pub struct ProviderRegistry {
    providers: HashMap<ProviderId, Arc<dyn PaymentProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider. Overwrites any existing provider with the same ID.
    pub fn register(&mut self, provider: Arc<dyn PaymentProvider>) {
        let id = provider.provider_id().clone();
        self.providers.insert(id, provider);
    }

    /// Look up a provider by ID.
    pub fn get(&self, id: &ProviderId) -> Option<Arc<dyn PaymentProvider>> {
        self.providers.get(id).cloned()
    }

    /// Return all registered providers.
    pub fn all(&self) -> Vec<Arc<dyn PaymentProvider>> {
        self.providers.values().cloned().collect()
    }

    /// Return the number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Return true if no providers are registered.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Return all provider IDs.
    pub fn provider_ids(&self) -> Vec<ProviderId> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
