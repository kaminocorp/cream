use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, VirtualCardId};
use crate::payment::Currency;
use crate::provider::ProviderId;

// ---------------------------------------------------------------------------
// Virtual Card
// ---------------------------------------------------------------------------

/// A scoped virtual card issued to an agent for card-rail payments.
///
/// Cards are the primary mechanism for agent spending on traditional
/// merchant rails. Each card has strict controls (amount caps, MCC codes,
/// expiry) and is tied to a single agent and provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualCard {
    pub id: VirtualCardId,
    /// The agent this card is issued to.
    pub agent_id: AgentId,
    /// Which provider issued this card (e.g., "stripe_issuing", "airwallex_issuing").
    pub provider: ProviderId,
    /// The provider's own card ID (for API calls back to the provider).
    pub provider_card_id: String,
    pub card_type: CardType,
    pub controls: CardControls,
    pub status: CardStatus,
    pub created_at: DateTime<Utc>,
    /// When this card expires. `None` for cards that don't auto-expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Whether the card is single-use (one authorization) or multi-use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    SingleUse,
    MultiUse,
}

/// Spending controls enforced at the card level by the issuing provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardControls {
    /// Max amount per individual transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_transaction: Option<Decimal>,
    /// Max cumulative amount per billing cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_cycle: Option<Decimal>,
    /// Merchant Category Codes the card is allowed to transact with.
    /// Empty means all MCCs are allowed.
    pub allowed_mcc_codes: Vec<String>,
    /// The currency this card transacts in.
    pub currency: Currency,
}

/// The lifecycle status of a virtual card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    Active,
    Frozen,
    Cancelled,
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_status_serde() {
        let s = CardStatus::Frozen;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"frozen\"");
    }

    #[test]
    fn card_type_serde() {
        let t = CardType::SingleUse;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"single_use\"");
    }
}
