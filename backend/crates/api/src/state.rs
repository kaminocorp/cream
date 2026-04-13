use std::sync::Arc;

use cream_audit::{AuditReader, AuditWriter};
use cream_policy::PolicyEngine;
use cream_providers::ProviderRegistry;
use cream_router::{CircuitBreaker, IdempotencyGuard, RouteSelector};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::db::PaymentRepository;
use crate::notifications::NotificationSender;
use metrics_exporter_prometheus::PrometheusHandle;

/// Shared application state injected into every Axum handler via `State<AppState>`.
///
/// All fields are cheaply cloneable (`Arc`-wrapped or natively `Clone`), which
/// is required by Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub policy_engine: Arc<PolicyEngine>,
    pub route_selector: Arc<RouteSelector>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub audit_writer: Arc<dyn AuditWriter>,
    pub audit_reader: Arc<dyn AuditReader>,
    pub idempotency_guard: Arc<IdempotencyGuard>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub payment_repo: Arc<dyn PaymentRepository>,
    pub notification_sender: Arc<dyn NotificationSender>,
    pub config: Arc<AppConfig>,
    /// Prometheus metrics handle for reading metric values. `None` when
    /// metrics are disabled. Used by the alert engine (Phase 17-G).
    pub metrics_handle: Option<PrometheusHandle>,
}
