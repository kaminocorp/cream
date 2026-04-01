use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Flags payments where the amount is disproportionate to the stated expected value.
///
/// **Stub implementation.** Full proportionality analysis requires semantic
/// understanding of the `expected_value` field (e.g., "is $5,000 proportional
/// to 'Complete onboarding batch'?"). This will be implemented post-scaffold
/// with an LLM-based background check, similar to justification quality.
pub struct ProportionalityEvaluator;

impl RuleEvaluator for ProportionalityEvaluator {
    fn evaluate(&self, rule: &PolicyRule, _ctx: &EvaluationContext) -> RuleResult {
        // TODO: Implement proportionality analysis.
        // When implemented, this would:
        // 1. Parse expected_value for numeric hints (if any)
        // 2. Compare amount to expected_value magnitude
        // 3. Flag if amount > 10× stated expected value
        // 4. LLM-based semantic check for non-numeric expected values
        tracing::warn!(
            rule_id = %rule.id,
            "proportionality evaluator is a stub — rule will always pass. Do not use in production policies"
        );
        RuleResult::Pass
    }
}
