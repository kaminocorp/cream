use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks payments to recipients in countries not in the profile's allowed list.
///
/// If `geographic_restrictions` is empty, all countries are allowed.
/// If the recipient has no country and restrictions are configured, the rule
/// triggers (fail-closed) — an agent cannot bypass geographic controls by
/// omitting the country field.
pub struct GeographicEvaluator;

impl RuleEvaluator for GeographicEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        if ctx.profile.geographic_restrictions.is_empty() {
            return RuleResult::Pass;
        }

        let recipient_country = match &ctx.request.recipient.country {
            Some(c) => c,
            // Fail-closed: restrictions are configured but country is unknown.
            None => return RuleResult::Triggered(rule.action),
        };

        let allowed = ctx
            .profile
            .geographic_restrictions
            .iter()
            .any(|cc| cc.as_str().eq_ignore_ascii_case(recipient_country.as_str()));

        if allowed {
            RuleResult::Pass
        } else {
            RuleResult::Triggered(rule.action)
        }
    }
}
