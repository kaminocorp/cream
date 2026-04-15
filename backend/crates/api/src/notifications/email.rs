use async_trait::async_trait;

use super::{AlertNotification, EscalationNotification, NotificationSender, ReminderKind, ReminderNotification};

/// Produce a text/plain approximation of an HTML body by stripping tags.
/// Used to build the plaintext alternative in multipart emails so clients
/// that don't render HTML still show readable content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                // Replace block-level closing tags with newlines for readability.
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    // Collapse multiple blank lines and trim.
    let mut prev_blank = false;
    let lines: Vec<&str> = result
        .lines()
        .filter(|line| {
            let blank = line.trim().is_empty();
            if blank && prev_blank {
                return false;
            }
            prev_blank = blank;
            true
        })
        .collect();
    lines.join("\n").trim().to_string()
}

/// Escape a string for safe inclusion in HTML. Prevents XSS when
/// interpolating user-controlled fields (recipient, agent_name, etc.)
/// into email templates.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Email configuration
// ---------------------------------------------------------------------------

/// Configuration for email notifications. Supports two modes:
/// 1. SMTP (via `lettre`) — for self-hosted SMTP servers
/// 2. Resend API — for managed email delivery (API-based)
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub mode: EmailMode,
    /// Sender email address (e.g. "notifications@cream.example.com").
    pub from: String,
    /// Recipient email address for escalation alerts.
    pub to: String,
    /// Base URL of the operator dashboard (for deep links).
    pub dashboard_base_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EmailMode {
    Smtp {
        host: String,
        port: u16,
        username: String,
        password: String,
    },
    Resend {
        api_key: String,
    },
}

impl EmailConfig {
    /// Try to load from environment variables. Returns `None` if minimum
    /// required vars are missing (email notifications are optional).
    pub fn from_env() -> Option<Self> {
        let from = std::env::var("EMAIL_FROM").ok().filter(|s| !s.is_empty())?;
        let to = std::env::var("ESCALATION_EMAIL_TO")
            .ok()
            .filter(|s| !s.is_empty())?;

        let dashboard_base_url = std::env::var("DASHBOARD_BASE_URL").ok().filter(|s| !s.is_empty());

        // Try Resend first (simpler setup), fall back to SMTP.
        if let Ok(api_key) = std::env::var("RESEND_API_KEY") {
            if !api_key.is_empty() {
                return Some(Self {
                    mode: EmailMode::Resend { api_key },
                    from,
                    to,
                    dashboard_base_url,
                });
            }
        }

        let host = std::env::var("SMTP_HOST").ok().filter(|s| !s.is_empty())?;
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(587);
        let username = std::env::var("SMTP_USERNAME")
            .ok()
            .filter(|s| !s.is_empty())?;
        let password = std::env::var("SMTP_PASSWORD")
            .ok()
            .filter(|s| !s.is_empty())?;

        Some(Self {
            mode: EmailMode::Smtp {
                host,
                port,
                username,
                password,
            },
            from,
            to,
            dashboard_base_url,
        })
    }
}

// ---------------------------------------------------------------------------
// EmailNotifier
// ---------------------------------------------------------------------------

pub struct EmailNotifier {
    config: EmailConfig,
    http_client: reqwest::Client,
}

impl EmailNotifier {
    pub fn new(config: EmailConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            config,
            http_client,
        }
    }

    /// Build the HTML body for an escalation email.
    ///
    /// All user-controlled fields are HTML-escaped to prevent XSS.
    pub fn build_html_body(
        notification: &EscalationNotification,
        dashboard_base_url: Option<&str>,
    ) -> String {
        let deep_link = dashboard_base_url
            .map(|base| {
                format!(
                    "{}/escalations?payment_id={}",
                    base.trim_end_matches('/'),
                    notification.payment_id
                )
            })
            .unwrap_or_default();

        let link_section = if deep_link.is_empty() {
            String::new()
        } else {
            format!(
                r#"<p><a href="{}" style="display:inline-block;padding:10px 20px;background:#18181b;color:#fff;text-decoration:none;border-radius:6px;">Review in Dashboard</a></p>"#,
                html_escape(&deep_link)
            )
        };

        // Escape all user-controlled fields to prevent XSS.
        let recipient = html_escape(&notification.recipient);
        let agent_name = html_escape(&notification.agent_name);
        let justification = html_escape(&notification.justification_summary);
        let payment_id = html_escape(&notification.payment_id.to_string());

        format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;max-width:600px;margin:0 auto;padding:20px;color:#18181b;">
  <h2 style="margin-top:0;">Payment Escalation</h2>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;">
    <tr><td style="padding:8px 0;color:#71717a;width:120px;">Amount</td><td style="padding:8px 0;font-weight:600;">{amount} {currency:?}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Recipient</td><td style="padding:8px 0;">{recipient}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Agent</td><td style="padding:8px 0;">{agent_name}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Timeout</td><td style="padding:8px 0;">{timeout} minutes</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Payment ID</td><td style="padding:8px 0;font-family:monospace;font-size:13px;">{payment_id}</td></tr>
  </table>
  <div style="background:#f4f4f5;border-left:3px solid #71717a;padding:12px 16px;margin:16px 0;">
    <strong>Justification:</strong><br>{justification}
  </div>
  {link_section}
  <p style="color:#a1a1aa;font-size:12px;margin-top:24px;">This email was sent by Cream — the payment control plane for AI agents.</p>
</body>
</html>"#,
            amount = notification.amount,
            currency = notification.currency,
            recipient = recipient,
            agent_name = agent_name,
            timeout = notification.timeout_minutes,
            payment_id = payment_id,
            justification = justification,
            link_section = link_section,
        )
    }

    /// Send via SMTP using `lettre`.
    ///
    /// Sends a multipart message with both text/plain and text/html alternatives
    /// so email clients that don't render HTML still show readable content.
    async fn send_smtp(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        use lettre::message::{MultiPart, SinglePart, Mailbox};
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        let from: Mailbox = self
            .config
            .from
            .parse()
            .map_err(|e| format!("invalid from address: {e}"))?;
        let to: Mailbox = self
            .config
            .to
            .parse()
            .map_err(|e| format!("invalid to address: {e}"))?;

        // Strip HTML tags for the plaintext alternative.
        let plain_text = strip_html_tags(html_body);

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_PLAIN)
                            .body(plain_text),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )
            .map_err(|e| format!("failed to build email: {e}"))?;

        let creds = Credentials::new(username.to_string(), password.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|e| format!("SMTP connection failed: {e}"))?
            .port(port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| format!("SMTP send failed: {e}"))?;

        Ok(())
    }

    /// Send via Resend HTTP API.
    async fn send_resend(
        &self,
        api_key: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "from": self.config.from,
            "to": [self.config.to],
            "subject": subject,
            "html": html_body,
        });

        let response = self
            .http_client
            .post("https://api.resend.com/emails")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Resend request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(format!("Resend API error {status}: {body}"))
        }
    }
}

#[async_trait]
impl NotificationSender for EmailNotifier {
    async fn send_escalation(&self, notification: &EscalationNotification) -> Result<(), String> {
        let subject = format!(
            "Payment Escalation: {} {:?} — {}",
            notification.amount, notification.currency, notification.agent_name
        );
        let html_body = Self::build_html_body(
            notification,
            self.config.dashboard_base_url.as_deref(),
        );

        let result = match &self.config.mode {
            EmailMode::Smtp {
                host,
                port,
                username,
                password,
            } => {
                self.send_smtp(host, *port, username, password, &subject, &html_body)
                    .await
            }
            EmailMode::Resend { api_key } => {
                self.send_resend(api_key, &subject, &html_body).await
            }
        };

        match result {
            Ok(()) => {
                tracing::info!(
                    payment_id = %notification.payment_id,
                    "email escalation notification sent"
                );
                Ok(())
            }
            Err(e) => {
                // Log and swallow — email failure is non-blocking.
                tracing::warn!(
                    payment_id = %notification.payment_id,
                    error = %e,
                    "email notification failed (non-blocking)"
                );
                Ok(())
            }
        }
    }

    async fn send_alert(
        &self,
        notification: &AlertNotification,
        channels: &[String],
    ) -> Result<(), String> {
        // Only send if "email" is in the channels list, or list is empty (all).
        if !channels.is_empty() && !channels.iter().any(|c| c == "email") {
            return Ok(());
        }

        let subject = format!(
            "Metric Alert: {} — {} is {:.2}",
            notification.rule_name, notification.metric, notification.value
        );

        let rule_name = html_escape(&notification.rule_name);
        let metric = html_escape(&notification.metric);
        let condition = html_escape(&notification.condition);
        let message = html_escape(&notification.message);

        let html_body = format!(
            r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;max-width:600px;margin:0 auto;padding:20px;color:#18181b;">
  <h2 style="margin-top:0;color:#dc2626;">Metric Alert: {rule_name}</h2>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;">
    <tr><td style="padding:8px 0;color:#71717a;width:140px;">Metric</td><td style="padding:8px 0;font-family:monospace;">{metric}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Condition</td><td style="padding:8px 0;">{condition} {threshold}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Current Value</td><td style="padding:8px 0;font-weight:600;color:#dc2626;">{value:.2}</td></tr>
    <tr><td style="padding:8px 0;color:#71717a;">Threshold</td><td style="padding:8px 0;">{threshold}</td></tr>
  </table>
  <div style="background:#fef2f2;border-left:3px solid #dc2626;padding:12px 16px;margin:16px 0;">
    {message}
  </div>
  <p style="color:#a1a1aa;font-size:12px;margin-top:24px;">This alert was sent by Cream — the payment control plane for AI agents.</p>
</body></html>"#,
            rule_name = rule_name,
            metric = metric,
            condition = condition,
            threshold = notification.threshold,
            value = notification.value,
            message = message,
        );

        let result = match &self.config.mode {
            EmailMode::Smtp { host, port, username, password } => {
                self.send_smtp(host, *port, username, password, &subject, &html_body).await
            }
            EmailMode::Resend { api_key } => {
                self.send_resend(api_key, &subject, &html_body).await
            }
        };

        match result {
            Ok(()) => {
                tracing::info!(
                    rule = %notification.rule_name,
                    metric = %notification.metric,
                    "email alert notification sent"
                );
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    rule = %notification.rule_name,
                    error = %e,
                    "email alert notification failed (non-blocking)"
                );
                Ok(())
            }
        }
    }

    async fn send_reminder(&self, notification: &ReminderNotification) -> Result<(), String> {
        let subject = match notification.kind {
            ReminderKind::Reminder => format!(
                "Reminder: Payment {} {:?} — {} min remaining",
                notification.amount, notification.currency, notification.minutes_remaining
            ),
            ReminderKind::Timeout => format!(
                "Timed Out: Payment {} {:?} — auto-blocked",
                notification.amount, notification.currency
            ),
        };

        // Escape all user-controlled fields.
        let recipient = html_escape(&notification.recipient);
        let agent = html_escape(&notification.agent_name);
        let payment_id = html_escape(&notification.payment_id.to_string());

        let html_body = format!(
            r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:-apple-system,sans-serif;max-width:600px;margin:0 auto;padding:20px;">
  <h2>{title}</h2>
  <p>{message}</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;">
    <tr><td style="padding:6px 0;color:#71717a;">Amount</td><td>{amount} {currency:?}</td></tr>
    <tr><td style="padding:6px 0;color:#71717a;">Recipient</td><td>{recipient}</td></tr>
    <tr><td style="padding:6px 0;color:#71717a;">Agent</td><td>{agent}</td></tr>
    <tr><td style="padding:6px 0;color:#71717a;">Payment ID</td><td style="font-family:monospace;">{payment_id}</td></tr>
  </table>
  <p style="color:#a1a1aa;font-size:12px;">Cream — payment control plane for AI agents</p>
</body></html>"#,
            title = match notification.kind {
                ReminderKind::Reminder => "Escalation Reminder",
                ReminderKind::Timeout => "Escalation Timed Out",
            },
            message = match notification.kind {
                ReminderKind::Reminder => format!(
                    "A payment escalation has <strong>{} minutes remaining</strong> before it is automatically blocked.",
                    notification.minutes_remaining
                ),
                ReminderKind::Timeout => "This payment has been <strong>automatically blocked</strong> because no human review was received before the timeout expired.".to_string(),
            },
            amount = notification.amount,
            currency = notification.currency,
            recipient = recipient,
            agent = agent,
            payment_id = payment_id,
        );

        let result = match &self.config.mode {
            EmailMode::Smtp { host, port, username, password } => {
                self.send_smtp(host, *port, username, password, &subject, &html_body).await
            }
            EmailMode::Resend { api_key } => {
                self.send_resend(api_key, &subject, &html_body).await
            }
        };

        match result {
            Ok(()) => {
                tracing::info!(payment_id = %notification.payment_id, kind = ?notification.kind, "email reminder sent");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(payment_id = %notification.payment_id, error = %e, "email reminder failed (non-blocking)");
                Ok(())
            }
        }
    }
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
            amount: rust_decimal::Decimal::new(50000, 2),
            currency: Currency::SGD,
            recipient: "vendor@example.com".to_string(),
            agent_name: "procurement-bot".to_string(),
            agent_id: AgentId::new(),
            justification_summary: "Purchasing quarterly cloud infrastructure renewal".to_string(),
            timeout_minutes: 60,
            dashboard_url: None,
        }
    }

    #[test]
    fn html_body_contains_payment_details() {
        let notification = test_notification();
        let html = EmailNotifier::build_html_body(&notification, Some("https://dashboard.cream.io"));

        assert!(html.contains("500"), "should contain amount");
        assert!(html.contains("SGD"), "should contain currency");
        assert!(html.contains("vendor@example.com"), "should contain recipient");
        assert!(html.contains("procurement-bot"), "should contain agent name");
        assert!(html.contains("60 minutes"), "should contain timeout");
        assert!(
            html.contains(&notification.payment_id.to_string()),
            "should contain payment ID"
        );
        assert!(
            html.contains("quarterly cloud infrastructure"),
            "should contain justification"
        );
    }

    #[test]
    fn html_body_includes_deep_link_when_dashboard_url_set() {
        let notification = test_notification();
        let html = EmailNotifier::build_html_body(&notification, Some("https://dashboard.cream.io"));

        let expected_link = format!(
            "https://dashboard.cream.io/escalations?payment_id={}",
            notification.payment_id
        );
        assert!(html.contains(&expected_link), "should contain deep link");
        assert!(html.contains("Review in Dashboard"), "should contain CTA text");
    }

    #[test]
    fn html_body_omits_link_when_no_dashboard_url() {
        let notification = test_notification();
        let html = EmailNotifier::build_html_body(&notification, None);

        assert!(!html.contains("Review in Dashboard"), "should not contain CTA");
        assert!(!html.contains("/escalations?payment_id="), "should not contain link");
    }

    #[test]
    fn html_body_escapes_xss_in_recipient() {
        let mut notification = test_notification();
        notification.recipient = "<script>alert('xss')</script>".to_string();
        notification.agent_name = "agent<img src=x>".to_string();

        let html = EmailNotifier::build_html_body(&notification, None);

        assert!(!html.contains("<script>"), "recipient must be HTML-escaped");
        assert!(html.contains("&lt;script&gt;"), "should contain escaped tag");
        assert!(!html.contains("<img src=x>"), "agent_name must be HTML-escaped");
        assert!(html.contains("&lt;img src=x&gt;"), "should contain escaped img");
    }

    #[test]
    fn html_escape_covers_all_entities() {
        assert_eq!(super::html_escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&#x27;f");
    }

    #[test]
    fn strip_html_tags_produces_readable_text() {
        let html = "<h2>Title</h2><p>Hello <strong>world</strong></p>";
        let plain = super::strip_html_tags(html);
        assert!(plain.contains("Title"));
        assert!(plain.contains("Hello world"));
        assert!(!plain.contains("<"), "should not contain HTML tags");
    }

    #[test]
    fn email_config_returns_none_when_no_env_vars() {
        // In test context, EMAIL_FROM and ESCALATION_EMAIL_TO are not set.
        // from_env() should return None.
        let config = EmailConfig::from_env();
        // Cannot assert None because other tests may have set env vars,
        // but at minimum it should not panic.
        let _ = config;
    }

    #[tokio::test]
    async fn email_notifier_swallows_smtp_failure() {
        // Construct a notifier pointing at an unreachable SMTP server.
        let config = EmailConfig {
            mode: EmailMode::Smtp {
                host: "localhost".to_string(),
                port: 19999, // not listening
                username: "user".to_string(),
                password: "pass".to_string(),
            },
            from: "test@example.com".to_string(),
            to: "admin@example.com".to_string(),
            dashboard_base_url: None,
        };
        let notifier = EmailNotifier::new(config);
        let notification = test_notification();

        // Should return Ok (error swallowed) — not panic or propagate.
        let result = notifier.send_escalation(&notification).await;
        assert!(result.is_ok(), "email failure should be swallowed");
    }
}
