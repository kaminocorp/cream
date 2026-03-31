use serde::{Deserialize, Serialize};

/// The recipient of a payment.
///
/// Mirrors the vision doc's recipient schema: a type discriminator plus a
/// polymorphic identifier that varies by recipient type (merchant ID, email,
/// wallet address, or bank account details).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub country: Option<String>,
}

/// The type of payment recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            country: Some("SG".to_string()),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["type"], "merchant");
        let parsed: Recipient = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.recipient_type, RecipientType::Merchant);
    }
}
