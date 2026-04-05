use chrono::Duration;
use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks duplicate payments: same amount + same recipient + same currency
/// within N minutes.
///
/// Default window is 5 minutes. Configurable via FieldCheck value
/// `{"window_minutes": N}` in the rule condition.
///
/// **Currency isolation (by design):** Duplicates must match on currency as
/// well as amount and recipient. A $100 USD payment and a $100 SGD payment
/// to the same recipient are not duplicates. See `SpendRateEvaluator` for
/// rationale on per-currency filtering.
pub struct DuplicateDetectionEvaluator;

const DEFAULT_WINDOW_MINUTES: i64 = 5;

impl RuleEvaluator for DuplicateDetectionEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let window_minutes = extract_window(&rule.condition).unwrap_or_else(|| {
            tracing::debug!(
                rule_id = %rule.id,
                default_minutes = DEFAULT_WINDOW_MINUTES,
                "duplicate_detection using default window"
            );
            DEFAULT_WINDOW_MINUTES
        });

        // Guard: non-positive window is a misconfiguration that would create
        // a future cutoff (never matching any past payment), silently disabling
        // the rule. Log and skip instead of silently passing.
        if window_minutes <= 0 {
            tracing::error!(
                rule_id = %rule.id,
                window_minutes,
                "duplicate_detection window_minutes must be positive, \
                 failing safe — treating as triggered to prevent policy bypass"
            );
            return RuleResult::Triggered(rule.action);
        }

        let cutoff = ctx.current_time - Duration::minutes(window_minutes);

        let request_id_lower = ctx.request.recipient.identifier.to_ascii_lowercase();
        // Only consider payments that count toward spend (excludes Failed, Blocked,
        // Rejected, TimedOut). Failed payments should not block legitimate retries —
        // a provider timeout or transient error is the most common reason for a
        // same-amount retry within the duplicate window.
        let is_duplicate = ctx.recent_payments.iter().any(|p| {
            p.status.counts_toward_spend()
                && p.created_at >= cutoff
                && p.amount == ctx.request.amount
                && p.currency == ctx.request.currency
                && p.recipient_identifier.to_ascii_lowercase() == request_id_lower
        });

        if is_duplicate {
            RuleResult::Triggered(rule.action)
        } else {
            RuleResult::Pass
        }
    }
}

fn extract_window(condition: &cream_models::prelude::PolicyCondition) -> Option<i64> {
    use cream_models::prelude::PolicyCondition;
    match condition {
        PolicyCondition::FieldCheck(check) if check.field == "duplicate" => {
            check.value.get("window_minutes")?.as_i64()
        }
        PolicyCondition::All(children) | PolicyCondition::Any(children) => {
            children.iter().find_map(extract_window)
        }
        PolicyCondition::Not(inner) => extract_window(inner),
        _ => None,
    }
}
