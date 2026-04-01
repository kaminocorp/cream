use chrono::Duration;
use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks duplicate payments: same amount + same recipient within N minutes.
///
/// Default window is 5 minutes. Configurable via FieldCheck value
/// `{"window_minutes": N}` in the rule condition.
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
            tracing::warn!(
                rule_id = %rule.id,
                window_minutes,
                "duplicate_detection window_minutes must be positive, skipping rule"
            );
            return RuleResult::Pass;
        }

        let cutoff = ctx.current_time - Duration::minutes(window_minutes);

        let is_duplicate = ctx.recent_payments.iter().any(|p| {
            p.created_at >= cutoff
                && p.amount == ctx.request.amount
                && p.currency == ctx.request.currency
                && p.recipient_identifier == ctx.request.recipient.identifier
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
