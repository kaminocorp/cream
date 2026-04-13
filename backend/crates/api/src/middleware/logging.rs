//! PII redaction middleware for request/response body logging (Phase 17-A).
//!
//! When `LOG_BODIES=true`, request and response bodies are logged at DEBUG level
//! with sensitive fields replaced by `"[REDACTED]"`. When `LOG_BODIES` is not
//! set (the default), this layer is a no-op pass-through.
//!
//! Sensitive field names (case-insensitive): `password`, `api_key`, `secret`,
//! `refresh_token`, `api_key_hash`, `token`.

use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use bytes::Bytes;
use http_body_util::BodyExt;

/// Fields whose values are replaced with `"[REDACTED]"` in logged bodies.
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "api_key",
    "secret",
    "refresh_token",
    "api_key_hash",
    "token",
];

/// Axum middleware that logs request and response bodies with PII redacted.
///
/// Only active when `log_bodies` is true in the config (checked by the caller
/// before adding this layer). When active, it buffers the request body, logs
/// the redacted version at DEBUG level, then forwards the original body.
pub async fn log_bodies_with_redaction(
    request: Request<Body>,
    next: Next,
) -> Response {
    // --- Log request body ---
    let (parts, body) = request.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            // Can't read body — forward an empty one and skip logging.
            let request = Request::from_parts(parts, Body::empty());
            return next.run(request).await;
        }
    };

    if !body_bytes.is_empty() {
        let redacted = redact_json_bytes(&body_bytes);
        tracing::debug!(
            body = %redacted,
            direction = "request",
            "request body (PII redacted)"
        );
    }

    let request = Request::from_parts(parts, Body::from(body_bytes));
    let response = next.run(request).await;

    // --- Log response body ---
    let (resp_parts, resp_body) = response.into_parts();
    let resp_bytes = match resp_body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return Response::from_parts(resp_parts, Body::empty());
        }
    };

    if !resp_bytes.is_empty() {
        let redacted = redact_json_bytes(&resp_bytes);
        tracing::debug!(
            body = %redacted,
            direction = "response",
            "response body (PII redacted)"
        );
    }

    Response::from_parts(resp_parts, Body::from(resp_bytes))
}

/// Attempt to parse bytes as JSON, redact sensitive fields, and return the
/// redacted string. If the bytes are not valid JSON, return a placeholder.
fn redact_json_bytes(bytes: &Bytes) -> String {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return "[non-JSON body]".to_string();
    };
    redact_value(&mut value);
    // Compact single-line output for log lines.
    serde_json::to_string(&value).unwrap_or_else(|_| "[redaction failed]".to_string())
}

/// Recursively walk a JSON value and replace sensitive field values with
/// `"[REDACTED]"`.
fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let key_lower = key.to_ascii_lowercase();
                if SENSITIVE_FIELDS.iter().any(|&s| key_lower == s) {
                    *val = serde_json::Value::String("[REDACTED]".to_string());
                } else {
                    redact_value(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_password_field() {
        let mut val = json!({"email": "a@b.com", "password": "hunter2"});
        redact_value(&mut val);
        assert_eq!(val["password"], "[REDACTED]");
        assert_eq!(val["email"], "a@b.com");
    }

    #[test]
    fn redacts_api_key_field() {
        let mut val = json!({"agent_id": "agt_1", "api_key": "crk_secret123"});
        redact_value(&mut val);
        assert_eq!(val["api_key"], "[REDACTED]");
        assert_eq!(val["agent_id"], "agt_1");
    }

    #[test]
    fn redacts_nested_secret() {
        let mut val = json!({"config": {"secret": "abc", "name": "test"}});
        redact_value(&mut val);
        assert_eq!(val["config"]["secret"], "[REDACTED]");
        assert_eq!(val["config"]["name"], "test");
    }

    #[test]
    fn redacts_refresh_token() {
        let mut val = json!({"refresh_token": "rt_xyz", "access_token": "visible"});
        redact_value(&mut val);
        assert_eq!(val["refresh_token"], "[REDACTED]");
        // access_token is not in SENSITIVE_FIELDS — kept as-is.
        assert_eq!(val["access_token"], "visible");
    }

    #[test]
    fn redacts_in_arrays() {
        let mut val = json!([{"password": "p1"}, {"password": "p2"}]);
        redact_value(&mut val);
        assert_eq!(val[0]["password"], "[REDACTED]");
        assert_eq!(val[1]["password"], "[REDACTED]");
    }

    #[test]
    fn case_insensitive_field_matching() {
        let mut val = json!({"Password": "x", "API_KEY": "y", "SECRET": "z"});
        redact_value(&mut val);
        assert_eq!(val["Password"], "[REDACTED]");
        assert_eq!(val["API_KEY"], "[REDACTED]");
        assert_eq!(val["SECRET"], "[REDACTED]");
    }

    #[test]
    fn non_json_bytes_returns_placeholder() {
        let bytes = Bytes::from_static(b"not json");
        assert_eq!(redact_json_bytes(&bytes), "[non-JSON body]");
    }

    #[test]
    fn empty_object_unchanged() {
        let mut val = json!({});
        redact_value(&mut val);
        assert_eq!(val, json!({}));
    }
}
