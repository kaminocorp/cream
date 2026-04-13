use serde::{Deserialize, Serialize};

use crate::agent::CountryCode;

/// Maximum allowed length for `Recipient.identifier`.
pub const MAX_RECIPIENT_IDENTIFIER_LEN: usize = 500;

/// Maximum allowed length for `Recipient.name`.
pub const MAX_RECIPIENT_NAME_LEN: usize = 255;

/// The recipient of a payment.
///
/// Mirrors the vision doc's recipient schema: a type discriminator plus a
/// polymorphic identifier that varies by recipient type (merchant ID, email,
/// wallet address, or bank account details).
///
/// Custom `Deserialize` enforces length bounds on string fields to prevent
/// audit log bloat (the audit ledger is append-only).
#[derive(Debug, Clone, Serialize)]
pub struct Recipient {
    /// What kind of entity is receiving the payment.
    #[serde(rename = "type")]
    pub recipient_type: RecipientType,

    /// The identifier for this recipient. Format depends on `recipient_type`:
    /// - Merchant: Stripe/Airwallex merchant ID
    /// - Individual: email address
    /// - Wallet: blockchain wallet address (e.g., 0x...)
    /// - BankAccount: structured bank account reference
    pub identifier: String,

    /// Optional human-readable name for audit/display purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional ISO 3166-1 alpha-2 country code for the recipient.
    /// Used by the routing engine for corridor-based provider selection
    /// and by policy rules for geographic restrictions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
}

impl<'de> Deserialize<'de> for Recipient {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "type")]
            recipient_type: RecipientType,
            identifier: String,
            name: Option<String>,
            country: Option<CountryCode>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.identifier.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "recipient.identifier must not be empty or whitespace-only",
            ));
        }
        if raw.identifier.len() > MAX_RECIPIENT_IDENTIFIER_LEN {
            return Err(serde::de::Error::custom(format!(
                "recipient.identifier exceeds maximum length of {} characters (got {})",
                MAX_RECIPIENT_IDENTIFIER_LEN,
                raw.identifier.len()
            )));
        }
        if let Some(ref name) = raw.name {
            if name.trim().is_empty() {
                return Err(serde::de::Error::custom(
                    "recipient.name must not be empty or whitespace-only when present",
                ));
            }
            if name.len() > MAX_RECIPIENT_NAME_LEN {
                return Err(serde::de::Error::custom(format!(
                    "recipient.name exceeds maximum length of {} characters (got {})",
                    MAX_RECIPIENT_NAME_LEN,
                    name.len()
                )));
            }
        }

        Ok(Recipient {
            recipient_type: raw.recipient_type,
            identifier: raw.identifier,
            name: raw.name,
            country: raw.country,
        })
    }
}

/// The type of payment recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipientType {
    /// A merchant (e.g., Stripe merchant, Shopify store)
    Merchant,
    /// An individual person (e.g., freelancer payout)
    Individual,
    /// A crypto wallet address
    Wallet,
    /// A bank account (for SWIFT/local transfers)
    BankAccount,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_serde_roundtrip() {
        let r = Recipient {
            recipient_type: RecipientType::Merchant,
            identifier: "stripe_merch_123".to_string(),
            name: Some("Acme Corp".to_string()),
            country: Some(CountryCode::new("SG")),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["type"], "merchant");
        let parsed: Recipient = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.recipient_type, RecipientType::Merchant);
        assert_eq!(parsed.country.unwrap().as_str(), "SG");
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: recipient string bounds
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 6.15: empty identifier rejection
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_empty_identifier_rejected() {
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": "",
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("identifier"));
    }

    #[test]
    fn recipient_whitespace_identifier_rejected() {
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": "   ",
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("identifier"));
    }

    // -----------------------------------------------------------------------
    // Phase 6.10: recipient string bounds
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_identifier_exceeding_limit_rejected() {
        let long = "m".repeat(MAX_RECIPIENT_IDENTIFIER_LEN + 1);
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": long,
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("identifier"));
    }

    #[test]
    fn recipient_name_exceeding_limit_rejected() {
        let long = "n".repeat(MAX_RECIPIENT_NAME_LEN + 1);
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": "valid_id",
            "name": long,
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.5: empty/whitespace recipient name rejection
    // -----------------------------------------------------------------------

    #[test]
    fn recipient_empty_name_rejected() {
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": "valid_id",
            "name": "",
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn recipient_whitespace_name_rejected() {
        let json = serde_json::json!({
            "type": "merchant",
            "identifier": "valid_id",
            "name": "   ",
        });
        let result: Result<Recipient, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn recipient_at_exact_limits_accepted() {
        let ident = "i".repeat(MAX_RECIPIENT_IDENTIFIER_LEN);
        let name = "n".repeat(MAX_RECIPIENT_NAME_LEN);
        let json = serde_json::json!({
            "type": "wallet",
            "identifier": ident,
            "name": name,
        });
        let parsed: Recipient = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.identifier.len(), MAX_RECIPIENT_IDENTIFIER_LEN);
        assert_eq!(parsed.name.unwrap().len(), MAX_RECIPIENT_NAME_LEN);
    }
}
