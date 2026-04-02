//! # cream-router
//!
//! Provider scoring, circuit breakers, cross-provider idempotency guards,
//! and the route selection engine for the Cream payment control plane.
//!
//! The router takes a payment request and the set of available providers,
//! scores them based on cost, speed, health, and rail preference, then
//! returns a ranked `RoutingDecision`. Circuit breakers automatically
//! demote unhealthy providers. Idempotency guards prevent double-payments
//! across provider failovers.

pub mod circuit_breaker;
pub mod config;
pub mod error;
pub mod idempotency;
pub mod scorer;
pub mod selector;

// Primary re-exports
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerStore, InMemoryCircuitBreakerStore};
pub use config::{CircuitBreakerConfig, IdempotencyConfig, RouterConfig, ScoringWeights};
pub use error::RoutingError;
pub use idempotency::{
    IdempotencyGuard, IdempotencyOutcome, IdempotencyStore, InMemoryIdempotencyStore,
};
pub use scorer::{ProviderCapabilities, ProviderScorer, ScoredProviderInput};
pub use selector::{HealthSource, RouteSelector, StaticHealthSource};
