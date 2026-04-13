use serde::{Deserialize, Serialize};

use crate::payment::MAX_CATEGORY_OTHER_LEN;

/// Maximum allowed length for `Justification.summary`.
/// Audit log is append-only, so unbounded summaries persist forever.
pub const MAX_JUSTIFICATION_SUMMARY_LEN: usize = 2000;

/// Maximum allowed length for optional justification string fields
/// (`task_id`, `expected_value`).
pub const MAX_JUSTIFICATION_FIELD_LEN: usize = 500;

/// The structured justification an agent must provide with every payment.
///
/// This is the **novel differentiator** — no payment moves without the agent
/// explaining why. The justification is persisted verbatim in the audit ledger
/// and can itself be evaluated by policy rules.
///
/// Custom `Deserialize` enforces length bounds on all string fields to prevent
/// audit log bloat (the audit ledger is append-only — oversized fields persist forever).
#[derive(Debug, Clone, Serialize)]
pub struct Justification {
    /// Human-readable explanation of why this payment is needed.
    /// Required. Minimum 10 words (enforced by policy engine).
    /// Maximum [`MAX_JUSTIFICATION_SUMMARY_LEN`] characters (enforced here).
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

impl<'de> Deserialize<'de> for Justification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            summary: String,
            task_id: Option<String>,
            category: PaymentCategory,
            expected_value: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.summary.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "justification.summary must not be empty or whitespace-only",
            ));
        }
        if raw.summary.len() > MAX_JUSTIFICATION_SUMMARY_LEN {
            return Err(serde::de::Error::custom(format!(
                "justification.summary exceeds maximum length of {} characters (got {})",
                MAX_JUSTIFICATION_SUMMARY_LEN,
                raw.summary.len()
            )));
        }
        if let Some(ref s) = raw.task_id {
            if s.trim().is_empty() {
                return Err(serde::de::Error::custom(
                    "justification.task_id must not be empty or whitespace-only when present",
                ));
            }
            if s.len() > MAX_JUSTIFICATION_FIELD_LEN {
                return Err(serde::de::Error::custom(format!(
                    "justification.task_id exceeds maximum length of {} characters (got {})",
                    MAX_JUSTIFICATION_FIELD_LEN,
                    s.len()
                )));
            }
        }
        if let Some(ref s) = raw.expected_value {
            if s.trim().is_empty() {
                return Err(serde::de::Error::custom(
                    "justification.expected_value must not be empty or whitespace-only when present",
                ));
            }
            if s.len() > MAX_JUSTIFICATION_FIELD_LEN {
                return Err(serde::de::Error::custom(format!(
                    "justification.expected_value exceeds maximum length of {} characters (got {})",
                    MAX_JUSTIFICATION_FIELD_LEN,
                    s.len()
                )));
            }
        }

        Ok(Justification {
            summary: raw.summary,
            task_id: raw.task_id,
            category: raw.category,
            expected_value: raw.expected_value,
        })
    }
}

/// Controlled vocabulary for payment categories.
///
/// Maps to MCC (Merchant Category Code) groups for card-rail payments.
/// The `Other` variant allows extensibility while keeping the common cases
/// strongly typed. The `Other` string is bounded to [`MAX_CATEGORY_OTHER_LEN`]
/// characters on deserialization to prevent audit log bloat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
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
                if s.trim().is_empty() {
                    return Err(serde::de::Error::custom(
                        "PaymentCategory::Other must not be empty or whitespace-only",
                    ));
                }
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

    // -----------------------------------------------------------------------
    // Phase 6.15: empty/whitespace summary rejection
    // -----------------------------------------------------------------------

    #[test]
    fn justification_empty_summary_rejected() {
        let json = serde_json::json!({
            "summary": "",
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn justification_whitespace_only_summary_rejected() {
        let json = serde_json::json!({
            "summary": "   \t\n  ",
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: justification string bounds
    // -----------------------------------------------------------------------

    #[test]
    fn justification_summary_within_limit_accepted() {
        let j = Justification {
            summary: "a]".repeat(100),
            task_id: None,
            category: PaymentCategory::ApiCredits,
            expected_value: None,
        };
        let json = serde_json::to_string(&j).unwrap();
        let parsed: Justification = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.summary, j.summary);
    }

    #[test]
    fn justification_summary_exceeding_limit_rejected() {
        let long = "x".repeat(MAX_JUSTIFICATION_SUMMARY_LEN + 1);
        let json = serde_json::json!({
            "summary": long,
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("summary"));
    }

    #[test]
    fn justification_summary_at_exact_limit_accepted() {
        let exact = "y".repeat(MAX_JUSTIFICATION_SUMMARY_LEN);
        let json = serde_json::json!({
            "summary": exact,
            "category": "api_credits"
        });
        let parsed: Justification = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.summary.len(), MAX_JUSTIFICATION_SUMMARY_LEN);
    }

    #[test]
    fn justification_task_id_exceeding_limit_rejected() {
        let long = "t".repeat(MAX_JUSTIFICATION_FIELD_LEN + 1);
        let json = serde_json::json!({
            "summary": "valid summary text",
            "task_id": long,
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_id"));
    }

    #[test]
    fn justification_expected_value_exceeding_limit_rejected() {
        let long = "e".repeat(MAX_JUSTIFICATION_FIELD_LEN + 1);
        let json = serde_json::json!({
            "summary": "valid summary text",
            "category": "api_credits",
            "expected_value": long
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected_value"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.5: empty/whitespace optional string fields
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 7.8: PaymentCategory::Other empty/whitespace guard
    // -----------------------------------------------------------------------

    #[test]
    fn category_other_empty_rejected() {
        let json = r#"{"other":""}"#;
        let result: Result<PaymentCategory, _> = serde_json::from_str(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    #[test]
    fn category_other_whitespace_only_rejected() {
        let json = r#"{"other":"   \t  "}"#;
        let result: Result<PaymentCategory, _> = serde_json::from_str(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    #[test]
    fn justification_empty_task_id_rejected() {
        let json = serde_json::json!({
            "summary": "valid summary text",
            "task_id": "",
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_id"));
    }

    #[test]
    fn justification_whitespace_task_id_rejected() {
        let json = serde_json::json!({
            "summary": "valid summary text",
            "task_id": "   ",
            "category": "api_credits"
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_id"));
    }

    #[test]
    fn justification_empty_expected_value_rejected() {
        let json = serde_json::json!({
            "summary": "valid summary text",
            "category": "api_credits",
            "expected_value": ""
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected_value"));
    }

    #[test]
    fn justification_whitespace_expected_value_rejected() {
        let json = serde_json::json!({
            "summary": "valid summary text",
            "category": "api_credits",
            "expected_value": "  \t  "
        });
        let result: Result<Justification, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected_value"));
    }
}
