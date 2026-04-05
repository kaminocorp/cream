use serde::{Deserialize, Serialize};

use crate::ids::{AgentProfileId, PolicyRuleId};

// ---------------------------------------------------------------------------
// Policy Rule
// ---------------------------------------------------------------------------

/// A declarative rule evaluated by the policy engine.
///
/// Rules are evaluated in priority order (lower number = higher priority).
/// Each rule has a condition tree and an action (approve, block, escalate).
/// Rules are pure functions — no database calls, no network I/O — so
/// evaluation completes in single-digit milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: PolicyRuleId,
    pub profile_id: AgentProfileId,
    /// Explicit rule type identifier (e.g., "amount_cap", "velocity_limit").
    /// Used by the PolicyEngine to dispatch to the correct evaluator.
    /// If None, the engine infers the type from the condition tree (legacy fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_type: Option<String>,
    /// Lower number = higher priority. Rules are evaluated in ascending order.
    pub priority: i32,
    /// The condition tree that determines if this rule fires.
    pub condition: PolicyCondition,
    /// What to do when the condition matches.
    pub action: PolicyAction,
    /// Escalation configuration (only relevant when action = Escalate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation: Option<EscalationConfig>,
    /// Whether this rule is active. Disabled rules are skipped during evaluation.
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Policy Action
// ---------------------------------------------------------------------------

/// The verdict the policy engine returns for a payment request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyAction {
    Approve,
    Block,
    Escalate,
}

// ---------------------------------------------------------------------------
// Policy Condition — recursive condition tree
// ---------------------------------------------------------------------------

/// Maximum nesting depth for [`PolicyCondition`] trees.
/// Prevents stack overflow from deeply recursive deserialization.
pub const MAX_CONDITION_DEPTH: usize = 32;

/// A composable condition tree supporting boolean logic.
///
/// `All` = AND, `Any` = OR, `Not` = negation, `FieldCheck` = leaf comparison.
/// This structure lets operators express arbitrarily complex rules in
/// declarative YAML/JSON without writing code.
///
/// Custom `Deserialize` enforces a maximum nesting depth of
/// [`MAX_CONDITION_DEPTH`] to prevent stack overflow from maliciously crafted
/// or accidentally recursive rule definitions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyCondition {
    /// All sub-conditions must be true (logical AND).
    All(Vec<PolicyCondition>),
    /// At least one sub-condition must be true (logical OR).
    Any(Vec<PolicyCondition>),
    /// The sub-condition must be false (logical NOT).
    Not(Box<PolicyCondition>),
    /// A leaf comparison against a specific field value.
    FieldCheck(FieldCheck),
}

impl<'de> Deserialize<'de> for PolicyCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// Inner representation with derived Deserialize to avoid recursion.
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Raw {
            All(Vec<serde_json::Value>),
            Any(Vec<serde_json::Value>),
            Not(Box<serde_json::Value>),
            FieldCheck(FieldCheck),
        }

        fn parse_depth(val: serde_json::Value, depth: usize) -> Result<PolicyCondition, String> {
            if depth >= MAX_CONDITION_DEPTH {
                return Err(format!(
                    "PolicyCondition nesting exceeds maximum depth of {MAX_CONDITION_DEPTH}"
                ));
            }
            let raw: Raw =
                serde_json::from_value(val).map_err(|e| format!("invalid condition: {e}"))?;
            match raw {
                Raw::All(children) => {
                    let parsed = children
                        .into_iter()
                        .map(|c| parse_depth(c, depth + 1))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(PolicyCondition::All(parsed))
                }
                Raw::Any(children) => {
                    let parsed = children
                        .into_iter()
                        .map(|c| parse_depth(c, depth + 1))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(PolicyCondition::Any(parsed))
                }
                Raw::Not(child) => {
                    let parsed = parse_depth(*child, depth + 1)?;
                    Ok(PolicyCondition::Not(Box::new(parsed)))
                }
                Raw::FieldCheck(fc) => Ok(PolicyCondition::FieldCheck(fc)),
            }
        }

        let val = serde_json::Value::deserialize(deserializer)?;
        parse_depth(val, 0).map_err(serde::de::Error::custom)
    }
}

/// A single field comparison.
///
/// `field` is a dot-path into the payment request (e.g., "amount",
/// "justification.category", "recipient.country"). The policy engine
/// resolves these paths against the payment context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCheck {
    /// Dot-path to the field (e.g., "justification.category").
    pub field: String,
    /// The comparison operator.
    pub op: ComparisonOp,
    /// The value to compare against. Uses serde_json::Value for flexibility
    /// (numbers, strings, arrays for In/NotIn).
    pub value: serde_json::Value,
}

/// Comparison operators for field checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    /// Value must be one of the items in the comparison array.
    In,
    /// Value must not be any of the items in the comparison array.
    NotIn,
    /// String contains substring.
    Contains,
    /// String matches regex pattern.
    Matches,
}

// ---------------------------------------------------------------------------
// Escalation Config
// ---------------------------------------------------------------------------

/// Configuration for human-in-the-loop escalation.
///
/// When a policy rule's action is `Escalate`, this config determines how
/// the escalation is delivered and what happens if nobody responds.
///
/// Custom `Deserialize` enforces that `on_timeout` is not `Escalate` —
/// that would create an infinite escalation loop where the payment cycles
/// through timeout → escalate → timeout → escalate forever.
#[derive(Debug, Clone, Serialize)]
pub struct EscalationConfig {
    /// How to notify the human reviewer.
    pub channel: EscalationChannel,
    /// How long to wait for a human decision before timing out.
    pub timeout_minutes: u32,
    /// What to do if the timeout expires without a human decision.
    /// Must be `Approve` or `Block` — never `Escalate` (infinite loop).
    pub on_timeout: PolicyAction,
}

impl<'de> Deserialize<'de> for EscalationConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            channel: EscalationChannel,
            timeout_minutes: u32,
            on_timeout: PolicyAction,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.timeout_minutes == 0 {
            return Err(serde::de::Error::custom(
                "escalation timeout_minutes must be > 0 — \
                 zero timeout means instant expiry with no human review window",
            ));
        }
        if raw.on_timeout == PolicyAction::Escalate {
            return Err(serde::de::Error::custom(
                "escalation on_timeout must not be ESCALATE — \
                 that would create an infinite escalation loop",
            ));
        }

        Ok(EscalationConfig {
            channel: raw.channel,
            timeout_minutes: raw.timeout_minutes,
            on_timeout: raw.on_timeout,
        })
    }
}

/// Delivery channel for escalation notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationChannel {
    Slack,
    Email,
    Webhook,
    Dashboard,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_condition_serde_roundtrip() {
        let condition = PolicyCondition::All(vec![
            PolicyCondition::FieldCheck(FieldCheck {
                field: "amount".to_string(),
                op: ComparisonOp::GreaterThan,
                value: serde_json::json!(500),
            }),
            PolicyCondition::Not(Box::new(PolicyCondition::FieldCheck(FieldCheck {
                field: "justification.category".to_string(),
                op: ComparisonOp::In,
                value: serde_json::json!(["saas_subscription", "cloud_infrastructure"]),
            }))),
        ]);

        let json = serde_json::to_value(&condition).unwrap();
        let parsed: PolicyCondition = serde_json::from_value(json).unwrap();

        // Verify structure survived the roundtrip
        match parsed {
            PolicyCondition::All(conditions) => assert_eq!(conditions.len(), 2),
            _ => panic!("expected All"),
        }
    }

    #[test]
    fn policy_action_serde() {
        let a = PolicyAction::Escalate;
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"ESCALATE\"");
    }

    #[test]
    fn escalation_config_serde() {
        let config = EscalationConfig {
            channel: EscalationChannel::Slack,
            timeout_minutes: 30,
            on_timeout: PolicyAction::Block,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["channel"], "slack");
        assert_eq!(json["timeout_minutes"], 30);
        assert_eq!(json["on_timeout"], "BLOCK");
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: escalation on_timeout loop prevention
    // -----------------------------------------------------------------------

    #[test]
    fn escalation_on_timeout_escalate_rejected() {
        let json = serde_json::json!({
            "channel": "slack",
            "timeout_minutes": 30,
            "on_timeout": "ESCALATE"
        });
        let result: Result<EscalationConfig, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("infinite escalation loop"));
    }

    #[test]
    fn escalation_zero_timeout_rejected() {
        let json = serde_json::json!({
            "channel": "slack",
            "timeout_minutes": 0,
            "on_timeout": "BLOCK"
        });
        let result: Result<EscalationConfig, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timeout_minutes"));
    }

    #[test]
    fn escalation_on_timeout_approve_accepted() {
        let json = serde_json::json!({
            "channel": "webhook",
            "timeout_minutes": 15,
            "on_timeout": "APPROVE"
        });
        let parsed: EscalationConfig = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.on_timeout, PolicyAction::Approve);
    }

    #[test]
    fn escalation_on_timeout_block_accepted() {
        let json = serde_json::json!({
            "channel": "email",
            "timeout_minutes": 60,
            "on_timeout": "BLOCK"
        });
        let parsed: EscalationConfig = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.on_timeout, PolicyAction::Block);
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: PolicyCondition depth limit
    // -----------------------------------------------------------------------

    #[test]
    fn policy_condition_shallow_nesting_accepted() {
        // 3 levels deep — well within limit
        let condition =
            PolicyCondition::All(vec![PolicyCondition::Any(vec![PolicyCondition::Not(
                Box::new(PolicyCondition::FieldCheck(FieldCheck {
                    field: "amount".to_string(),
                    op: ComparisonOp::GreaterThan,
                    value: serde_json::json!(100),
                })),
            )])]);
        let json = serde_json::to_value(&condition).unwrap();
        let parsed: PolicyCondition = serde_json::from_value(json).unwrap();
        match parsed {
            PolicyCondition::All(children) => assert_eq!(children.len(), 1),
            _ => panic!("expected All"),
        }
    }

    #[test]
    fn policy_condition_excessive_nesting_rejected() {
        // Build a deeply nested condition tree exceeding MAX_CONDITION_DEPTH
        let mut val = serde_json::json!({
            "field_check": {
                "field": "amount",
                "op": "greater_than",
                "value": 100
            }
        });
        for _ in 0..MAX_CONDITION_DEPTH + 5 {
            val = serde_json::json!({ "not": val });
        }
        let result: Result<PolicyCondition, _> = serde_json::from_value(val);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("maximum depth"));
    }
}
