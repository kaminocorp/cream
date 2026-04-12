// cream-api: Axum HTTP server. Wires all crates together into the payment lifecycle orchestrator.

pub mod config;
pub mod db;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod orchestrator;
pub mod routes;
pub mod state;

pub use config::AppConfig;
pub use error::ApiError;
pub use state::AppState;

use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};

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
        // Webhooks (Vision Section 2.5)
        .route("/v1/webhooks", post(routes::webhooks::register))
        // Rate limiting middleware — applied to all /v1/* routes.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit,
        ));

    Router::new()
        // Health check — no auth, no rate limit.
        .route("/health", get(|| async { "ok" }))
        // Merge in API routes.
        .merge(api_routes)
        // Global layers (applied to all routes including /health).
        .layer(middleware::request_id::propagate_request_id_layer())
        .layer(middleware::request_id::set_request_id_layer())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(build_cors_layer(&state.config.cors_allowed_origins))
        .with_state(state)
}

/// Build a CORS layer from configured origins.
///
/// If `allowed_origins` is empty (no `CORS_ALLOWED_ORIGINS` set), falls back to
/// permissive mode for local development. In production, operators MUST set
/// `CORS_ALLOWED_ORIGINS` to restrict cross-origin access.
fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        tracing::warn!("CORS_ALLOWED_ORIGINS not set — using permissive CORS (development only)");
        return CorsLayer::permissive();
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
