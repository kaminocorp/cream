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

/// A composable condition tree supporting boolean logic.
///
/// `All` = AND, `Any` = OR, `Not` = negation, `FieldCheck` = leaf comparison.
/// This structure lets operators express arbitrarily complex rules in
/// declarative YAML/JSON without writing code.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// How to notify the human reviewer.
    pub channel: EscalationChannel,
    /// How long to wait for a human decision before timing out.
    pub timeout_minutes: u32,
    /// What to do if the timeout expires without a human decision.
    pub on_timeout: PolicyAction,
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
}
