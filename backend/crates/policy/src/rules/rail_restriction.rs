use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks payments requesting a rail not in the profile's allowed list.
///
/// If `allowed_rails` is empty, all rails are allowed. `RailPreference::Auto`
/// always passes because the routing engine will select an allowed rail.
pub struct RailRestrictionEvaluator;

impl RuleEvaluator for RailRestrictionEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        use cream_models::prelude::RailPreference;

        if ctx.profile.allowed_rails.is_empty() {
            return RuleResult::Pass;
        }

        // Auto is always allowed — the routing engine handles rail selection
        if ctx.request.preferred_rail == RailPreference::Auto {
            return RuleResult::Pass;
        }

        if ctx
            .profile
            .allowed_rails
            .contains(&ctx.request.preferred_rail)
        {
            RuleResult::Pass
        } else {
            RuleResult::Triggered(rule.action)
        }
    }
}
