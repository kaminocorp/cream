use serde::{Deserialize, Serialize};

/// The structured justification an agent must provide with every payment.
///
/// This is the **novel differentiator** — no payment moves without the agent
/// explaining why. The justification is persisted verbatim in the audit ledger
/// and can itself be evaluated by policy rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Justification {
    /// Human-readable explanation of why this payment is needed.
    /// Required. Minimum 10 words (enforced by policy engine, not here —
    /// models crate is validation-free).
    pub summary: String,

    /// Optional reference to the task or workflow that triggered this payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Controlled vocabulary category, mapped to operator policy rules.
    pub category: PaymentCategory,

    /// Optional description of expected value or outcome.
    /// Enables proportionality rules (e.g., "block if amount > 10× expected value").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<String>,
}

/// Controlled vocabulary for payment categories.
///
/// Maps to MCC (Merchant Category Code) groups for card-rail payments.
/// The `Other` variant allows extensibility while keeping the common cases
/// strongly typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentCategory {
    SaasSubscription,
    CloudInfrastructure,
    ApiCredits,
    Travel,
    Procurement,
    Marketing,
    Legal,
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_serde_roundtrip() {
        let cat = PaymentCategory::CloudInfrastructure;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"cloud_infrastructure\"");
        let parsed: PaymentCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cat);
    }

    #[test]
    fn category_other_serde() {
        let cat = PaymentCategory::Other("custom_category".to_string());
        let json = serde_json::to_string(&cat).unwrap();
        let parsed: PaymentCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cat);
    }

    #[test]
    fn justification_serde() {
        let j = Justification {
            summary: "Purchasing API credits for batch processing".to_string(),
            task_id: Some("task_123".to_string()),
            category: PaymentCategory::ApiCredits,
            expected_value: Some("Complete onboarding batch".to_string()),
        };
        let json = serde_json::to_value(&j).unwrap();
        assert_eq!(json["category"], "api_credits");
    }
}
