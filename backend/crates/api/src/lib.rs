// cream-api: Axum HTTP server. Wires all crates together into the payment lifecycle orchestrator.

pub mod alert_engine;
pub mod audit_export;
pub mod config;
mod docs_coverage;
pub mod openapi;
pub mod db;
pub mod error;
pub mod extractors;
pub mod metrics;
pub mod middleware;
pub mod notifications;
pub mod orchestrator;
pub mod routes;
pub mod state;
pub mod webhook_worker;

pub use config::AppConfig;
pub use error::ApiError;
pub use state::AppState;

use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa_swagger_ui::SwaggerUi;

/// Build the Axum router with all routes, middleware, and shared state.
pub fn build_router(state: AppState) -> Router {
    // API routes (require auth via extractor, rate-limited).
    let api_routes = Router::new()
        // Payments (Vision Section 4.1)
        .route("/v1/payments", post(routes::payments::initiate))
        .route("/v1/payments/{id}", get(routes::payments::get_status))
        .route("/v1/payments/{id}/approve", post(routes::payments::approve))
        .route("/v1/payments/{id}/reject", post(routes::payments::reject))
        // Virtual Cards (Vision Section 7.3)
        .route("/v1/cards", post(routes::cards::create))
        .route(
            "/v1/cards/{id}",
            patch(routes::cards::update).delete(routes::cards::cancel),
        )
        // Audit (Vision Section 8)
        .route("/v1/audit", get(routes::audit::query))
        // Agent Management (Phase 15.1 — operator-only lifecycle endpoints)
        .route(
            "/v1/agents",
            get(routes::agents::list_agents).post(routes::agents::create_agent),
        )
        .route(
            "/v1/agents/{id}",
            patch(routes::agents::update_agent),
        )
        .route(
            "/v1/agents/{id}/rotate-key",
            post(routes::agents::rotate_agent_key),
        )
        // Agent Policy (Vision Section 4.3)
        .route(
            "/v1/agents/{id}/policy",
            get(routes::agents::get_policy).put(routes::agents::update_policy),
        )
        // Provider Health (Vision Section 6.1)
        .route("/v1/providers/health", get(routes::providers::health))
        // Webhooks (Vision Section 2.5 + Phase 16-A)
        .route(
            "/v1/webhooks",
            get(routes::webhooks::list_webhooks).post(routes::webhooks::register),
        )
        .route(
            "/v1/webhooks/{id}",
            delete(routes::webhooks::delete_webhook),
        )
        .route(
            "/v1/webhooks/{id}/deliveries",
            get(routes::webhooks::list_deliveries),
        )
        .route(
            "/v1/webhooks/{id}/test",
            post(routes::webhooks::test_webhook),
        )
        // Settings (Phase 16-F)
        .route(
            "/v1/settings/provider-keys",
            get(routes::settings::list_provider_keys).put(routes::settings::save_provider_key),
        )
        // Alerts (Phase 17-G)
        .route(
            "/v1/alerts",
            get(routes::alerts::list_alerts).post(routes::alerts::create_alert),
        )
        .route(
            "/v1/alerts/{id}",
            patch(routes::alerts::update_alert).delete(routes::alerts::delete_alert),
        )
        .route(
            "/v1/alerts/history",
            get(routes::alerts::alert_history),
        )
        // Audit Export (Phase 17-E)
        .route(
            "/v1/audit/export",
            post(routes::audit_export::create_export),
        )
        .route(
            "/v1/audit/exports/{id}",
            get(routes::audit_export::get_export_status),
        )
        // Policy Templates (Phase 16-G)
        .route(
            "/v1/policy-templates",
            get(routes::templates::list_templates),
        )
        .route(
            "/v1/policy-templates/{id}",
            get(routes::templates::get_template),
        )
        .route(
            "/v1/policy-templates/{template_id}/apply/{agent_id}",
            post(routes::templates::apply_template),
        )
        // Rate limiting middleware — applied to all /v1/* routes.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit,
        ));

    // PII-redacted body logging — only active when LOG_BODIES=true.
    // Placed after rate limiting so rejected requests don't incur body buffering.
    let api_routes = if state.config.log_bodies {
        api_routes.layer(axum::middleware::from_fn(
            middleware::logging::log_bodies_with_redaction,
        ))
    } else {
        api_routes
    };

    // Auth routes — behind a stricter IP-based rate limiter (20 req/60s)
    // to prevent brute-force attacks on login/register. Separate from the
    // main per-agent rate limiter since auth callers don't yet have a token.
    let auth_routes = Router::new()
        .route("/v1/auth/status", get(routes::auth::status))
        .route("/v1/auth/register", post(routes::auth::register))
        .route("/v1/auth/login", post(routes::auth::login))
        .route("/v1/auth/refresh", post(routes::auth::refresh))
        .route("/v1/auth/logout", post(routes::auth::logout))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::auth_rate_limit,
        ));

    // Integration callback routes — verified by their own signing secrets.
    let integration_routes = Router::new().route(
        "/v1/integrations/slack/callback",
        post(routes::integrations::slack_callback),
    );

    // Build the OpenAPI spec once for the spec endpoint and Swagger UI.
    let openapi_spec = openapi::build_openapi_spec();

    Router::new()
        // Health check — no auth, no rate limit.
        .route("/health", get(|| async { "ok" }))
        // OpenAPI spec (Phase 17-F) — unauthenticated for developer access.
        .route(
            "/v1/openapi.json",
            get({
                let spec = openapi_spec.clone();
                move || {
                    let spec = spec.clone();
                    async move { axum::Json(spec) }
                }
            }),
        )
        // Swagger UI — serves interactive API docs.
        .merge(SwaggerUi::new("/docs").url("/v1/openapi.json", openapi_spec))
        // Merge in API routes (rate-limited).
        .merge(api_routes)
        // Merge in auth routes (IP-based rate-limited, separate from API).
        .merge(auth_routes)
        // Merge in integration callback routes.
        .merge(integration_routes)
        // Global layers (applied to all routes including /health).
        // Order matters: outermost layer runs first. The stack below executes as:
        // 1. CORS (outermost)
        // 2. TraceLayer creates a span with a `request_id` field (empty initially)
        // 3. SetRequestId layer assigns X-Request-Id header
        // 4. inject_request_id middleware reads the header and records it into the span
        // 5. PropagateRequestId copies X-Request-Id to the response
        //
        // This means every tracing event emitted inside the request (including
        // nested spans from orchestrator, policy engine, audit writer, etc.)
        // automatically includes `request_id` in structured log output.
        .layer(middleware::request_id::propagate_request_id_layer())
        .layer(axum::middleware::from_fn(
            middleware::request_id::inject_request_id,
        ))
        .layer(middleware::request_id::set_request_id_layer())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = tracing::field::Empty,
                )
            }),
        )
        .layer(build_cors_layer(&state.config.cors_allowed_origins))
        // Security headers — outermost application layer so every response
        // (including CORS preflight, errors, 404s) gets hardened headers.
        .layer(middleware::security_headers::SecurityHeadersLayer::new(
            state.config.hsts_max_age,
        ))
        .with_state(state)
}

/// Build a CORS layer from configured origins.
///
/// If `allowed_origins` is empty, requires `ALLOW_PERMISSIVE_CORS=true` to
/// fall back to permissive mode (for local development). Without the explicit
/// opt-in, an empty origin list panics at startup — fail-fast rather than
/// silently running with all origins allowed.
fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        let allow_permissive = std::env::var("ALLOW_PERMISSIVE_CORS")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        if allow_permissive {
            tracing::warn!(
                "CORS_ALLOWED_ORIGINS not set and ALLOW_PERMISSIVE_CORS=true — \
                 using permissive CORS (development only)"
            );
            return CorsLayer::permissive();
        }
        panic!(
            "CORS_ALLOWED_ORIGINS is empty and ALLOW_PERMISSIVE_CORS is not set. \
             Either set CORS_ALLOWED_ORIGINS to a comma-separated list of origins, \
             or set ALLOW_PERMISSIVE_CORS=true for local development."
        );
    }

    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
}
