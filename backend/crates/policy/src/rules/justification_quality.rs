use cream_models::prelude::PolicyRule;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Validates the structural quality of the agent's justification.
///
/// Checks:
/// 1. Summary is non-empty
/// 2. Summary has at least 10 words (minimum for meaningful justification)
///
/// LLM-based semantic coherence checking is intentionally NOT on the hot path.
/// Per Vision Section 15 item 4, the 2-second Stripe Issuing authorization
/// constraint means the policy engine must complete in under 500ms. LLM checks
/// should be async background tasks that flag for review.
pub struct JustificationQualityEvaluator;

const MIN_WORD_COUNT: usize = 10;

impl RuleEvaluator for JustificationQualityEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let summary = &ctx.request.justification.summary;

        // Check 1: Non-empty
        if summary.trim().is_empty() {
            return RuleResult::Triggered(rule.action);
        }

        // Check 2: Minimum word count
        let word_count = summary.split_whitespace().count();
        if word_count < MIN_WORD_COUNT {
            return RuleResult::Triggered(rule.action);
        }

        // TODO: LLM coherence check (feature-flagged, async, non-blocking)
        // When implemented, this would:
        // 1. Submit summary + category to an LLM for semantic consistency check
        // 2. Fire-and-forget — result flags for human review, does NOT block payment
        // 3. Gated behind a feature flag: `#[cfg(feature = "llm-justification")]`

        RuleResult::Pass
    }
}
