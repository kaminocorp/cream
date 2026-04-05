use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::AppState;

/// Per-agent fixed-window rate limiting via Redis.
///
/// Reads the bearer token from the Authorization header and hashes it
/// to identify the agent (same hash the auth extractor computes).
///
/// Redis key: `cream:rate:{key_hash}:{window_epoch}`
/// where `window_epoch = unix_secs / window_secs`.
///
/// On Redis failure: fail-open (log warning, allow request through).
pub async fn rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let agent_key_hash = extract_key_hash_from_header(&request);

    if let Some(key_hash) = agent_key_hash {
        let window_secs = state.config.rate_limit_window_secs;
        let max_requests = state.config.rate_limit_requests;

        let now = chrono::Utc::now().timestamp() as u64;
        let window_epoch = now / window_secs;
        let key = format!("cream:rate:{key_hash}:{window_epoch}");

        match increment_counter(&state.redis, &key, window_secs).await {
            Ok(count) => {
                if count > max_requests {
                    let retry_after = window_secs - (now % window_secs);
                    return Err(ApiError::RateLimited {
                        retry_after_secs: retry_after,
                    });
                }
            }
            Err(e) => {
                // Fail-open: Redis unavailable should not block requests.
                tracing::warn!(error = %e, "rate limiter: redis unavailable, allowing request");
            }
        }
    }

    Ok(next.run(request).await)
}

/// Extract the bearer token and hash it for rate-limit identity.
/// This avoids a DB round-trip in the middleware layer.
fn extract_key_hash_from_header(request: &Request<Body>) -> Option<String> {
    let header = request.headers().get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    if token.is_empty() {
        return None;
    }
    use sha2::{Digest, Sha256};
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    Some(hash)
}

async fn increment_counter(
    redis: &redis::aio::ConnectionManager,
    key: &str,
    window_secs: u64,
) -> Result<u64, redis::RedisError> {
    let mut conn = redis.clone();
    let count: u64 = redis::cmd("INCR").arg(key).query_async(&mut conn).await?;

    // Set expiry only on the first increment (when count == 1) to avoid
    // resetting the TTL on subsequent requests within the same window.
    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(key)
            .arg(window_secs)
            .query_async(&mut conn)
            .await?;
    }

    Ok(count)
}
