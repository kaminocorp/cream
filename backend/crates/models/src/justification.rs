use serde::{Deserialize, Serialize};

use crate::payment::MAX_CATEGORY_OTHER_LEN;

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
/// strongly typed. The `Other` string is bounded to [`MAX_CATEGORY_OTHER_LEN`]
/// characters on deserialization to prevent audit log bloat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// Custom deserializer that enforces length bounds on `Other(String)`.
impl<'de> Deserialize<'de> for PaymentCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize using the same tagged-enum representation as serde's
        // rename_all = "snake_case".  The inner helper mirrors the enum but
        // with derived Deserialize so we don't recurse.
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Raw {
            SaasSubscription,
            CloudInfrastructure,
            ApiCredits,
            Travel,
            Procurement,
            Marketing,
            Legal,
            Other(String),
        }

        match Raw::deserialize(deserializer)? {
            Raw::SaasSubscription => Ok(Self::SaasSubscription),
            Raw::CloudInfrastructure => Ok(Self::CloudInfrastructure),
            Raw::ApiCredits => Ok(Self::ApiCredits),
            Raw::Travel => Ok(Self::Travel),
            Raw::Procurement => Ok(Self::Procurement),
            Raw::Marketing => Ok(Self::Marketing),
            Raw::Legal => Ok(Self::Legal),
            Raw::Other(s) => {
                if s.len() > MAX_CATEGORY_OTHER_LEN {
                    return Err(serde::de::Error::custom(format!(
                        "PaymentCategory::Other exceeds maximum length of {} characters (got {})",
                        MAX_CATEGORY_OTHER_LEN,
                        s.len()
                    )));
                }
                Ok(Self::Other(s))
            }
        }
    }
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

    #[test]
    fn category_other_within_limit_deserializes() {
        let short = PaymentCategory::Other("custom_cat".to_string());
        let json = serde_json::to_string(&short).unwrap();
        let parsed: PaymentCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, short);
    }

    #[test]
    fn category_other_exceeding_limit_rejected() {
        let long_str = "x".repeat(MAX_CATEGORY_OTHER_LEN + 1);
        let json = format!("{{\"other\":\"{}\"}}", long_str);
        let result: Result<PaymentCategory, _> = serde_json::from_str(&json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("maximum length"));
    }

    #[test]
    fn category_other_at_exact_limit_accepted() {
        let exact = "y".repeat(MAX_CATEGORY_OTHER_LEN);
        let cat = PaymentCategory::Other(exact.clone());
        let json = serde_json::to_string(&cat).unwrap();
        let parsed: PaymentCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PaymentCategory::Other(exact));
    }
}
