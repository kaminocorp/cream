use std::str::FromStr;
use std::sync::{LazyLock, Mutex};

use cream_models::prelude::{ComparisonOp, PolicyAction, PolicyCondition, PolicyRule};
use rust_decimal::Decimal;

use crate::context::EvaluationContext;

/// Cache for compiled regex patterns. Avoids re-compiling the same pattern on
/// every `Matches` evaluation. Bounded to prevent unbounded memory growth from
/// operator-defined patterns — evicts all entries when the limit is reached.
static REGEX_CACHE: LazyLock<Mutex<std::collections::HashMap<String, regex::Regex>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

const REGEX_CACHE_MAX: usize = 256;

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
            Some(c) => serde_json::Value::String(c.as_str().to_owned()),
            None => serde_json::Value::Null,
        },
        "agent.status" => serde_json::to_value(ctx.agent.status).unwrap_or_default(),
        "metadata.agent_session_id" => match &ctx.request.metadata {
            Some(m) => match &m.agent_session_id {
                Some(v) => serde_json::Value::String(v.clone()),
                None => serde_json::Value::Null,
            },
            None => serde_json::Value::Null,
        },
        "metadata.workflow_id" => match &ctx.request.metadata {
            Some(m) => match &m.workflow_id {
                Some(v) => serde_json::Value::String(v.clone()),
                None => serde_json::Value::Null,
            },
            None => serde_json::Value::Null,
        },
        "metadata.operator_ref" => match &ctx.request.metadata {
            Some(m) => match &m.operator_ref {
                Some(v) => serde_json::Value::String(v.clone()),
                None => serde_json::Value::Null,
            },
            None => serde_json::Value::Null,
        },
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
        ComparisonOp::GreaterThan => compare_decimal(field, expected, |a, b| a > b),
        ComparisonOp::LessThan => compare_decimal(field, expected, |a, b| a < b),
        ComparisonOp::GreaterThanOrEqual => compare_decimal(field, expected, |a, b| a >= b),
        ComparisonOp::LessThanOrEqual => compare_decimal(field, expected, |a, b| a <= b),
        ComparisonOp::In => match expected {
            serde_json::Value::Array(arr) => arr.contains(field),
            _ => {
                tracing::warn!("In condition has non-array value, returning false");
                false
            }
        },
        ComparisonOp::NotIn => match expected {
            serde_json::Value::Array(arr) => !arr.contains(field),
            _ => {
                tracing::warn!(
                    "NotIn condition has non-array value, failing safe (returning false)"
                );
                false
            }
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

/// Compare two JSON values as `rust_decimal::Decimal` for financial precision.
///
/// Handles both numeric JSON values (e.g., `100.05`) and string-serialized
/// decimals (e.g., `"100.05"` from `serde-with-str`). Using Decimal instead
/// of f64 eliminates IEEE 754 precision issues in the money path.
fn compare_decimal(
    a: &serde_json::Value,
    b: &serde_json::Value,
    cmp: fn(&Decimal, &Decimal) -> bool,
) -> bool {
    match (as_decimal(a), as_decimal(b)) {
        (Some(va), Some(vb)) => cmp(&va, &vb),
        _ => false,
    }
}

fn as_decimal(v: &serde_json::Value) -> Option<Decimal> {
    // Try string first (rust_decimal serializes as string with serde-with-str),
    // then try numeric JSON values via their string representation.
    if let Some(s) = v.as_str() {
        return Decimal::from_str(s).ok();
    }
    if let Some(n) = v.as_u64() {
        return Some(Decimal::from(n));
    }
    if let Some(n) = v.as_i64() {
        return Some(Decimal::from(n));
    }
    // For f64 JSON numbers, convert via string to preserve the displayed value
    // rather than the binary representation (e.g., 100.05 stays "100.05").
    if let Some(n) = v.as_f64() {
        return Decimal::from_str(&n.to_string()).ok();
    }
    None
}

fn regex_matches(text: &str, pattern: &str) -> bool {
    let cache = match REGEX_CACHE.lock() {
        Ok(c) => c,
        Err(_) => {
            // Poisoned mutex — fall back to uncached compilation
            return regex::Regex::new(pattern)
                .map(|re| re.is_match(text))
                .unwrap_or(false);
        }
    };

    if let Some(re) = cache.get(pattern) {
        return re.is_match(text);
    }
    // Drop the read lock before compiling
    drop(cache);

    match regex::Regex::new(pattern) {
        Ok(re) => {
            let result = re.is_match(text);
            if let Ok(mut cache) = REGEX_CACHE.lock() {
                if cache.len() >= REGEX_CACHE_MAX {
                    // Evict the oldest entry (by insertion order via arbitrary key)
                    // rather than clearing the entire cache, so hot patterns survive.
                    if let Some(oldest_key) = cache.keys().next().cloned() {
                        cache.remove(&oldest_key);
                    }
                }
                cache.insert(pattern.to_string(), re);
            }
            result
        }
        Err(e) => {
            tracing::warn!(pattern, error = %e, "invalid regex pattern in Matches condition, returning false");
            false
        }
    }
}
