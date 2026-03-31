use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, AgentProfileId};
use crate::justification::PaymentCategory;
use crate::payment::RailPreference;

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// An AI agent registered on the platform.
///
/// Each agent has an identity, belongs to a policy profile, and can be
/// independently suspended or revoked without affecting other agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub profile_id: AgentProfileId,
    pub name: String,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The operational status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is fully operational and can initiate payments.
    Active,
    /// Agent is temporarily disabled. Existing payments continue but no new
    /// payments are accepted.
    Suspended,
    /// Agent is permanently disabled. All credentials are invalidated.
    Revoked,
}

// ---------------------------------------------------------------------------
// Agent Profile
// ---------------------------------------------------------------------------

/// A policy profile that defines an agent's spending authority and constraints.
///
/// Multiple agents can share a profile. Profiles are versioned — every update
/// creates a new version and the change is logged in the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: AgentProfileId,
    pub name: String,
    pub version: i32,
    /// Maximum amount for a single transaction.
    pub max_per_transaction: Decimal,
    /// Maximum cumulative spend per day.
    pub max_daily_spend: Decimal,
    /// Maximum cumulative spend per week.
    pub max_weekly_spend: Decimal,
    /// Maximum cumulative spend per month.
    pub max_monthly_spend: Decimal,
    /// Payment categories this agent is allowed to use.
    /// Empty means all categories are allowed.
    pub allowed_categories: Vec<PaymentCategory>,
    /// Payment rails this agent is allowed to use.
    /// Empty means all rails are allowed.
    pub allowed_rails: Vec<RailPreference>,
    /// ISO 3166-1 alpha-2 country codes where payments can be sent.
    /// Empty means no geographic restrictions.
    pub geographic_restrictions: Vec<CountryCode>,
    /// Transactions above this amount trigger escalation to a human reviewer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_threshold: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Country Code
// ---------------------------------------------------------------------------

/// ISO 3166-1 alpha-2 country code (e.g., "SG", "US", "JP").
///
/// Stored as a simple 2-character string. Validation is intentionally light
/// in the models crate — the API layer enforces strict ISO compliance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountryCode(String);

impl CountryCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_serde() {
        let s = AgentStatus::Suspended;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"suspended\"");
    }

    #[test]
    fn country_code_display() {
        let sg = CountryCode::new("SG");
        assert_eq!(sg.to_string(), "SG");
        assert_eq!(sg.as_str(), "SG");
    }
}
