use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks or escalates payments that exceed the agent's per-transaction limit.
pub struct AmountCapEvaluator;

impl RuleEvaluator for AmountCapEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        if ctx.request.amount > ctx.profile.max_per_transaction {
            RuleResult::Triggered(rule.action)
        } else {
            RuleResult::Pass
        }
    }
}
