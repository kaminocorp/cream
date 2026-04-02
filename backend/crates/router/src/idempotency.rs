use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use cream_models::prelude::{IdempotencyKey, PaymentId};

use crate::config::IdempotencyConfig;
use crate::error::RoutingError;

// ---------------------------------------------------------------------------
// Idempotency outcome
// ---------------------------------------------------------------------------

/// The result of attempting to acquire an idempotency lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyOutcome {
    /// Lock acquired — this is the first attempt for this key.
    Acquired,
    /// A payment already exists for this idempotency key.
    Existing(PaymentId),
}

// ---------------------------------------------------------------------------
// Store trait
// ---------------------------------------------------------------------------

/// Persistent store for idempotency locks.
///
/// Two implementations:
/// - `RedisIdempotencyStore` for production
/// - `InMemoryIdempotencyStore` for unit tests
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Try to set the key if it doesn't exist (NX semantics).
    /// Returns `None` if the key was set (acquired), or `Some(existing_value)` if it already existed.
    async fn try_set(
        &self,
        key: &str,
        payment_id: &str,
        ttl_secs: u64,
    ) -> Result<Option<String>, RoutingError>;

    /// Delete the key (release the lock).
    async fn delete(&self, key: &str) -> Result<(), RoutingError>;

    /// Overwrite the key with a new value and refresh the TTL.
    async fn set(&self, key: &str, payment_id: &str, ttl_secs: u64) -> Result<(), RoutingError>;
}

// ---------------------------------------------------------------------------
// Idempotency guard
// ---------------------------------------------------------------------------

/// Prevents double-payments when a request is retried or fails over.
///
/// Uses a distributed lock keyed on `IdempotencyKey`. The lock prevents
/// concurrent processing of the same payment request across server instances.
pub struct IdempotencyGuard {
    store: Box<dyn IdempotencyStore>,
    config: IdempotencyConfig,
}

impl IdempotencyGuard {
    pub fn new(store: Box<dyn IdempotencyStore>, config: IdempotencyConfig) -> Self {
        Self { store, config }
    }

    /// Attempt to acquire the idempotency lock.
    pub async fn acquire(
        &self,
        key: &IdempotencyKey,
        payment_id: &PaymentId,
    ) -> Result<IdempotencyOutcome, RoutingError> {
        let redis_key = format!("cream:idemp:{}", key);
        let payment_str = payment_id.as_uuid().to_string();

        match self
            .store
            .try_set(&redis_key, &payment_str, self.config.lock_ttl_secs)
            .await?
        {
            None => {
                tracing::debug!(key = %key, "idempotency lock acquired");
                Ok(IdempotencyOutcome::Acquired)
            }
            Some(existing) => {
                let existing_id = uuid::Uuid::parse_str(&existing).map_err(|e| {
                    RoutingError::IdempotencyLockFailed(format!("corrupt idempotency value: {e}"))
                })?;
                tracing::debug!(
                    key = %key,
                    existing_payment = %existing,
                    "idempotency conflict: payment already exists"
                );
                Ok(IdempotencyOutcome::Existing(PaymentId::from_uuid(
                    existing_id,
                )))
            }
        }
    }

    /// Release the lock (called when a payment is permanently abandoned).
    pub async fn release(&self, key: &IdempotencyKey) -> Result<(), RoutingError> {
        let redis_key = format!("cream:idemp:{}", key);
        self.store.delete(&redis_key).await?;
        tracing::debug!(key = %key, "idempotency lock released");
        Ok(())
    }

    /// Mark the payment as completed (refreshes TTL for idempotent returns).
    pub async fn complete(
        &self,
        key: &IdempotencyKey,
        payment_id: &PaymentId,
    ) -> Result<(), RoutingError> {
        let redis_key = format!("cream:idemp:{}", key);
        let payment_str = payment_id.as_uuid().to_string();
        self.store
            .set(&redis_key, &payment_str, self.config.lock_ttl_secs)
            .await?;
        tracing::debug!(key = %key, "idempotency entry completed");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// In-memory store (for tests)
// ---------------------------------------------------------------------------

/// In-memory implementation of `IdempotencyStore` for unit tests.
pub struct InMemoryIdempotencyStore {
    data: Mutex<HashMap<String, String>>,
}

impl InMemoryIdempotencyStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryIdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn try_set(
        &self,
        key: &str,
        payment_id: &str,
        _ttl_secs: u64,
    ) -> Result<Option<String>, RoutingError> {
        let mut data = self.data.lock().unwrap();
        if let Some(existing) = data.get(key) {
            Ok(Some(existing.clone()))
        } else {
            data.insert(key.to_owned(), payment_id.to_owned());
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        data.remove(key);
        Ok(())
    }

    async fn set(&self, key: &str, payment_id: &str, _ttl_secs: u64) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        data.insert(key.to_owned(), payment_id.to_owned());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_guard() -> IdempotencyGuard {
        IdempotencyGuard::new(
            Box::new(InMemoryIdempotencyStore::new()),
            IdempotencyConfig { lock_ttl_secs: 300 },
        )
    }

    #[tokio::test]
    async fn first_acquire_succeeds() {
        let guard = make_guard();
        let key = IdempotencyKey::new("idem_001");
        let pid = PaymentId::new();
        let result = guard.acquire(&key, &pid).await.unwrap();
        assert_eq!(result, IdempotencyOutcome::Acquired);
    }

    #[tokio::test]
    async fn second_acquire_returns_existing() {
        let guard = make_guard();
        let key = IdempotencyKey::new("idem_002");
        let pid1 = PaymentId::new();
        let pid2 = PaymentId::new();

        guard.acquire(&key, &pid1).await.unwrap();
        let result = guard.acquire(&key, &pid2).await.unwrap();
        match result {
            IdempotencyOutcome::Existing(existing) => {
                assert_eq!(existing, pid1);
            }
            IdempotencyOutcome::Acquired => panic!("expected Existing"),
        }
    }

    #[tokio::test]
    async fn release_then_reacquire() {
        let guard = make_guard();
        let key = IdempotencyKey::new("idem_003");
        let pid1 = PaymentId::new();
        let pid2 = PaymentId::new();

        guard.acquire(&key, &pid1).await.unwrap();
        guard.release(&key).await.unwrap();

        let result = guard.acquire(&key, &pid2).await.unwrap();
        assert_eq!(result, IdempotencyOutcome::Acquired);
    }

    #[tokio::test]
    async fn complete_persists_payment() {
        let guard = make_guard();
        let key = IdempotencyKey::new("idem_004");
        let pid = PaymentId::new();

        guard.acquire(&key, &pid).await.unwrap();
        guard.complete(&key, &pid).await.unwrap();

        // Subsequent acquire returns existing
        let pid2 = PaymentId::new();
        let result = guard.acquire(&key, &pid2).await.unwrap();
        assert_eq!(result, IdempotencyOutcome::Existing(pid));
    }

    #[tokio::test]
    async fn different_keys_do_not_conflict() {
        let guard = make_guard();
        let key1 = IdempotencyKey::new("idem_a");
        let key2 = IdempotencyKey::new("idem_b");
        let pid1 = PaymentId::new();
        let pid2 = PaymentId::new();

        let r1 = guard.acquire(&key1, &pid1).await.unwrap();
        let r2 = guard.acquire(&key2, &pid2).await.unwrap();

        assert_eq!(r1, IdempotencyOutcome::Acquired);
        assert_eq!(r2, IdempotencyOutcome::Acquired);
    }
}
