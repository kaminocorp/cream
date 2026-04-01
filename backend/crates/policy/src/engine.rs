use std::collections::HashMap;
use std::time::Instant;

use cream_models::prelude::{PolicyAction, PolicyRule, PolicyRuleId};
use tracing::instrument;

use crate::context::EvaluationContext;
use crate::error::PolicyError;
use crate::evaluator::{RuleEvaluator, RuleResult};
use crate::rules::{
    amount_cap::AmountCapEvaluator, category_check::CategoryCheckEvaluator,
    duplicate_detection::DuplicateDetectionEvaluator,
    first_time_merchant::FirstTimeMerchantEvaluator, geographic::GeographicEvaluator,
    justification_quality::JustificationQualityEvaluator, merchant_check::MerchantCheckEvaluator,
    rail_restriction::RailRestrictionEvaluator, spend_rate::SpendRateEvaluator,
    time_window::TimeWindowEvaluator, velocity_limit::VelocityLimitEvaluator,
};

// ---------------------------------------------------------------------------
// Policy decision (output of the engine)
// ---------------------------------------------------------------------------

/// The result of evaluating all policy rules against a payment request.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// The final verdict: Approve, Block, or Escalate.
    pub action: PolicyAction,
    /// All rules that were evaluated (by ID).
    pub rules_evaluated: Vec<PolicyRuleId>,
    /// Rules whose conditions matched (subset of rules_evaluated).
    pub matching_rules: Vec<PolicyRuleId>,
    /// How long the evaluation took, in milliseconds.
    pub latency_ms: u64,
}

// ---------------------------------------------------------------------------
// Policy engine
// ---------------------------------------------------------------------------

/// The core policy evaluation engine.
///
/// Holds a registry of rule evaluators keyed by rule type string. Rules are
/// sorted by priority (ascending = higher priority) and evaluated in order.
///
/// Evaluation semantics:
/// - First `Block` halts evaluation immediately
/// - `Escalate` is collected but evaluation continues (a later `Block` overrides)
/// - If no rule triggers, the decision is `Approve`
pub struct PolicyEngine {
    evaluators: HashMap<String, Box<dyn RuleEvaluator>>,
}

impl PolicyEngine {
    /// Create a new engine with all built-in rule evaluators registered.
    pub fn new() -> Self {
        let mut evaluators: HashMap<String, Box<dyn RuleEvaluator>> = HashMap::new();

        evaluators.insert("amount_cap".into(), Box::new(AmountCapEvaluator));
        evaluators.insert("velocity_limit".into(), Box::new(VelocityLimitEvaluator));
        evaluators.insert("spend_rate".into(), Box::new(SpendRateEvaluator));
        evaluators.insert("category_check".into(), Box::new(CategoryCheckEvaluator));
        evaluators.insert("merchant_check".into(), Box::new(MerchantCheckEvaluator));
        evaluators.insert("geographic".into(), Box::new(GeographicEvaluator));
        evaluators.insert(
            "rail_restriction".into(),
            Box::new(RailRestrictionEvaluator),
        );
        evaluators.insert(
            "justification_quality".into(),
            Box::new(JustificationQualityEvaluator),
        );
        evaluators.insert("time_window".into(), Box::new(TimeWindowEvaluator));
        evaluators.insert(
            "first_time_merchant".into(),
            Box::new(FirstTimeMerchantEvaluator),
        );
        evaluators.insert(
            "duplicate_detection".into(),
            Box::new(DuplicateDetectionEvaluator),
        );
        // NOTE: ProportionalityEvaluator is intentionally NOT registered.
        // It is a stub that always passes (requires future LLM integration).
        // Registering it would silently approve all payments matching
        // proportionality rules. The struct is retained in rules/ for future
        // implementation — register it here once evaluate() is complete.

        Self { evaluators }
    }

    /// Register a custom rule evaluator. Overwrites any existing evaluator
    /// for the same rule type.
    pub fn register(&mut self, rule_type: impl Into<String>, evaluator: Box<dyn RuleEvaluator>) {
        self.evaluators.insert(rule_type.into(), evaluator);
    }

    /// Evaluate all rules against the given context.
    ///
    /// Rules are sorted by priority (ascending = higher priority) and only
    /// enabled rules are evaluated. Disabled rules are skipped.
    #[instrument(skip(self, rules, ctx), fields(rule_count = rules.len()))]
    pub fn evaluate(
        &self,
        rules: &[PolicyRule],
        ctx: &EvaluationContext,
    ) -> Result<PolicyDecision, PolicyError> {
        let start = Instant::now();

        // Sort rules by priority (ascending = higher priority)
        let mut sorted_rules: Vec<&PolicyRule> = rules.iter().filter(|r| r.enabled).collect();
        sorted_rules.sort_by_key(|r| r.priority);

        let mut rules_evaluated = Vec::new();
        let mut matching_rules = Vec::new();
        let mut has_escalate = false;

        for rule in &sorted_rules {
            // Use explicit rule_type if set; fall back to inference from
            // condition tree for backwards compatibility with rules that
            // don't have rule_type set.
            let rule_type = rule
                .rule_type
                .clone()
                .unwrap_or_else(|| infer_rule_type(rule));

            let evaluator = match self.evaluators.get(&rule_type) {
                Some(e) => e,
                None => {
                    tracing::warn!(rule_id = %rule.id, rule_type = %rule_type, "unknown rule type, skipping");
                    rules_evaluated.push(rule.id);
                    continue;
                }
            };

            rules_evaluated.push(rule.id);

            match evaluator.evaluate(rule, ctx) {
                RuleResult::Pass => {}
                RuleResult::Triggered(PolicyAction::Block) => {
                    matching_rules.push(rule.id);
                    tracing::info!(rule_id = %rule.id, "rule triggered: BLOCK");
                    return Ok(PolicyDecision {
                        action: PolicyAction::Block,
                        rules_evaluated,
                        matching_rules,
                        latency_ms: start.elapsed().as_millis() as u64,
                    });
                }
                RuleResult::Triggered(PolicyAction::Escalate) => {
                    matching_rules.push(rule.id);
                    has_escalate = true;
                    tracing::info!(rule_id = %rule.id, "rule triggered: ESCALATE");
                    // Continue evaluating — a later Block may override
                }
                RuleResult::Triggered(PolicyAction::Approve) => {
                    // Explicit approve from a rule — unusual but valid
                    matching_rules.push(rule.id);
                }
            }
        }

        let action = if has_escalate {
            PolicyAction::Escalate
        } else {
            PolicyAction::Approve
        };

        Ok(PolicyDecision {
            action,
            rules_evaluated,
            matching_rules,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Infer the rule type from the rule's condition tree.
///
/// Walks the condition tree to find the first FieldCheck and maps well-known
/// field names to rule types. Falls back to the field name itself.
fn infer_rule_type(rule: &PolicyRule) -> String {
    infer_from_condition(&rule.condition).unwrap_or_else(|| "unknown".to_string())
}

fn infer_from_condition(condition: &cream_models::prelude::PolicyCondition) -> Option<String> {
    use cream_models::prelude::PolicyCondition;
    match condition {
        PolicyCondition::FieldCheck(check) => {
            let rule_type = match check.field.as_str() {
                "amount" => "amount_cap",
                "velocity" => "velocity_limit",
                "spend_rate" | "daily_spend" | "weekly_spend" | "monthly_spend" => "spend_rate",
                "justification.category" => "category_check",
                "recipient.identifier" => "merchant_check",
                "recipient.country" => "geographic",
                "preferred_rail" | "rail" => "rail_restriction",
                "justification.summary" | "justification_quality" => "justification_quality",
                "time_window" => "time_window",
                "first_time_merchant" => "first_time_merchant",
                "duplicate" => "duplicate_detection",
                "proportionality" | "expected_value" => "proportionality",
                other => other,
            };
            Some(rule_type.to_string())
        }
        PolicyCondition::All(children) | PolicyCondition::Any(children) => {
            children.iter().find_map(infer_from_condition)
        }
        PolicyCondition::Not(inner) => infer_from_condition(inner),
    }
}
