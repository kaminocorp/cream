use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;

use crate::state::AppState;
use crate::notifications::slack::verify_slack_signature;

/// `POST /v1/integrations/slack/callback`
///
/// Handles interactive message actions from Slack (approve/reject buttons).
/// Verifies the Slack signing secret before processing any action.
///
/// Slack sends the body as `application/x-www-form-urlencoded` with a
/// `payload` field containing JSON.
pub async fn slack_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // --- Verify Slack signature ---
    let signing_secret = match state.config.slack_signing_secret.as_deref() {
        Some(s) => s,
        None => {
            tracing::warn!("slack callback received but SLACK_SIGNING_SECRET is not configured");
            return (StatusCode::FORBIDDEN, "slack integration not configured").into_response();
        }
    };

    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let signature = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_slack_signature(signing_secret, timestamp, &body, signature) {
        tracing::warn!("slack callback rejected: invalid signature");
        return (StatusCode::UNAUTHORIZED, "invalid signature").into_response();
    }

    // --- Parse the payload ---
    // Slack sends: payload=<url-encoded JSON>
    let body_str = String::from_utf8_lossy(&body);
    let payload_json = body_str
        .strip_prefix("payload=")
        .and_then(|s| urlencoding::decode(s).ok())
        .and_then(|decoded| serde_json::from_str::<serde_json::Value>(&decoded).ok());

    let payload = match payload_json {
        Some(p) => p,
        None => {
            tracing::warn!("slack callback: failed to parse payload");
            return (StatusCode::BAD_REQUEST, "invalid payload").into_response();
        }
    };

    // Extract the action (approve or reject) and payment_id from the interactive message.
    let actions = match payload.get("actions").and_then(|a| a.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => {
            tracing::debug!("slack callback: no actions in payload");
            return (StatusCode::OK, "").into_response();
        }
    };

    let action = &actions[0];
    let action_id = action.get("action_id").and_then(|v| v.as_str()).unwrap_or("");
    let payment_id_str = action.get("value").and_then(|v| v.as_str()).unwrap_or("");

    if payment_id_str.is_empty() {
        tracing::warn!("slack callback: missing payment_id in action value");
        return (StatusCode::BAD_REQUEST, "missing payment_id").into_response();
    }

    // Extract who clicked the button (for audit attribution).
    let slack_user = payload
        .get("user")
        .and_then(|u| u.get("username"))
        .and_then(|v| v.as_str())
        .unwrap_or("slack-user");
    let reviewer_id = format!("slack:{}", slack_user);

    // Parse payment ID.
    let payment_id = match uuid::Uuid::parse_str(payment_id_str) {
        Ok(uuid) => cream_models::prelude::PaymentId::from_uuid(uuid),
        Err(_) => {
            tracing::warn!(payment_id = %payment_id_str, "slack callback: invalid payment_id format");
            return (StatusCode::BAD_REQUEST, "invalid payment_id").into_response();
        }
    };

    // Look up the payment.
    let payment = match state.payment_repo.get_payment(&payment_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "payment not found").into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "slack callback: failed to fetch payment");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    // Only process if payment is still pending approval.
    if payment.status() != cream_models::prelude::PaymentStatus::PendingApproval {
        return (StatusCode::OK, "payment no longer pending approval").into_response();
    }

    match action_id {
        "escalation_approve" => {
            match crate::routes::payments::approve_payment_internal(
                &state,
                payment,
                &reviewer_id,
                Some("approved via Slack"),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(payment_id = %payment_id, reviewer = %reviewer_id, "payment approved via Slack");
                    (StatusCode::OK, "payment approved").into_response()
                }
                Err(e) => {
                    tracing::error!(error = %e, "slack callback: approve failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "approve failed").into_response()
                }
            }
        }
        "escalation_reject" => {
            match crate::routes::payments::reject_payment_internal(
                &state,
                payment,
                &reviewer_id,
                Some("rejected via Slack"),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(payment_id = %payment_id, reviewer = %reviewer_id, "payment rejected via Slack");
                    (StatusCode::OK, "payment rejected").into_response()
                }
                Err(e) => {
                    tracing::error!(error = %e, "slack callback: reject failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "reject failed").into_response()
                }
            }
        }
        other => {
            tracing::debug!(action_id = %other, "slack callback: unrecognized action");
            (StatusCode::OK, "").into_response()
        }
    }
}
