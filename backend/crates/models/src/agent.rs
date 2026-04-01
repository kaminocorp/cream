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
    /// IANA timezone identifier (e.g., "Asia/Singapore", "America/New_York").
    /// Used by the TimeWindowEvaluator to convert UTC to the operator's local
    /// time when evaluating time-based rules. Defaults to UTC if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Country Code
// ---------------------------------------------------------------------------

/// ISO 3166-1 alpha-2 country code (e.g., "SG", "US", "JP").
///
/// Validated on construction: must be exactly 2 ASCII alphabetic characters.
/// Stored uppercase internally for consistent comparison.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CountryCode(String);

impl CountryCode {
    /// Create a new CountryCode. Panics if the code is not exactly 2 ASCII letters.
    ///
    /// Use `try_new()` for fallible construction from untrusted input.
    pub fn new(code: impl Into<String>) -> Self {
        let code = code.into();
        Self::validate(&code).unwrap_or_else(|e| panic!("invalid CountryCode: {e}"));
        Self(code.to_ascii_uppercase())
    }

    /// Fallible constructor for untrusted input. Returns an error if the code
    /// is not exactly 2 ASCII alphabetic characters.
    pub fn try_new(code: impl Into<String>) -> Result<Self, crate::error::DomainError> {
        let code = code.into();
        Self::validate(&code)?;
        Ok(Self(code.to_ascii_uppercase()))
    }

    fn validate(code: &str) -> Result<(), crate::error::DomainError> {
        if code.len() != 2 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(crate::error::DomainError::InvalidIdFormat(format!(
                "country code must be exactly 2 ASCII letters, got '{code}'"
            )));
        }
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Custom deserializer that validates the country code format.
impl<'de> Deserialize<'de> for CountryCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_new(s).map_err(serde::de::Error::custom)
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

    #[test]
    fn country_code_uppercases_on_construction() {
        let sg = CountryCode::new("sg");
        assert_eq!(sg.as_str(), "SG");
    }

    #[test]
    fn country_code_try_new_rejects_invalid() {
        assert!(CountryCode::try_new("").is_err());
        assert!(CountryCode::try_new("A").is_err());
        assert!(CountryCode::try_new("ABC").is_err());
        assert!(CountryCode::try_new("12").is_err());
        assert!(CountryCode::try_new("S1").is_err());
    }

    #[test]
    fn country_code_try_new_accepts_valid() {
        assert!(CountryCode::try_new("SG").is_ok());
        assert!(CountryCode::try_new("us").is_ok());
        assert_eq!(CountryCode::try_new("jp").unwrap().as_str(), "JP");
    }

    #[test]
    fn country_code_deserialize_validates() {
        let valid: Result<CountryCode, _> = serde_json::from_str("\"SG\"");
        assert!(valid.is_ok());

        let invalid: Result<CountryCode, _> = serde_json::from_str("\"GARBAGE\"");
        assert!(invalid.is_err());
    }

    #[test]
    #[should_panic(expected = "invalid CountryCode")]
    fn country_code_new_panics_on_invalid() {
        CountryCode::new("INVALID");
    }
}
