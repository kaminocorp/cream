use async_trait::async_trait;

use super::{EscalationNotification, NotificationSender, ReminderKind, ReminderNotification};

// ---------------------------------------------------------------------------
// Slack notifier — sends Block Kit messages via chat.postMessage
// ---------------------------------------------------------------------------

/// Configuration for the Slack integration.
#[derive(Debug, Clone)]
pub struct SlackConfig {
    /// Bot OAuth token (xoxb-...).
    pub bot_token: String,
    /// Default channel ID to post escalation messages to.
    pub channel_id: String,
    /// Signing secret for verifying inbound Slack callbacks.
    pub signing_secret: String,
}

impl SlackConfig {
    /// Try to load from environment variables. Returns `None` if any required
    /// var is missing (Slack integration is optional).
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("SLACK_BOT_TOKEN").ok()?;
        let channel_id = std::env::var("SLACK_CHANNEL_ID").ok()?;
        let signing_secret = std::env::var("SLACK_SIGNING_SECRET").ok()?;

        if bot_token.is_empty() || channel_id.is_empty() || signing_secret.is_empty() {
            return None;
        }

        Some(Self {
            bot_token,
            channel_id,
            signing_secret,
        })
    }
}

/// Sends escalation notifications to Slack using the Block Kit API.
pub struct SlackNotifier {
    config: SlackConfig,
    client: reqwest::Client,
}

impl SlackNotifier {
    pub fn new(config: SlackConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    /// Build the Block Kit message payload for an escalation.
    pub fn build_message_payload(
        notification: &EscalationNotification,
        channel_id: &str,
    ) -> serde_json::Value {
        let payment_id = notification.payment_id.to_string();

        serde_json::json!({
            "channel": channel_id,
            "text": format!(
                "Payment escalation: {} {:?} to {} (agent: {})",
                notification.amount, notification.currency, notification.recipient, notification.agent_name
            ),
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": "Payment Escalation"
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Amount:*\n{} {:?}", notification.amount, notification.currency)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Recipient:*\n{}", notification.recipient)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Agent:*\n{}", notification.agent_name)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Timeout:*\n{} minutes", notification.timeout_minutes)
                        }
                    ]
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("*Justification:*\n> {}", notification.justification_summary)
                    }
                },
                {
                    "type": "actions",
                    "elements": [
                        {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": "Approve"
                            },
                            "style": "primary",
                            "action_id": "escalation_approve",
                            "value": payment_id.clone()
                        },
                        {
                            "type": "button",
                            "text": {
                                "type": "plain_text",
                                "text": "Reject"
                            },
                            "style": "danger",
                            "action_id": "escalation_reject",
                            "value": payment_id
                        }
                    ]
                },
                {
                    "type": "context",
                    "elements": [
                        {
                            "type": "mrkdwn",
                            "text": format!("Payment ID: `{}`", notification.payment_id)
                        }
                    ]
                }
            ]
        })
    }
}

#[async_trait]
impl NotificationSender for SlackNotifier {
    async fn send_escalation(&self, notification: &EscalationNotification) -> Result<(), String> {
        let payload = Self::build_message_payload(notification, &self.config.channel_id);

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.config.bot_token)
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Slack returns 200 even for API errors — check the `ok` field.
                    match resp.json::<serde_json::Value>().await {
                        Ok(body) => {
                            if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                                tracing::info!(
                                    payment_id = %notification.payment_id,
                                    "slack escalation notification sent"
                                );
                                Ok(())
                            } else {
                                let error = body
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                tracing::warn!(
                                    payment_id = %notification.payment_id,
                                    slack_error = %error,
                                    "slack API returned error (non-blocking)"
                                );
                                // Swallow — notification failure is non-blocking.
                                Ok(())
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                payment_id = %notification.payment_id,
                                error = %e,
                                "failed to parse slack response (non-blocking)"
                            );
                            Ok(())
                        }
                    }
                } else {
                    tracing::warn!(
                        payment_id = %notification.payment_id,
                        status = %resp.status(),
                        "slack HTTP error (non-blocking)"
                    );
                    Ok(())
                }
            }
            Err(e) => {
                tracing::warn!(
                    payment_id = %notification.payment_id,
                    error = %e,
                    "slack request failed (non-blocking)"
                );
                // Swallow network errors — dashboard is always the fallback.
                Ok(())
            }
        }
    }

    async fn send_reminder(&self, notification: &ReminderNotification) -> Result<(), String> {
        let emoji = match notification.kind {
            ReminderKind::Reminder => ":warning:",
            ReminderKind::Timeout => ":no_entry:",
        };
        let title = match notification.kind {
            ReminderKind::Reminder => "Escalation Reminder",
            ReminderKind::Timeout => "Escalation Timed Out",
        };
        let text = match notification.kind {
            ReminderKind::Reminder => format!(
                "{} *{}*: Payment {} {:?} to {} has *{} minutes remaining* before auto-block.",
                emoji, title, notification.amount, notification.currency,
                notification.recipient, notification.minutes_remaining
            ),
            ReminderKind::Timeout => format!(
                "{} *{}*: Payment {} {:?} to {} has been *automatically blocked* (timeout expired).",
                emoji, title, notification.amount, notification.currency, notification.recipient
            ),
        };

        let payload = serde_json::json!({
            "channel": self.config.channel_id,
            "text": text,
            "blocks": [
                {
                    "type": "section",
                    "text": { "type": "mrkdwn", "text": text }
                },
                {
                    "type": "context",
                    "elements": [
                        { "type": "mrkdwn", "text": format!("Agent: {} · Payment: `{}`", notification.agent_name, notification.payment_id) }
                    ]
                }
            ]
        });

        match self.client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.config.bot_token)
            .json(&payload)
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!(
                    payment_id = %notification.payment_id,
                    kind = ?notification.kind,
                    "slack reminder sent"
                );
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    payment_id = %notification.payment_id,
                    error = %e,
                    "slack reminder failed (non-blocking)"
                );
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Slack callback signature verification
// ---------------------------------------------------------------------------

/// Verify a Slack request signature per
/// <https://api.slack.com/authentication/verifying-requests-from-slack>.
///
/// `signing_secret`: the app's signing secret (not the bot token).
/// `timestamp`: the `X-Slack-Request-Timestamp` header value.
/// `body`: the raw request body bytes.
/// `signature`: the `X-Slack-Signature` header value (e.g. `v0=abc123...`).
pub fn verify_slack_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Guard against replay attacks — reject if timestamp is older than 5 minutes.
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        if (now - ts).unsigned_abs() > 300 {
            tracing::debug!("slack signature rejected: timestamp too old");
            return false;
        }
    } else {
        return false;
    }

    let sig_basestring = format!("v0:{}:{}", timestamp, String::from_utf8_lossy(body));

    let mut mac =
        match Hmac::<Sha256>::new_from_slice(signing_secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
    mac.update(sig_basestring.as_bytes());
    let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

    // Constant-time comparison.
    use subtle::ConstantTimeEq;
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cream_models::prelude::*;

    fn test_notification() -> EscalationNotification {
        EscalationNotification {
            payment_id: PaymentId::new(),
            amount: rust_decimal::Decimal::new(15000, 2),
            currency: Currency::USD,
            recipient: "merchant@example.com".to_string(),
            agent_name: "procurement-agent".to_string(),
            agent_id: AgentId::new(),
            justification_summary: "Purchasing 10 API credits for batch #42".to_string(),
            timeout_minutes: 30,
            dashboard_url: None,
        }
    }

    #[test]
    fn block_kit_payload_has_required_fields() {
        let notification = test_notification();
        let payload = SlackNotifier::build_message_payload(&notification, "C12345");

        // Channel is set correctly.
        assert_eq!(payload["channel"], "C12345");

        // Fallback text is present (for notifications/accessibility).
        assert!(payload["text"].as_str().unwrap().contains("150"));

        // Blocks array exists with header, section fields, justification, actions, context.
        let blocks = payload["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 5);

        // Header block
        assert_eq!(blocks[0]["type"], "header");

        // Actions block has approve and reject buttons.
        let actions = blocks[3]["elements"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0]["action_id"], "escalation_approve");
        assert_eq!(actions[1]["action_id"], "escalation_reject");

        // Both buttons carry the payment ID as value.
        let pid = notification.payment_id.to_string();
        assert_eq!(actions[0]["value"], pid);
        assert_eq!(actions[1]["value"], pid);
    }

    #[test]
    fn block_kit_payload_shows_amount_and_currency() {
        let notification = test_notification();
        let payload = SlackNotifier::build_message_payload(&notification, "C12345");

        let fields = payload["blocks"][1]["fields"].as_array().unwrap();
        let amount_field = fields[0]["text"].as_str().unwrap();
        assert!(amount_field.contains("150"));
        assert!(amount_field.contains("USD"));
    }

    #[test]
    fn verify_signature_valid() {
        let secret = "8f742231b10e8888abcd99yyyzzz85a5";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = b"payload={\"token\":\"test\"}";

        // Compute the expected signature.
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let sig_basestring = format!("v0:{}:{}", timestamp, String::from_utf8_lossy(body));
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(sig_basestring.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_slack_signature(secret, timestamp, body, &signature));
    }

    #[test]
    fn verify_signature_wrong_secret_rejected() {
        let secret = "8f742231b10e8888abcd99yyyzzz85a5";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = b"payload={\"token\":\"test\"}";

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let sig_basestring = format!("v0:{}:{}", timestamp, String::from_utf8_lossy(body));
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(sig_basestring.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        // Different secret → should fail.
        assert!(!verify_slack_signature("wrong_secret_0000000000000000000", timestamp, body, &signature));
    }

    #[test]
    fn verify_signature_old_timestamp_rejected() {
        let secret = "8f742231b10e8888abcd99yyyzzz85a5";
        // 10 minutes ago — beyond the 5-minute replay window.
        let old_timestamp = (chrono::Utc::now().timestamp() - 600).to_string();
        let body = b"payload={\"token\":\"test\"}";

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let sig_basestring = format!("v0:{}:{}", old_timestamp, String::from_utf8_lossy(body));
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(sig_basestring.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        assert!(!verify_slack_signature(secret, &old_timestamp, body, &signature));
    }

    #[test]
    fn verify_signature_malformed_rejected() {
        assert!(!verify_slack_signature("secret", "not-a-number", b"body", "v0=abc"));
        assert!(!verify_slack_signature("secret", "12345", b"body", "garbage"));
    }
}
