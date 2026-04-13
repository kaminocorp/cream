pub mod email;
pub mod slack;

use async_trait::async_trait;
use cream_models::prelude::*;

// ---------------------------------------------------------------------------
// Escalation context — data the notification channel needs to render a message
// ---------------------------------------------------------------------------

/// All the information a notification channel needs to render an escalation
/// message. Cheaply cloneable so dispatch can fire notifications without
/// blocking the orchestrator.
#[derive(Debug, Clone)]
pub struct EscalationNotification {
    pub payment_id: PaymentId,
    pub amount: rust_decimal::Decimal,
    pub currency: Currency,
    pub recipient: String,
    pub agent_name: String,
    pub agent_id: AgentId,
    pub justification_summary: String,
    pub timeout_minutes: u32,
    /// Deep link to the payment in the operator dashboard.
    pub dashboard_url: Option<String>,
}

/// Context for reminder and timeout notifications. Carries the same payment
/// info as `EscalationNotification` plus additional context about remaining time.
#[derive(Debug, Clone)]
pub struct ReminderNotification {
    pub payment_id: PaymentId,
    pub amount: rust_decimal::Decimal,
    pub currency: Currency,
    pub recipient: String,
    pub agent_name: String,
    pub minutes_remaining: u32,
    /// "reminder" or "timeout" — determines message tone.
    pub kind: ReminderKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderKind {
    /// 50% timeout reminder — nudge the reviewer.
    Reminder,
    /// Payment has timed out and been blocked.
    Timeout,
}

// ---------------------------------------------------------------------------
// NotificationSender trait
// ---------------------------------------------------------------------------

/// Async trait for sending escalation notifications to an external channel
/// (Slack, email, webhook, etc.).
///
/// Implementations MUST NOT return errors that should block the payment
/// pipeline. All failures should be logged internally and swallowed —
/// the caller treats all methods as fire-and-forget.
#[async_trait]
pub trait NotificationSender: Send + Sync + 'static {
    /// Send an escalation notification (payment just entered PendingApproval).
    async fn send_escalation(&self, notification: &EscalationNotification) -> Result<(), String>;

    /// Send a reminder or timeout notification. Default implementation logs
    /// and returns Ok (channels that don't support reminders can skip it).
    async fn send_reminder(&self, notification: &ReminderNotification) -> Result<(), String> {
        tracing::debug!(
            payment_id = %notification.payment_id,
            kind = ?notification.kind,
            "send_reminder not implemented for this channel, skipping"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// NoopNotifier — used when no notification channel is configured
// ---------------------------------------------------------------------------

/// Does nothing. Used as the default when no Slack/email config is provided.
pub struct NoopNotifier;

#[async_trait]
impl NotificationSender for NoopNotifier {
    async fn send_escalation(&self, _notification: &EscalationNotification) -> Result<(), String> {
        tracing::debug!("no notification channel configured, skipping escalation notification");
        Ok(())
    }

    async fn send_reminder(&self, _notification: &ReminderNotification) -> Result<(), String> {
        tracing::debug!("no notification channel configured, skipping reminder");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CompositeNotifier — dispatches to multiple channels
// ---------------------------------------------------------------------------

/// Sends escalation notifications to all registered channels. Failures in
/// one channel do not affect others.
pub struct CompositeNotifier {
    senders: Vec<Box<dyn NotificationSender>>,
}

impl CompositeNotifier {
    pub fn new(senders: Vec<Box<dyn NotificationSender>>) -> Self {
        Self { senders }
    }
}

#[async_trait]
impl NotificationSender for CompositeNotifier {
    async fn send_escalation(&self, notification: &EscalationNotification) -> Result<(), String> {
        for sender in &self.senders {
            if let Err(e) = sender.send_escalation(notification).await {
                tracing::warn!(error = %e, "notification channel failed (non-blocking)");
            }
        }
        Ok(())
    }

    async fn send_reminder(&self, notification: &ReminderNotification) -> Result<(), String> {
        for sender in &self.senders {
            if let Err(e) = sender.send_reminder(notification).await {
                tracing::warn!(error = %e, "reminder channel failed (non-blocking)");
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingSender {
        escalations: Arc<AtomicUsize>,
        reminders: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl NotificationSender for CountingSender {
        async fn send_escalation(
            &self,
            _notification: &EscalationNotification,
        ) -> Result<(), String> {
            self.escalations.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn send_reminder(
            &self,
            _notification: &ReminderNotification,
        ) -> Result<(), String> {
            self.reminders.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FailingSender;

    #[async_trait]
    impl NotificationSender for FailingSender {
        async fn send_escalation(
            &self,
            _notification: &EscalationNotification,
        ) -> Result<(), String> {
            Err("simulated failure".to_string())
        }

        async fn send_reminder(
            &self,
            _notification: &ReminderNotification,
        ) -> Result<(), String> {
            Err("simulated reminder failure".to_string())
        }
    }

    fn test_notification() -> EscalationNotification {
        EscalationNotification {
            payment_id: PaymentId::new(),
            amount: rust_decimal::Decimal::new(15000, 2),
            currency: Currency::USD,
            recipient: "merchant@example.com".to_string(),
            agent_name: "test-agent".to_string(),
            agent_id: AgentId::new(),
            justification_summary: "Purchasing API credits for batch processing".to_string(),
            timeout_minutes: 30,
            dashboard_url: Some("https://dashboard.example.com/escalations".to_string()),
        }
    }

    fn test_reminder() -> ReminderNotification {
        ReminderNotification {
            payment_id: PaymentId::new(),
            amount: rust_decimal::Decimal::new(15000, 2),
            currency: Currency::USD,
            recipient: "merchant@example.com".to_string(),
            agent_name: "test-agent".to_string(),
            minutes_remaining: 15,
            kind: ReminderKind::Reminder,
        }
    }

    #[tokio::test]
    async fn noop_notifier_succeeds() {
        let noop = NoopNotifier;
        assert!(noop.send_escalation(&test_notification()).await.is_ok());
        assert!(noop.send_reminder(&test_reminder()).await.is_ok());
    }

    #[tokio::test]
    async fn composite_dispatches_to_all_channels() {
        let esc = Arc::new(AtomicUsize::new(0));
        let rem = Arc::new(AtomicUsize::new(0));
        let composite = CompositeNotifier::new(vec![
            Box::new(CountingSender {
                escalations: esc.clone(),
                reminders: rem.clone(),
            }),
            Box::new(CountingSender {
                escalations: esc.clone(),
                reminders: rem.clone(),
            }),
        ]);
        composite.send_escalation(&test_notification()).await.unwrap();
        composite.send_reminder(&test_reminder()).await.unwrap();
        assert_eq!(esc.load(Ordering::SeqCst), 2);
        assert_eq!(rem.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn composite_continues_after_channel_failure() {
        let esc = Arc::new(AtomicUsize::new(0));
        let rem = Arc::new(AtomicUsize::new(0));
        let composite = CompositeNotifier::new(vec![
            Box::new(FailingSender),
            Box::new(CountingSender {
                escalations: esc.clone(),
                reminders: rem.clone(),
            }),
        ]);
        assert!(composite.send_escalation(&test_notification()).await.is_ok());
        assert!(composite.send_reminder(&test_reminder()).await.is_ok());
        assert_eq!(esc.load(Ordering::SeqCst), 1);
        assert_eq!(rem.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn reminder_kind_variants_distinguishable() {
        let reminder = ReminderNotification {
            kind: ReminderKind::Reminder,
            ..test_reminder()
        };
        assert_eq!(reminder.kind, ReminderKind::Reminder);

        let timeout = ReminderNotification {
            kind: ReminderKind::Timeout,
            ..test_reminder()
        };
        assert_eq!(timeout.kind, ReminderKind::Timeout);
        assert_ne!(reminder.kind, timeout.kind);
    }
}
