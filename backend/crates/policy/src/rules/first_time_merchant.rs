use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Escalates payments to merchants the agent has never transacted with before.
///
/// Checks the `known_merchants` set in the context — if the recipient's
/// identifier is not present, the merchant is considered first-time.
pub struct FirstTimeMerchantEvaluator;

impl RuleEvaluator for FirstTimeMerchantEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let id_lower = ctx.request.recipient.identifier.to_ascii_lowercase();
        if ctx.known_merchants.contains(&id_lower) {
            RuleResult::Pass
        } else {
            RuleResult::Triggered(rule.action)
        }
    }
}
