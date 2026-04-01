use cream_models::prelude::{ComparisonOp, PolicyAction, PolicyCondition, PolicyRule};

use crate::context::EvaluationContext;

// ---------------------------------------------------------------------------
// Rule result
// ---------------------------------------------------------------------------

/// The outcome of evaluating a single rule against a context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleResult {
    /// The rule's condition did not match — no action taken.
    Pass,
    /// The rule's condition matched — return its prescribed action.
    Triggered(PolicyAction),
}

// ---------------------------------------------------------------------------
// Rule evaluator trait
// ---------------------------------------------------------------------------

/// Evaluates a single policy rule against the evaluation context.
///
/// Each rule type (amount_cap, velocity_limit, etc.) has its own implementation.
/// The `PolicyEngine` dispatches to the correct evaluator based on the rule type.
pub trait RuleEvaluator: Send + Sync {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult;
}

// ---------------------------------------------------------------------------
// Condition evaluator — walks the PolicyCondition tree
// ---------------------------------------------------------------------------

/// Evaluate a `PolicyCondition` tree against the evaluation context.
///
/// This resolves the generic condition tree (AND/OR/NOT/FieldCheck) that
/// operators define in YAML/JSON rules. Each FieldCheck is resolved against
/// known fields from the payment request and context.
pub fn evaluate_condition(condition: &PolicyCondition, ctx: &EvaluationContext) -> bool {
    match condition {
        PolicyCondition::All(conditions) => conditions.iter().all(|c| evaluate_condition(c, ctx)),
        PolicyCondition::Any(conditions) => conditions.iter().any(|c| evaluate_condition(c, ctx)),
        PolicyCondition::Not(inner) => !evaluate_condition(inner, ctx),
        PolicyCondition::FieldCheck(check) => {
            let field_value = resolve_field(&check.field, ctx);
            compare_values(&field_value, &check.op, &check.value)
        }
    }
}

/// Resolve a dot-path field name to its current value from the context.
fn resolve_field(field: &str, ctx: &EvaluationContext) -> serde_json::Value {
    match field {
        "amount" => serde_json::to_value(ctx.request.amount).unwrap_or_default(),
        "currency" => serde_json::to_value(ctx.request.currency).unwrap_or_default(),
        "preferred_rail" => serde_json::to_value(ctx.request.preferred_rail).unwrap_or_default(),
        "justification.category" => {
            serde_json::to_value(&ctx.request.justification.category).unwrap_or_default()
        }
        "justification.summary" => {
            serde_json::Value::String(ctx.request.justification.summary.clone())
        }
        "recipient.type" => {
            serde_json::to_value(&ctx.request.recipient.recipient_type).unwrap_or_default()
        }
        "recipient.identifier" => {
            serde_json::Value::String(ctx.request.recipient.identifier.clone())
        }
        "recipient.country" => match &ctx.request.recipient.country {
            Some(c) => serde_json::Value::String(c.clone()),
            None => serde_json::Value::Null,
        },
        "agent.status" => serde_json::to_value(ctx.agent.status).unwrap_or_default(),
        unknown => {
            tracing::warn!(
                field = unknown,
                "unrecognized field in condition, resolving to null"
            );
            serde_json::Value::Null
        }
    }
}

/// Compare a resolved field value against an expected value using the given operator.
fn compare_values(
    field: &serde_json::Value,
    op: &ComparisonOp,
    expected: &serde_json::Value,
) -> bool {
    match op {
        ComparisonOp::Equals => field == expected,
        ComparisonOp::NotEquals => field != expected,
        ComparisonOp::GreaterThan => compare_numeric(field, expected, |a, b| a > b),
        ComparisonOp::LessThan => compare_numeric(field, expected, |a, b| a < b),
        ComparisonOp::GreaterThanOrEqual => compare_numeric(field, expected, |a, b| a >= b),
        ComparisonOp::LessThanOrEqual => compare_numeric(field, expected, |a, b| a <= b),
        ComparisonOp::In => match expected {
            serde_json::Value::Array(arr) => arr.contains(field),
            _ => false,
        },
        ComparisonOp::NotIn => match expected {
            serde_json::Value::Array(arr) => !arr.contains(field),
            _ => true,
        },
        ComparisonOp::Contains => match (field.as_str(), expected.as_str()) {
            (Some(haystack), Some(needle)) => haystack.contains(needle),
            _ => false,
        },
        ComparisonOp::Matches => match (field.as_str(), expected.as_str()) {
            (Some(text), Some(pattern)) => regex_matches(text, pattern),
            _ => false,
        },
    }
}

/// Compare two JSON values as f64 numbers.
fn compare_numeric(
    a: &serde_json::Value,
    b: &serde_json::Value,
    cmp: fn(f64, f64) -> bool,
) -> bool {
    match (as_f64(a), as_f64(b)) {
        (Some(va), Some(vb)) => cmp(va, vb),
        _ => false,
    }
}

fn as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.parse::<f64>().ok())
}

fn regex_matches(text: &str, pattern: &str) -> bool {
    match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(e) => {
            tracing::warn!(pattern, error = %e, "invalid regex pattern in Matches condition, returning false");
            false
        }
    }
}
