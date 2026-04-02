use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;

use cream_models::prelude::{CircuitState, ProviderId};

use crate::config::CircuitBreakerConfig;
use crate::error::RoutingError;

// ---------------------------------------------------------------------------
// Store trait — abstracts Redis for testability
// ---------------------------------------------------------------------------

/// Persistent store for circuit breaker state.
///
/// Two implementations:
/// - `RedisCircuitBreakerStore` for production (Redis-backed, shared across instances)
/// - `InMemoryCircuitBreakerStore` for unit tests (no Redis required)
#[async_trait]
pub trait CircuitBreakerStore: Send + Sync {
    /// Record a success or failure event for a provider.
    async fn record_event(
        &self,
        provider_id: &ProviderId,
        success: bool,
    ) -> Result<(), RoutingError>;

    /// Get the error rate within the rolling window.
    async fn get_error_rate(
        &self,
        provider_id: &ProviderId,
        window_secs: u64,
    ) -> Result<f64, RoutingError>;

    /// Get the current circuit state.
    async fn get_state(&self, provider_id: &ProviderId) -> Result<CircuitState, RoutingError>;

    /// Set the circuit state.
    async fn set_state(
        &self,
        provider_id: &ProviderId,
        state: CircuitState,
    ) -> Result<(), RoutingError>;

    /// Get the timestamp (Unix secs) when the breaker was opened.
    async fn get_opened_at(&self, provider_id: &ProviderId) -> Result<Option<i64>, RoutingError>;

    /// Set the opened_at timestamp.
    async fn set_opened_at(
        &self,
        provider_id: &ProviderId,
        timestamp: i64,
    ) -> Result<(), RoutingError>;

    /// Get the number of requests allowed through in HalfOpen.
    async fn get_half_open_count(&self, provider_id: &ProviderId) -> Result<u32, RoutingError>;

    /// Increment and return the half-open request count.
    async fn increment_half_open_count(
        &self,
        provider_id: &ProviderId,
    ) -> Result<u32, RoutingError>;

    /// Reset the half-open request counter (called when transitioning into HalfOpen).
    async fn reset_half_open_count(&self, provider_id: &ProviderId) -> Result<(), RoutingError>;

    /// Reset all circuit breaker state for a provider to Closed.
    async fn reset(&self, provider_id: &ProviderId) -> Result<(), RoutingError>;
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

/// Per-provider circuit breaker.
///
/// Implements the standard Closed → Open → HalfOpen pattern:
/// - **Closed**: healthy, accepting traffic
/// - **Open**: error rate exceeded threshold, all traffic rejected
/// - **HalfOpen**: cooldown expired, testing with limited traffic
pub struct CircuitBreaker {
    store: Box<dyn CircuitBreakerStore>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(store: Box<dyn CircuitBreakerStore>, config: CircuitBreakerConfig) -> Self {
        Self { store, config }
    }

    /// Record a successful provider call.
    pub async fn record_success(&self, provider_id: &ProviderId) -> Result<(), RoutingError> {
        self.store.record_event(provider_id, true).await?;

        let state = self.store.get_state(provider_id).await?;
        if state == CircuitState::HalfOpen {
            let count = self.store.get_half_open_count(provider_id).await?;
            // +1 because the current success hasn't been counted in half_open_count
            // (half_open_count tracks requests allowed through, not successes)
            if count >= self.config.half_open_max_requests {
                tracing::info!(
                    provider = %provider_id,
                    "circuit breaker closing: enough successful requests in half-open"
                );
                self.store.reset(provider_id).await?;
            }
        }

        Ok(())
    }

    /// Record a failed provider call.
    pub async fn record_failure(&self, provider_id: &ProviderId) -> Result<(), RoutingError> {
        self.store.record_event(provider_id, false).await?;

        let state = self.store.get_state(provider_id).await?;

        match state {
            CircuitState::Closed => {
                let error_rate = self
                    .store
                    .get_error_rate(provider_id, self.config.window_secs)
                    .await?;
                if error_rate >= self.config.error_threshold {
                    tracing::warn!(
                        provider = %provider_id,
                        error_rate,
                        threshold = self.config.error_threshold,
                        "circuit breaker tripped: error rate exceeded threshold"
                    );
                    self.store
                        .set_state(provider_id, CircuitState::Open)
                        .await?;
                    self.store
                        .set_opened_at(provider_id, Utc::now().timestamp())
                        .await?;
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!(
                    provider = %provider_id,
                    "circuit breaker re-opening: failure during half-open"
                );
                self.store
                    .set_state(provider_id, CircuitState::Open)
                    .await?;
                self.store
                    .set_opened_at(provider_id, Utc::now().timestamp())
                    .await?;
            }
            CircuitState::Open => {
                // Already open — nothing to do
            }
        }

        Ok(())
    }

    /// Get the current circuit state for a provider.
    pub async fn state(&self, provider_id: &ProviderId) -> Result<CircuitState, RoutingError> {
        self.store.get_state(provider_id).await
    }

    /// Check if a provider is accepting traffic.
    ///
    /// Returns `true` for Closed state. For Open, checks if cooldown has
    /// expired and transitions to HalfOpen if so. For HalfOpen, checks
    /// if the request count is within the limit.
    pub async fn is_allowed(&self, provider_id: &ProviderId) -> Result<bool, RoutingError> {
        let state = self.store.get_state(provider_id).await?;

        match state {
            CircuitState::Closed => Ok(true),
            CircuitState::Open => {
                let opened_at = self.store.get_opened_at(provider_id).await?;
                let now = Utc::now().timestamp();
                if let Some(opened) = opened_at {
                    if (now - opened) >= self.config.cooldown_secs as i64 {
                        tracing::info!(
                            provider = %provider_id,
                            "circuit breaker transitioning to half-open after cooldown"
                        );
                        self.store
                            .set_state(provider_id, CircuitState::HalfOpen)
                            .await?;
                        // Reset half-open counter for the new testing phase
                        self.store.reset_half_open_count(provider_id).await?;
                        self.store.increment_half_open_count(provider_id).await?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            CircuitState::HalfOpen => {
                let count = self.store.get_half_open_count(provider_id).await?;
                if count < self.config.half_open_max_requests {
                    self.store.increment_half_open_count(provider_id).await?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Manually reset a circuit breaker to Closed (admin operation).
    pub async fn reset(&self, provider_id: &ProviderId) -> Result<(), RoutingError> {
        tracing::info!(provider = %provider_id, "circuit breaker manually reset to closed");
        self.store.reset(provider_id).await
    }
}

// ---------------------------------------------------------------------------
// In-memory store (for tests)
// ---------------------------------------------------------------------------

/// Event record for the in-memory store.
struct Event {
    timestamp: i64,
    success: bool,
}

/// Per-provider state in the in-memory store.
#[derive(Default)]
struct ProviderState {
    events: Vec<Event>,
    state: Option<CircuitState>,
    opened_at: Option<i64>,
    half_open_count: u32,
}

/// In-memory implementation of `CircuitBreakerStore` for unit tests.
pub struct InMemoryCircuitBreakerStore {
    data: Mutex<HashMap<String, ProviderState>>,
}

impl InMemoryCircuitBreakerStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCircuitBreakerStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CircuitBreakerStore for InMemoryCircuitBreakerStore {
    async fn record_event(
        &self,
        provider_id: &ProviderId,
        success: bool,
    ) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        let state = data.entry(provider_id.as_str().to_owned()).or_default();
        state.events.push(Event {
            timestamp: Utc::now().timestamp(),
            success,
        });
        Ok(())
    }

    async fn get_error_rate(
        &self,
        provider_id: &ProviderId,
        window_secs: u64,
    ) -> Result<f64, RoutingError> {
        let data = self.data.lock().unwrap();
        let state = match data.get(provider_id.as_str()) {
            Some(s) => s,
            None => return Ok(0.0),
        };
        let cutoff = Utc::now().timestamp() - window_secs as i64;
        let recent: Vec<&Event> = state
            .events
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .collect();
        if recent.is_empty() {
            return Ok(0.0);
        }
        let failures = recent.iter().filter(|e| !e.success).count();
        Ok(failures as f64 / recent.len() as f64)
    }

    async fn get_state(&self, provider_id: &ProviderId) -> Result<CircuitState, RoutingError> {
        let data = self.data.lock().unwrap();
        Ok(data
            .get(provider_id.as_str())
            .and_then(|s| s.state)
            .unwrap_or(CircuitState::Closed))
    }

    async fn set_state(
        &self,
        provider_id: &ProviderId,
        state: CircuitState,
    ) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        data.entry(provider_id.as_str().to_owned())
            .or_default()
            .state = Some(state);
        Ok(())
    }

    async fn get_opened_at(&self, provider_id: &ProviderId) -> Result<Option<i64>, RoutingError> {
        let data = self.data.lock().unwrap();
        Ok(data.get(provider_id.as_str()).and_then(|s| s.opened_at))
    }

    async fn set_opened_at(
        &self,
        provider_id: &ProviderId,
        timestamp: i64,
    ) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        data.entry(provider_id.as_str().to_owned())
            .or_default()
            .opened_at = Some(timestamp);
        Ok(())
    }

    async fn get_half_open_count(&self, provider_id: &ProviderId) -> Result<u32, RoutingError> {
        let data = self.data.lock().unwrap();
        Ok(data
            .get(provider_id.as_str())
            .map(|s| s.half_open_count)
            .unwrap_or(0))
    }

    async fn increment_half_open_count(
        &self,
        provider_id: &ProviderId,
    ) -> Result<u32, RoutingError> {
        let mut data = self.data.lock().unwrap();
        let state = data.entry(provider_id.as_str().to_owned()).or_default();
        state.half_open_count += 1;
        Ok(state.half_open_count)
    }

    async fn reset_half_open_count(&self, provider_id: &ProviderId) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        let state = data.entry(provider_id.as_str().to_owned()).or_default();
        state.half_open_count = 0;
        Ok(())
    }

    async fn reset(&self, provider_id: &ProviderId) -> Result<(), RoutingError> {
        let mut data = self.data.lock().unwrap();
        let state = data.entry(provider_id.as_str().to_owned()).or_default();
        state.state = Some(CircuitState::Closed);
        state.opened_at = None;
        state.half_open_count = 0;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_breaker() -> (CircuitBreaker, ProviderId) {
        let store = InMemoryCircuitBreakerStore::new();
        let config = CircuitBreakerConfig {
            error_threshold: 0.5,
            window_secs: 300,
            cooldown_secs: 2, // Short cooldown for testing
            half_open_max_requests: 3,
        };
        let provider = ProviderId::new("test_provider");
        (CircuitBreaker::new(Box::new(store), config), provider)
    }

    #[tokio::test]
    async fn closed_after_success() {
        let (breaker, pid) = make_breaker();
        breaker.record_success(&pid).await.unwrap();
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn closed_is_allowed() {
        let (breaker, pid) = make_breaker();
        assert!(breaker.is_allowed(&pid).await.unwrap());
    }

    #[tokio::test]
    async fn trips_open_on_high_error_rate() {
        let (breaker, pid) = make_breaker();
        // 3 failures out of 4 = 75% > 50% threshold
        breaker.record_success(&pid).await.unwrap();
        breaker.record_failure(&pid).await.unwrap();
        breaker.record_failure(&pid).await.unwrap();
        breaker.record_failure(&pid).await.unwrap();
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Open);
    }

    #[tokio::test]
    async fn open_rejects_traffic() {
        let (breaker, pid) = make_breaker();
        // Trip the breaker
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        assert!(!breaker.is_allowed(&pid).await.unwrap());
    }

    #[tokio::test]
    async fn open_transitions_to_half_open_after_cooldown() {
        let store = InMemoryCircuitBreakerStore::new();
        let config = CircuitBreakerConfig {
            error_threshold: 0.5,
            cooldown_secs: 0, // Immediate cooldown for testing
            ..Default::default()
        };
        let pid = ProviderId::new("test");
        let breaker = CircuitBreaker::new(Box::new(store), config);

        // Trip the breaker
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Open);

        // With cooldown=0, is_allowed should transition to HalfOpen
        assert!(breaker.is_allowed(&pid).await.unwrap());
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn half_open_failure_reopens() {
        let store = InMemoryCircuitBreakerStore::new();
        let config = CircuitBreakerConfig {
            error_threshold: 0.5,
            cooldown_secs: 0,
            half_open_max_requests: 3,
            ..Default::default()
        };
        let pid = ProviderId::new("test");
        let breaker = CircuitBreaker::new(Box::new(store), config);

        // Trip → HalfOpen
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        breaker.is_allowed(&pid).await.unwrap(); // Transitions to HalfOpen

        // Failure in HalfOpen → re-opens
        breaker.record_failure(&pid).await.unwrap();
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Open);
    }

    #[tokio::test]
    async fn half_open_successes_close_breaker() {
        let store = InMemoryCircuitBreakerStore::new();
        let config = CircuitBreakerConfig {
            error_threshold: 0.5,
            cooldown_secs: 0,
            half_open_max_requests: 2,
            ..Default::default()
        };
        let pid = ProviderId::new("test");
        let breaker = CircuitBreaker::new(Box::new(store), config);

        // Trip → HalfOpen
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        breaker.is_allowed(&pid).await.unwrap(); // Transitions to HalfOpen, count=1

        // Allow another request through
        breaker.is_allowed(&pid).await.unwrap(); // count=2

        // Record enough successes to close
        breaker.record_success(&pid).await.unwrap();
        breaker.record_success(&pid).await.unwrap();
        // half_open_count (2) >= half_open_max_requests (2) → close
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn half_open_max_requests_limits_traffic() {
        let store = InMemoryCircuitBreakerStore::new();
        let config = CircuitBreakerConfig {
            error_threshold: 0.5,
            cooldown_secs: 0,
            half_open_max_requests: 2,
            ..Default::default()
        };
        let pid = ProviderId::new("test");
        let breaker = CircuitBreaker::new(Box::new(store), config);

        // Trip → HalfOpen
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        assert!(breaker.is_allowed(&pid).await.unwrap()); // count=1
        assert!(breaker.is_allowed(&pid).await.unwrap()); // count=2
        assert!(!breaker.is_allowed(&pid).await.unwrap()); // count=2 >= max=2 → blocked
    }

    #[tokio::test]
    async fn manual_reset_returns_to_closed() {
        let (breaker, pid) = make_breaker();
        for _ in 0..5 {
            breaker.record_failure(&pid).await.unwrap();
        }
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Open);

        breaker.reset(&pid).await.unwrap();
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Closed);
        assert!(breaker.is_allowed(&pid).await.unwrap());
    }

    #[tokio::test]
    async fn stays_closed_below_threshold() {
        let (breaker, pid) = make_breaker();
        // 1 failure out of 10 = 10% < 50% threshold
        for _ in 0..9 {
            breaker.record_success(&pid).await.unwrap();
        }
        breaker.record_failure(&pid).await.unwrap();
        assert_eq!(breaker.state(&pid).await.unwrap(), CircuitState::Closed);
    }
}
