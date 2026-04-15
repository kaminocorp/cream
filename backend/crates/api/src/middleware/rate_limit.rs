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

        check_rate_limit(&state, &key_hash, "cream:rate", window_secs, max_requests).await?;
    }

    Ok(next.run(request).await)
}

/// Stricter rate limiting for auth routes (login, register, refresh).
///
/// Uses the source IP (or forwarded IP) instead of bearer token, since auth
/// callers don't yet have a token. Limit: 20 requests per 60-second window
/// — generous enough for normal usage, tight enough to block brute-force.
///
/// Redis key: `cream:auth_rate:{ip_hash}:{window_epoch}`
pub async fn auth_rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let ip_key = extract_ip_hash(&request);

    // 20 attempts per 60s window — prevents brute-force while allowing
    // legitimate login retries and token refreshes.
    const AUTH_WINDOW_SECS: u64 = 60;
    const AUTH_MAX_REQUESTS: u64 = 20;

    check_rate_limit(&state, &ip_key, "cream:auth_rate", AUTH_WINDOW_SECS, AUTH_MAX_REQUESTS).await?;

    Ok(next.run(request).await)
}

/// Shared rate-limit check used by both API and auth rate limiters.
async fn check_rate_limit(
    state: &AppState,
    identity_hash: &str,
    key_prefix: &str,
    window_secs: u64,
    max_requests: u64,
) -> Result<(), ApiError> {
    let now = chrono::Utc::now().timestamp() as u64;
    let window_epoch = now / window_secs;
    let key = format!("{key_prefix}:{identity_hash}:{window_epoch}");

    match increment_counter(&state.redis, &key, window_secs).await {
        Ok(count) => {
            if count > max_requests {
                ::metrics::counter!(crate::metrics::RATE_LIMIT_HITS_TOTAL).increment(1);
                let retry_after = window_secs - (now % window_secs);
                return Err(ApiError::RateLimited {
                    retry_after_secs: retry_after,
                });
            }
        }
        Err(e) => {
            // Fail-open: Redis unavailable should not block requests.
            ::metrics::counter!(crate::metrics::REDIS_CONNECTION_ERRORS_TOTAL).increment(1);
            tracing::warn!(error = %e, "rate limiter: redis unavailable, allowing request");
        }
    }

    Ok(())
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

/// Extract the client IP and hash it for auth rate-limit identity.
///
/// Checks `X-Forwarded-For` first (reverse proxy scenario), then falls back
/// to a static key. The IP is hashed to avoid storing raw IPs in Redis.
fn extract_ip_hash(request: &Request<Body>) -> String {
    use sha2::{Digest, Sha256};

    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    hex::encode(Sha256::digest(ip.as_bytes()))
}

/// Lua script for truly atomic INCR + conditional EXPIRE.
///
/// Redis pipelines batch commands over the network but do NOT execute them
/// atomically — other clients can interleave commands between INCR and EXPIRE.
/// A Lua script runs inside Redis's single-threaded event loop, making the
/// increment-and-set-TTL operation indivisible.
///
/// Only the first request in a window (count == 1) sets the TTL, avoiding
/// redundant EXPIRE calls. If a key somehow loses its TTL (Redis bug, AOF
/// corruption), it will eventually be replaced by a new window key since the
/// key includes the window epoch.
const RATE_LIMIT_SCRIPT: &str = r#"
    local count = redis.call('INCR', KEYS[1])
    if count == 1 then
        redis.call('EXPIRE', KEYS[1], ARGV[1])
    end
    return count
"#;

async fn increment_counter(
    redis: &redis::aio::ConnectionManager,
    key: &str,
    window_secs: u64,
) -> Result<u64, redis::RedisError> {
    let mut conn = redis.clone();

    let count: u64 = redis::Script::new(RATE_LIMIT_SCRIPT)
        .key(key)
        .arg(window_secs)
        .invoke_async(&mut conn)
        .await?;

    Ok(count)
}
