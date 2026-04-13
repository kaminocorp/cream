//! Outbound webhook delivery system.
//!
//! Two background workers run as long-lived Tokio tasks:
//!
//! 1. **Delivery worker** — pops events from a Redis queue (`cream:webhook:queue`)
//!    and delivers them to registered webhook endpoints. Successful deliveries
//!    are logged as `delivered`; failures are scheduled for retry.
//!
//! 2. **Retry worker** — periodically polls the `webhook_delivery_log` table for
//!    failed deliveries whose `next_retry_at` has passed, and re-attempts them
//!    with exponential backoff.
//!
//! Webhook payloads are signed with HMAC-SHA256 using a Stripe-compatible scheme:
//! `Cream-Signature: t=<unix_ts>,v1=<hex_hmac>`.

use std::time::Duration;

use chrono::Utc;
use hmac::{Hmac, Mac};
use redis::AsyncCommands;
use sha2::Sha256;
use uuid::Uuid;

use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Redis key for the webhook event queue.
const WEBHOOK_QUEUE_KEY: &str = "cream:webhook:queue";

/// Maximum response body length stored in the delivery log (truncated beyond).
const MAX_RESPONSE_BODY_LEN: usize = 2048;

/// Backoff schedule for retries. Index = attempt number (0-based after first failure).
/// 5s, 30s, 2m, 15m, 1h.
const RETRY_BACKOFF_SECS: [i64; 5] = [5, 30, 120, 900, 3600];

// ---------------------------------------------------------------------------
// Webhook event (serialized into Redis queue)
// ---------------------------------------------------------------------------

/// An event to be delivered to webhook endpoints.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookEvent {
    /// Event type identifier, e.g. "payment.settled", "payment.failed".
    pub event_type: String,
    /// The full event payload (JSON object).
    pub payload: serde_json::Value,
    /// Agent ID scoping — only endpoints registered by this agent (or wildcard)
    /// will receive the event. `None` means broadcast to all endpoints.
    pub agent_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Signing
// ---------------------------------------------------------------------------

/// Produce a Stripe-compatible HMAC-SHA256 signature.
///
/// The signed message is `<timestamp>.<body>` where `timestamp` is a Unix
/// epoch in seconds and `body` is the raw JSON payload. The result is
/// formatted as `t=<ts>,v1=<hex_hmac>`.
pub fn sign_payload(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret)
        .expect("HMAC-SHA256 accepts any key length");
    mac.update(format!("{timestamp}.").as_bytes());
    mac.update(body);
    let result = mac.finalize();
    let hex = hex::encode(result.into_bytes());
    format!("t={timestamp},v1={hex}")
}

/// Maximum age (in seconds) of a webhook signature before it is rejected.
/// Prevents replay attacks with old signatures.
const SIGNATURE_MAX_AGE_SECS: i64 = 300; // 5 minutes

/// Verify a Cream-Signature header against the expected secret and body.
/// Returns `true` if the signature is valid and the timestamp is within
/// the allowed window (±5 minutes).
pub fn verify_signature(secret: &[u8], signature: &str, body: &[u8]) -> bool {
    // Parse "t=<ts>,v1=<hex>"
    let parts: Vec<&str> = signature.split(',').collect();
    if parts.len() != 2 {
        return false;
    }
    let timestamp = match parts[0].strip_prefix("t=") {
        Some(t) => t,
        None => return false,
    };
    let ts_i64: i64 = match timestamp.parse() {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Reject signatures with timestamps outside the allowed window.
    let now = Utc::now().timestamp();
    if (now - ts_i64).unsigned_abs() > SIGNATURE_MAX_AGE_SECS as u64 {
        return false;
    }

    let expected = sign_payload(secret, ts_i64, body);
    // Constant-time comparison to prevent timing attacks.
    expected.len() == signature.len()
        && expected
            .as_bytes()
            .iter()
            .zip(signature.as_bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

// ---------------------------------------------------------------------------
// Enqueue
// ---------------------------------------------------------------------------

/// Push a webhook event onto the Redis queue for async delivery.
///
/// This is a non-blocking fire-and-forget operation. If Redis is unavailable,
/// the event is logged but not retried — the caller (orchestrator) should not
/// block the payment pipeline on webhook delivery.
pub async fn enqueue_webhook(
    redis: &redis::aio::ConnectionManager,
    event: WebhookEvent,
) {
    let serialized = match serde_json::to_string(&event) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize webhook event");
            return;
        }
    };
    let mut conn = redis.clone();
    if let Err(e) = conn.lpush::<_, _, ()>(WEBHOOK_QUEUE_KEY, &serialized).await {
        tracing::error!(
            error = %e,
            event_type = %event.event_type,
            "failed to enqueue webhook event — event will not be delivered"
        );
    }
}

// ---------------------------------------------------------------------------
// Delivery worker
// ---------------------------------------------------------------------------

/// Long-running task that pops events from the Redis queue and delivers them.
///
/// For each event, it looks up all matching webhook endpoints, creates a
/// delivery log entry, signs the payload, and POSTs to the endpoint URL.
pub async fn webhook_delivery_worker(state: AppState) {
    tracing::info!("webhook delivery worker started");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            state.config.webhook_delivery_timeout_secs,
        ))
        .build()
        .expect("reqwest client build should not fail");

    loop {
        // BRPOP blocks until an event is available (5s timeout to check for shutdown).
        let mut conn = state.redis.clone();
        let result: Result<Option<(String, String)>, _> =
            redis::cmd("BRPOP")
                .arg(WEBHOOK_QUEUE_KEY)
                .arg(5) // 5 second timeout
                .query_async(&mut conn)
                .await;

        let raw = match result {
            Ok(Some((_key, value))) => value,
            Ok(None) => continue, // timeout, loop again
            Err(e) => {
                tracing::warn!(error = %e, "redis BRPOP error in webhook worker");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let event: WebhookEvent = match serde_json::from_str(&raw) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = %e, raw = %raw, "malformed webhook event in queue");
                continue;
            }
        };

        // Look up matching endpoints.
        let endpoints = match load_matching_endpoints(&state.db, &event).await {
            Ok(eps) => eps,
            Err(e) => {
                tracing::error!(error = %e, "failed to load webhook endpoints");
                continue;
            }
        };

        for ep in endpoints {
            deliver_to_endpoint(&state.db, &client, &ep, &event).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Retry worker
// ---------------------------------------------------------------------------

/// Periodically scans the delivery log for failed deliveries eligible for retry.
pub async fn webhook_retry_worker(state: AppState) {
    tracing::info!("webhook retry worker started");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            state.config.webhook_delivery_timeout_secs,
        ))
        .build()
        .expect("reqwest client build should not fail");

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;

        let rows = match sqlx::query_as::<_, RetryRow>(
            "SELECT dl.id, dl.webhook_endpoint_id, dl.event_type, dl.payload,
                    dl.attempt, dl.max_attempts, dl.signature,
                    we.url, we.secret_hash
             FROM webhook_delivery_log dl
             JOIN webhook_endpoints we ON we.id = dl.webhook_endpoint_id
             WHERE dl.status = 'failed'
               AND dl.attempt < dl.max_attempts
               AND dl.next_retry_at <= now()
             ORDER BY dl.next_retry_at ASC
             LIMIT 50
             FOR UPDATE OF dl SKIP LOCKED",
        )
        .fetch_all(&state.db)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "webhook retry query failed");
                continue;
            }
        };

        for row in rows {
            retry_delivery(&state.db, &client, row).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct WebhookEndpointRow {
    id: Uuid,
    url: String,
    secret_hash: String,
    events: serde_json::Value,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)] // Fields read by sqlx::FromRow derive, not all accessed in Rust
struct RetryRow {
    id: Uuid,
    webhook_endpoint_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    attempt: i16,
    max_attempts: i16,
    signature: String,
    url: String,
    secret_hash: String,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load webhook endpoints that match the event (by event type filter and agent scope).
async fn load_matching_endpoints(
    db: &sqlx::PgPool,
    event: &WebhookEvent,
) -> Result<Vec<WebhookEndpointRow>, sqlx::Error> {
    // We fetch all active endpoints and filter in Rust because the JSONB
    // event filter array is small and the endpoint count per tenant is low.
    let rows: Vec<WebhookEndpointRow> = sqlx::query_as(
        "SELECT id, url, secret_hash, events
         FROM webhook_endpoints
         WHERE status = 'active'
           AND (agent_id IS NULL OR agent_id = $1)",
    )
    .bind(event.agent_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|ep| event_matches(&ep.events, &event.event_type))
        .collect())
}

/// Check if an endpoint's event filter matches a given event type.
/// A filter of `["*"]` matches everything. Otherwise the event_type must
/// appear literally, or a prefix match is checked (e.g. "payment.*" matches
/// "payment.settled").
fn event_matches(events: &serde_json::Value, event_type: &str) -> bool {
    let arr = match events.as_array() {
        Some(a) => a,
        None => {
            // Malformed events column (not a JSON array) — fail closed by
            // rejecting all events. Silently accepting all would be a data
            // leak if someone corrupts the column.
            tracing::warn!(
                events = %events,
                "webhook endpoint has malformed events filter (not an array) — rejecting all events"
            );
            return false;
        }
    };
    for v in arr {
        let s = match v.as_str() {
            Some(s) => s,
            None => continue,
        };
        if s == "*" || s == event_type {
            return true;
        }
        // Prefix match: "payment.*" matches "payment.settled"
        if let Some(prefix) = s.strip_suffix(".*") {
            if event_type.starts_with(prefix) {
                return true;
            }
        }
    }
    false
}

/// Deliver a single event to a single endpoint. Creates the delivery log entry,
/// sends the HTTP POST, and updates the log with the result.
async fn deliver_to_endpoint(
    db: &sqlx::PgPool,
    client: &reqwest::Client,
    ep: &WebhookEndpointRow,
    event: &WebhookEvent,
) {
    let body = match serde_json::to_vec(&serde_json::json!({
        "event_type": event.event_type,
        "payload": event.payload,
        "timestamp": Utc::now().to_rfc3339(),
    })) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize webhook body");
            return;
        }
    };

    let timestamp = Utc::now().timestamp();
    let signature = sign_payload(ep.secret_hash.as_bytes(), timestamp, &body);
    let delivery_id = Uuid::now_v7();
    let max_attempts: i16 = 5;

    // Insert delivery log row.
    if let Err(e) = sqlx::query(
        "INSERT INTO webhook_delivery_log
            (id, webhook_endpoint_id, event_type, payload, status, attempt, max_attempts, signature, created_at)
         VALUES ($1, $2, $3, $4, 'pending', 0, $5, $6, now())",
    )
    .bind(delivery_id)
    .bind(ep.id)
    .bind(&event.event_type)
    .bind(&event.payload)
    .bind(max_attempts)
    .bind(&signature)
    .execute(db)
    .await
    {
        tracing::error!(error = %e, endpoint_id = %ep.id, "failed to create delivery log entry");
        return;
    }

    // Send the HTTP POST.
    let result = client
        .post(&ep.url)
        .header("Content-Type", "application/json")
        .header("Cream-Signature", &signature)
        .header("Cream-Delivery-Id", delivery_id.to_string())
        .body(body)
        .send()
        .await;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16() as i16;
            let resp_body = resp
                .text()
                .await
                .unwrap_or_default();
            let truncated = if resp_body.len() > MAX_RESPONSE_BODY_LEN {
                &resp_body[..MAX_RESPONSE_BODY_LEN]
            } else {
                &resp_body
            };

            if (200..300).contains(&(status as u16)) {
                // Success.
                mark_delivered(db, delivery_id, status, truncated).await;
            } else {
                // Non-2xx — schedule retry.
                mark_failed(db, delivery_id, status, truncated, 1, max_attempts).await;
            }
        }
        Err(e) => {
            let err_msg = format!("connection error: {e}");
            mark_failed(db, delivery_id, 0, &err_msg, 1, max_attempts).await;
        }
    }
}

/// Re-attempt a previously failed delivery.
async fn retry_delivery(
    db: &sqlx::PgPool,
    client: &reqwest::Client,
    row: RetryRow,
) {
    let body = match serde_json::to_vec(&serde_json::json!({
        "event_type": row.event_type,
        "payload": row.payload,
        "timestamp": Utc::now().to_rfc3339(),
    })) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, delivery_id = %row.id, "failed to serialize retry body");
            return;
        }
    };

    // Re-sign with the current timestamp (receiver should verify freshness).
    let timestamp = Utc::now().timestamp();
    let signature = sign_payload(row.secret_hash.as_bytes(), timestamp, &body);

    let result = client
        .post(&row.url)
        .header("Content-Type", "application/json")
        .header("Cream-Signature", &signature)
        .header("Cream-Delivery-Id", row.id.to_string())
        .body(body)
        .send()
        .await;

    let new_attempt = row.attempt + 1;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16() as i16;
            let resp_body = resp.text().await.unwrap_or_default();
            let truncated = if resp_body.len() > MAX_RESPONSE_BODY_LEN {
                &resp_body[..MAX_RESPONSE_BODY_LEN]
            } else {
                &resp_body
            };

            if (200..300).contains(&(status as u16)) {
                mark_delivered(db, row.id, status, truncated).await;
            } else {
                mark_failed(db, row.id, status, truncated, new_attempt, row.max_attempts).await;
            }
        }
        Err(e) => {
            let err_msg = format!("connection error: {e}");
            mark_failed(db, row.id, 0, &err_msg, new_attempt, row.max_attempts).await;
        }
    }
}

/// Mark a delivery as successfully delivered.
async fn mark_delivered(db: &sqlx::PgPool, delivery_id: Uuid, http_status: i16, response_body: &str) {
    if let Err(e) = sqlx::query(
        "UPDATE webhook_delivery_log
         SET status = 'delivered', http_status = $2, response_body = $3,
             delivered_at = now(), last_attempted_at = now(), attempt = attempt + 1
         WHERE id = $1",
    )
    .bind(delivery_id)
    .bind(http_status)
    .bind(response_body)
    .execute(db)
    .await
    {
        tracing::error!(error = %e, delivery_id = %delivery_id, "failed to mark delivery as delivered");
    }
}

/// Mark a delivery as failed and schedule retry (or mark exhausted).
async fn mark_failed(
    db: &sqlx::PgPool,
    delivery_id: Uuid,
    http_status: i16,
    response_body: &str,
    new_attempt: i16,
    max_attempts: i16,
) {
    let (status, next_retry) = if new_attempt >= max_attempts {
        ("exhausted".to_string(), None)
    } else {
        let backoff_idx = (new_attempt as usize).min(RETRY_BACKOFF_SECS.len() - 1);
        let backoff_secs = RETRY_BACKOFF_SECS[backoff_idx];
        let next = Utc::now() + chrono::Duration::seconds(backoff_secs);
        ("failed".to_string(), Some(next))
    };

    if let Err(e) = sqlx::query(
        "UPDATE webhook_delivery_log
         SET status = $2, http_status = $3, response_body = $4,
             attempt = $5, next_retry_at = $6, last_attempted_at = now()
         WHERE id = $1",
    )
    .bind(delivery_id)
    .bind(&status)
    .bind(http_status)
    .bind(response_body)
    .bind(new_attempt)
    .bind(next_retry)
    .execute(db)
    .await
    {
        tracing::error!(error = %e, delivery_id = %delivery_id, "failed to update delivery log after failure");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Signing tests ---

    #[test]
    fn sign_payload_produces_expected_format() {
        let sig = sign_payload(b"test_secret", 1713100000, b"{}");
        assert!(sig.starts_with("t=1713100000,v1="));
        // v1= followed by 64 hex chars (SHA256)
        let hex_part = sig.strip_prefix("t=1713100000,v1=").unwrap();
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_payload_deterministic() {
        let sig1 = sign_payload(b"secret", 12345, b"body");
        let sig2 = sign_payload(b"secret", 12345, b"body");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_payload_different_secrets_differ() {
        let sig1 = sign_payload(b"secret_a", 12345, b"body");
        let sig2 = sign_payload(b"secret_b", 12345, b"body");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn sign_payload_different_bodies_differ() {
        let sig1 = sign_payload(b"secret", 12345, b"body_a");
        let sig2 = sign_payload(b"secret", 12345, b"body_b");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn sign_payload_different_timestamps_differ() {
        let sig1 = sign_payload(b"secret", 12345, b"body");
        let sig2 = sign_payload(b"secret", 12346, b"body");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn verify_signature_valid() {
        let now = Utc::now().timestamp();
        let sig = sign_payload(b"secret", now, b"hello world");
        assert!(verify_signature(b"secret", &sig, b"hello world"));
    }

    #[test]
    fn verify_signature_wrong_secret() {
        let now = Utc::now().timestamp();
        let sig = sign_payload(b"secret", now, b"hello world");
        assert!(!verify_signature(b"wrong", &sig, b"hello world"));
    }

    #[test]
    fn verify_signature_wrong_body() {
        let now = Utc::now().timestamp();
        let sig = sign_payload(b"secret", now, b"hello world");
        assert!(!verify_signature(b"secret", &sig, b"tampered"));
    }

    #[test]
    fn verify_signature_malformed() {
        assert!(!verify_signature(b"secret", "garbage", b"body"));
        assert!(!verify_signature(b"secret", "", b"body"));
        assert!(!verify_signature(b"secret", "t=abc,v1=def", b"body"));
    }

    #[test]
    fn verify_signature_rejects_old_timestamp() {
        // 10 minutes ago — beyond the 5-minute window.
        let old_ts = Utc::now().timestamp() - 600;
        let sig = sign_payload(b"secret", old_ts, b"hello world");
        assert!(!verify_signature(b"secret", &sig, b"hello world"));
    }

    #[test]
    fn verify_signature_rejects_future_timestamp() {
        // 10 minutes in the future.
        let future_ts = Utc::now().timestamp() + 600;
        let sig = sign_payload(b"secret", future_ts, b"hello world");
        assert!(!verify_signature(b"secret", &sig, b"hello world"));
    }

    // --- Event matching tests ---

    #[test]
    fn event_matches_wildcard() {
        let events = serde_json::json!(["*"]);
        assert!(event_matches(&events, "payment.settled"));
        assert!(event_matches(&events, "anything.at.all"));
    }

    #[test]
    fn event_matches_exact() {
        let events = serde_json::json!(["payment.settled", "payment.failed"]);
        assert!(event_matches(&events, "payment.settled"));
        assert!(event_matches(&events, "payment.failed"));
        assert!(!event_matches(&events, "payment.blocked"));
    }

    #[test]
    fn event_matches_prefix() {
        let events = serde_json::json!(["payment.*"]);
        assert!(event_matches(&events, "payment.settled"));
        assert!(event_matches(&events, "payment.failed"));
        assert!(!event_matches(&events, "agent.created"));
    }

    #[test]
    fn event_matches_empty_array() {
        let events = serde_json::json!([]);
        assert!(!event_matches(&events, "payment.settled"));
    }

    #[test]
    fn event_matches_malformed_rejects_all() {
        let events = serde_json::json!("not an array");
        assert!(!event_matches(&events, "anything"), "malformed events filter must fail closed");
        let events_null = serde_json::json!(null);
        assert!(!event_matches(&events_null, "payment.settled"), "null events filter must fail closed");
    }

    // --- Retry backoff tests ---

    #[test]
    fn retry_backoff_schedule() {
        assert_eq!(RETRY_BACKOFF_SECS[0], 5);
        assert_eq!(RETRY_BACKOFF_SECS[1], 30);
        assert_eq!(RETRY_BACKOFF_SECS[2], 120);
        assert_eq!(RETRY_BACKOFF_SECS[3], 900);
        assert_eq!(RETRY_BACKOFF_SECS[4], 3600);
    }
}
