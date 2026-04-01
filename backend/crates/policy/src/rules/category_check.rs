use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks payments whose category is not in the agent's allowed categories.
///
/// If the profile's `allowed_categories` is empty, all categories are allowed.
pub struct CategoryCheckEvaluator;

impl RuleEvaluator for CategoryCheckEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        if ctx.profile.allowed_categories.is_empty() {
            return RuleResult::Pass;
        }

        if ctx
            .profile
            .allowed_categories
            .contains(&ctx.request.justification.category)
        {
            RuleResult::Pass
        } else {
            RuleResult::Triggered(rule.action)
        }
    }
}
