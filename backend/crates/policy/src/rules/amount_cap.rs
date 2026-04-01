use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks or escalates payments that exceed the agent's per-transaction limit.
///
/// **Currency note:** Profile limits (`max_per_transaction`) are currency-agnostic
/// numeric ceilings — they cap the raw amount regardless of denomination. Operators
/// who need per-currency caps should create separate profiles per currency.
pub struct AmountCapEvaluator;

impl RuleEvaluator for AmountCapEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        if ctx.request.amount > ctx.profile.max_per_transaction {
            tracing::info!(
                amount = %ctx.request.amount,
                currency = ?ctx.request.currency,
                limit = %ctx.profile.max_per_transaction,
                "amount_cap triggered"
            );
            RuleResult::Triggered(rule.action)
        } else {
            RuleResult::Pass
        }
    }
}
