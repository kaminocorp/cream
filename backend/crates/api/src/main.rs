use std::collections::HashMap;
use std::sync::Arc;

use cream_api::config::AppConfig;
use cream_api::db::PgPaymentRepository;
use cream_api::notifications::{self, NotificationSender};
use cream_api::notifications::email::{EmailConfig, EmailNotifier};
use cream_api::notifications::slack::{SlackConfig, SlackNotifier};
use cream_api::alert_engine::alert_evaluation_worker;
use cream_api::orchestrator::{credential_age_monitor, escalation_timeout_monitor};
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

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use rust_decimal::Decimal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration first (before tracing init) so LOG_FORMAT and
    // OTEL_ENABLED can drive the subscriber setup.
    let config = AppConfig::from_env()?;

    // Build the env filter: RUST_LOG takes precedence, then LOG_LEVEL fallback.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = &config.log_level;
        EnvFilter::new(format!(
            "cream_api={level},cream_models={level},cream_policy={level},\
             cream_providers={level},cream_router={level},cream_audit={level},{level}"
        ))
    });

    // Build the subscriber as a layered stack:
    // 1. Registry base
    // 2. EnvFilter
    // 3. Fmt layer (JSON or pretty)
    // 4. [optional] OpenTelemetry layer
    //
    // The OTEL tracer provider is initialised once, but the tracing layer is
    // created inside each fmt branch because `OpenTelemetryLayer` is generic
    // over the subscriber type and the JSON/pretty branches produce different
    // concrete types. The `.with(Option<L>)` pattern means `None` adds zero
    // overhead when OTEL is disabled.
    let tracer_provider = if config.otel_enabled {
        let endpoint = config.otel_exporter_endpoint.as_deref()
            .expect("OTEL_EXPORTER_OTLP_ENDPOINT validated as required in config");

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name(config.otel_service_name.clone())
                    .build(),
            )
            .build();

        opentelemetry::global::set_tracer_provider(provider.clone());
        Some(provider)
    } else {
        None
    };

    match config.log_format {
        cream_api::config::LogFormat::Json => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_span_list(false)
                .flatten_event(true);

            let otel_layer = tracer_provider.as_ref().map(|tp| {
                tracing_opentelemetry::OpenTelemetryLayer::new(tp.tracer("cream-api"))
            });

            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(otel_layer)
                .init();
        }
        cream_api::config::LogFormat::Pretty => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_target(true);

            let otel_layer = tracer_provider.as_ref().map(|tp| {
                tracing_opentelemetry::OpenTelemetryLayer::new(tp.tracer("cream-api"))
            });

            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(otel_layer)
                .init();
        }
    }

    if config.otel_enabled {
        tracing::info!(
            endpoint = config.otel_exporter_endpoint.as_deref().unwrap_or(""),
            service_name = %config.otel_service_name,
            "OpenTelemetry tracing enabled"
        );
    }
    // Initialise Prometheus metrics (Phase 17-C). The recorder is global and
    // starts an HTTP listener on METRICS_PORT for Prometheus scraping.
    let metrics_handle = if config.metrics_enabled {
        let handle = cream_api::metrics::init_metrics(config.metrics_port);
        tracing::info!(port = config.metrics_port, "Prometheus metrics enabled on /metrics");
        Some(handle)
    } else {
        None
    };

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
        metrics_handle,
    };

    // Build router.
    let router = cream_api::build_router(state.clone());

    // Spawn background workers with panic supervision. Each task is named
    // and wrapped so that a panic surfaces immediately in logs rather than
    // being silently swallowed by a discarded JoinHandle.
    let mut worker_handles = tokio::task::JoinSet::new();

    let monitor_state = state.clone();
    worker_handles.spawn(async move {
        escalation_timeout_monitor(monitor_state).await;
    });

    let delivery_state = state.clone();
    worker_handles.spawn(async move {
        webhook_delivery_worker(delivery_state).await;
    });

    let retry_state = state.clone();
    worker_handles.spawn(async move {
        webhook_retry_worker(retry_state).await;
    });

    let credential_state = state.clone();
    worker_handles.spawn(async move {
        credential_age_monitor(credential_state).await;
    });

    let alert_state = state.clone();
    worker_handles.spawn(async move {
        alert_evaluation_worker(alert_state).await;
    });

    // Supervisor: if any background worker panics or exits unexpectedly,
    // log the error immediately. Runs in its own task so it doesn't block
    // the main server loop.
    tokio::spawn(async move {
        while let Some(result) = worker_handles.join_next().await {
            match result {
                Ok(()) => {
                    tracing::warn!("background worker exited unexpectedly (returned Ok)");
                }
                Err(e) if e.is_panic() => {
                    tracing::error!(
                        error = %e,
                        "background worker panicked — this is a bug"
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "background worker task error");
                }
            }
        }
    });

    // Start serving with graceful shutdown.
    let addr = format!("{}:{}", config.host, config.port);

    if let (Some(cert_path), Some(key_path)) = (&config.tls_cert_path, &config.tls_key_path) {
        // TLS mode: use axum-server with rustls.
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .expect("failed to load TLS certificate/key — check TLS_CERT_PATH and TLS_KEY_PATH");

        tracing::info!(%addr, "cream-api listening (HTTPS/TLS)");

        // axum-server uses a Handle for graceful shutdown instead of
        // `.with_graceful_shutdown()`.
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        let otel = config.otel_enabled;
        tokio::spawn(async move {
            shutdown_signal(otel).await;
            shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
        });

        axum_server::bind_rustls(addr.parse()?, tls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    } else {
        // Plain HTTP (local dev or behind a reverse proxy).
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        tracing::info!(%addr, "cream-api listening (HTTP)");

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal(config.otel_enabled))
            .await?;
    }

    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM, then flush OTEL traces before exit.
async fn shutdown_signal(otel_enabled: bool) {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("received SIGINT, shutting down"); }
        _ = terminate => { tracing::info!("received SIGTERM, shutting down"); }
    }

    if otel_enabled {
        // SdkTracerProvider flushes pending spans on drop. Logging here confirms
        // the shutdown path was reached; the actual flush happens when the global
        // provider is dropped at process exit.
        tracing::info!("OpenTelemetry tracer provider will flush on shutdown");
    }
}
