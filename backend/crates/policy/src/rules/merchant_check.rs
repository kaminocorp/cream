use cream_models::prelude::{PolicyCondition, PolicyRule};

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks or escalates payments to merchants on a deny list, or not on an allow list.
///
/// The rule's condition tree should contain a FieldCheck with field "merchant"
/// and a value that is an array of merchant identifiers. The comparison op
/// determines allow-list (In) vs deny-list (NotIn) semantics.
pub struct MerchantCheckEvaluator;

impl RuleEvaluator for MerchantCheckEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let merchant_id = &ctx.request.recipient.identifier;

        if has_merchant_match(&rule.condition, merchant_id) {
            RuleResult::Triggered(rule.action)
        } else {
            RuleResult::Pass
        }
    }
}

fn has_merchant_match(condition: &PolicyCondition, merchant_id: &str) -> bool {
    match condition {
        PolicyCondition::FieldCheck(check) if check.field == "recipient.identifier" => {
            use cream_models::prelude::ComparisonOp;
            let merchant_lower = merchant_id.to_ascii_lowercase();
            match check.op {
                // "In" means merchant IS in the blocked list → trigger
                ComparisonOp::In => {
                    if let serde_json::Value::Array(arr) = &check.value {
                        arr.iter().any(|v| match v.as_str() {
                            Some(s) => s.eq_ignore_ascii_case(&merchant_lower),
                            None => false,
                        })
                    } else {
                        false
                    }
                }
                // "NotIn" means merchant is NOT in the allowed list → trigger
                ComparisonOp::NotIn => {
                    if let serde_json::Value::Array(arr) = &check.value {
                        !arr.iter().any(|v| match v.as_str() {
                            Some(s) => s.eq_ignore_ascii_case(&merchant_lower),
                            None => false,
                        })
                    } else {
                        false
                    }
                }
                ComparisonOp::Equals => match check.value.as_str() {
                    Some(s) => s.eq_ignore_ascii_case(merchant_id),
                    None => false,
                },
                _ => false,
            }
        }
        PolicyCondition::All(children) => {
            children.iter().all(|c| has_merchant_match(c, merchant_id))
        }
        PolicyCondition::Any(children) => {
            children.iter().any(|c| has_merchant_match(c, merchant_id))
        }
        PolicyCondition::Not(inner) => !has_merchant_match(inner, merchant_id),
        _ => false,
    }
}
