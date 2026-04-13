use std::collections::HashMap;
use std::sync::Arc;

use cream_api::config::AppConfig;
use cream_api::db::PgPaymentRepository;
use cream_api::notifications::{self, NotificationSender};
use cream_api::notifications::email::{EmailConfig, EmailNotifier};
use cream_api::notifications::slack::{SlackConfig, SlackNotifier};
use cream_api::orchestrator::escalation_timeout_monitor;
use cream_api::webhook_worker::{webhook_delivery_worker, webhook_retry_worker};
use cream_api::state::AppState;
use cream_audit::{PgAuditReader, PgAuditWriter};
use cream_models::prelude::*;
use cream_policy::PolicyEngine;
use cream_providers::{MockProvider, ProviderRegistry};
use cream_router::{
    CircuitBreaker, CircuitBreakerConfig, IdempotencyConfig, IdempotencyGuard,
    InMemoryCircuitBreakerStore, InMemoryIdempotencyStore, ProviderCapabilities, ProviderScorer,
    RouteSelector, ScoringWeights, StaticHealthSource,
};
use std::str::FromStr;

use rust_decimal::Decimal;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise tracing: controlled via RUST_LOG env var.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("cream_api=debug,cream_models=debug,cream_policy=debug,cream_providers=debug,cream_router=debug,cream_audit=debug,info")
    });
    fmt().with_env_filter(filter).with_target(true).init();

    // Load configuration.
    let config = AppConfig::from_env()?;
    tracing::info!(host = %config.host, port = config.port, "loading configuration");

    // Database pool.
    let db = sqlx::PgPool::connect(&config.database_url).await?;
    tracing::info!("connected to PostgreSQL");

    // Redis connection.
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("connected to Redis");

    // Policy engine (stateless — registers all 12 built-in evaluators).
    let policy_engine = Arc::new(PolicyEngine::new());

    // Provider registry with a mock provider for scaffold.
    let mut provider_registry = ProviderRegistry::new();
    let mock_provider = MockProvider::success("mock_provider");
    provider_registry.register(Arc::new(mock_provider));
    let provider_registry = Arc::new(provider_registry);

    // Routing engine.
    let scorer = ProviderScorer::new(ScoringWeights::default())?;
    let mock_id = ProviderId::new("mock_provider");
    let mut capabilities = HashMap::new();
    capabilities.insert(
        mock_id.clone(),
        ProviderCapabilities {
            provider_id: mock_id.clone(),
            supported_rails: vec![
                RailPreference::Auto,
                RailPreference::Card,
                RailPreference::Stablecoin,
            ],
            supported_currencies: vec![Currency::USD, Currency::SGD, Currency::EUR],
            fee_percentage: Decimal::from_str("0.029").unwrap(),
            flat_fee_usd: Decimal::from_str("0.30").unwrap(),
        },
    );

    let mut health_map = HashMap::new();
    health_map.insert(
        mock_id.clone(),
        ProviderHealth {
            provider_id: mock_id,
            is_healthy: true,
            error_rate_5m: 0.0,
            p50_latency_ms: 50,
            p99_latency_ms: 200,
            last_checked_at: chrono::Utc::now(),
            circuit_state: CircuitState::Closed,
        },
    );

    let route_selector = Arc::new(RouteSelector::new(
        scorer,
        capabilities,
        Box::new(StaticHealthSource::new(health_map)),
    ));

    // Audit writer/reader.
    let audit_writer = Arc::new(PgAuditWriter::new(db.clone()));
    let audit_reader = Arc::new(PgAuditReader::new(db.clone()));

    // Circuit breaker (in-memory store for scaffold).
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        Box::new(InMemoryCircuitBreakerStore::new()),
        CircuitBreakerConfig::default(),
    )?);

    // Idempotency guard (in-memory store for scaffold).
    let idempotency_guard = Arc::new(IdempotencyGuard::new(
        Box::new(InMemoryIdempotencyStore::new()),
        IdempotencyConfig::default(),
    )?);

    // Payment repository.
    let payment_repo = Arc::new(PgPaymentRepository::new(db.clone()));

    // Notification sender (Slack, email, etc.). Falls back to NoopNotifier
    // when no channel is configured.
    let notification_sender: Arc<dyn NotificationSender> = {
        let mut senders: Vec<Box<dyn NotificationSender>> = Vec::new();

        if let Some(slack_config) = SlackConfig::from_env() {
            tracing::info!("Slack escalation notifications enabled");
            senders.push(Box::new(SlackNotifier::new(slack_config)));
        }

        if let Some(email_config) = EmailConfig::from_env() {
            tracing::info!("Email escalation notifications enabled");
            senders.push(Box::new(EmailNotifier::new(email_config)));
        }

        if senders.is_empty() {
            Arc::new(notifications::NoopNotifier)
        } else {
            Arc::new(notifications::CompositeNotifier::new(senders))
        }
    };

    // Build AppState.
    let state = AppState {
        db,
        redis,
        policy_engine,
        route_selector,
        provider_registry,
        audit_writer,
        audit_reader,
        idempotency_guard,
        circuit_breaker,
        payment_repo,
        notification_sender,
        config: Arc::new(config.clone()),
    };

    // Build router.
    let router = cream_api::build_router(state.clone());

    // Spawn the escalation timeout monitor.
    let monitor_state = state.clone();
    tokio::spawn(async move {
        escalation_timeout_monitor(monitor_state).await;
    });

    // Spawn webhook delivery workers (Phase 16-A).
    let delivery_state = state.clone();
    tokio::spawn(async move {
        webhook_delivery_worker(delivery_state).await;
    });
    let retry_state = state.clone();
    tokio::spawn(async move {
        webhook_retry_worker(retry_state).await;
    });

    // Start serving.
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "cream-api listening");
    axum::serve(listener, router).await?;

    Ok(())
}
