use cream_models::prelude::{PolicyAction, PolicyRule};

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Escalates payments that exceed the agent profile's escalation threshold.
///
/// Unlike `AmountCapEvaluator` (which uses `max_per_transaction` and the rule's
/// own action — typically Block), this evaluator reads the profile's
/// `escalation_threshold` and always returns `Escalate` when triggered,
/// requiring human approval for high-value payments.
///
/// If the profile has no escalation_threshold set, the rule passes silently.
pub struct EscalationThresholdEvaluator;

impl RuleEvaluator for EscalationThresholdEvaluator {
    fn evaluate(&self, _rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let threshold = match ctx.profile.escalation_threshold {
            Some(t) => t,
            None => return RuleResult::Pass,
        };

        if ctx.request.amount > threshold {
            RuleResult::Triggered(PolicyAction::Escalate)
        } else {
            RuleResult::Pass
        }
    }
}
