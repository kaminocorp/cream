use axum::extract::State;
use axum::Json;
use cream_models::prelude::*;
use serde::Serialize;

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedAgent;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ProviderHealthResponse {
    pub providers: Vec<ProviderHealth>,
}

/// `GET /v1/providers/health` — real-time health status of all connected providers.
pub async fn health(
    State(state): State<AppState>,
    _agent: AuthenticatedAgent,
) -> Result<Json<ProviderHealthResponse>, ApiError> {
    let mut providers = Vec::new();

    for provider in state.provider_registry.all() {
        match provider.health_check().await {
            Ok(health) => providers.push(health),
            Err(e) => {
                tracing::warn!(
                    provider = %provider.provider_id(),
                    error = %e,
                    "health check failed"
                );
                // Include a degraded health entry rather than omitting.
                providers.push(ProviderHealth {
                    provider_id: provider.provider_id().clone(),
                    is_healthy: false,
                    error_rate_5m: 1.0,
                    p50_latency_ms: 0,
                    p99_latency_ms: 0,
                    last_checked_at: chrono::Utc::now(),
                    circuit_state: CircuitState::Open,
                });
            }
        }
    }

    Ok(Json(ProviderHealthResponse { providers }))
}
