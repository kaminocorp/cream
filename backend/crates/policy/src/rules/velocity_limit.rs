use chrono::Duration;
use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks or escalates when the agent exceeds N transactions within a time window.
///
/// The condition's FieldCheck should specify `value` as an object with
/// `max_count` (integer) and `window_minutes` (integer). The evaluator counts
/// all payments that count toward spend (settled + in-flight) within the window,
/// excluding failed/blocked/rejected.
pub struct VelocityLimitEvaluator;

impl RuleEvaluator for VelocityLimitEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        // Extract parameters from the rule condition's value field.
        // Convention: the FieldCheck value is {"max_count": N, "window_minutes": M}
        let (max_count, window_minutes) = match extract_params(rule) {
            Some(params) => params,
            None => {
                tracing::warn!(
                    rule_id = %rule.id,
                    "velocity_limit rule has missing or invalid config (expected max_count + window_minutes), skipping"
                );
                return RuleResult::Pass;
            }
        };

        let cutoff = ctx.current_time - Duration::minutes(window_minutes);
        let count = ctx
            .recent_payments
            .iter()
            .filter(|p| p.created_at >= cutoff && p.status.counts_toward_spend())
            .count()
            + 1; // +1 for the current request

        if count as i64 > max_count {
            RuleResult::Triggered(rule.action)
        } else {
            RuleResult::Pass
        }
    }
}

fn extract_params(rule: &PolicyRule) -> Option<(i64, i64)> {
    // Walk the condition tree to find a FieldCheck with "velocity" field
    extract_from_condition(&rule.condition)
}

fn extract_from_condition(
    condition: &cream_models::prelude::PolicyCondition,
) -> Option<(i64, i64)> {
    use cream_models::prelude::PolicyCondition;
    match condition {
        PolicyCondition::FieldCheck(check) if check.field == "velocity" => {
            let max_count = check.value.get("max_count")?.as_i64()?;
            let window_minutes = check.value.get("window_minutes")?.as_i64()?;
            if max_count <= 0 || window_minutes <= 0 {
                tracing::warn!(
                    max_count,
                    window_minutes,
                    "velocity_limit config has non-positive values, rule will be skipped"
                );
                return None;
            }
            Some((max_count, window_minutes))
        }
        PolicyCondition::All(children) | PolicyCondition::Any(children) => {
            children.iter().find_map(extract_from_condition)
        }
        PolicyCondition::Not(inner) => extract_from_condition(inner),
        _ => None,
    }
}
