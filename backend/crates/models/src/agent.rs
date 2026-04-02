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
/// Custom `Deserialize` validates that spending limits are strictly positive
/// and escalation threshold (if set) is strictly positive. Zero limits would
/// silently block all payments; negative limits are semantically invalid.
#[derive(Debug, Clone, Serialize)]
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

impl<'de> Deserialize<'de> for AgentProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            id: AgentProfileId,
            name: String,
            version: i32,
            max_per_transaction: Decimal,
            max_daily_spend: Decimal,
            max_weekly_spend: Decimal,
            max_monthly_spend: Decimal,
            allowed_categories: Vec<PaymentCategory>,
            allowed_rails: Vec<RailPreference>,
            geographic_restrictions: Vec<CountryCode>,
            escalation_threshold: Option<Decimal>,
            timezone: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let raw = Raw::deserialize(deserializer)?;

        fn validate_positive<E: serde::de::Error>(value: &Decimal, name: &str) -> Result<(), E> {
            if *value <= Decimal::ZERO {
                return Err(E::custom(format!("{name} must be positive, got {value}")));
            }
            Ok(())
        }

        validate_positive::<D::Error>(&raw.max_per_transaction, "max_per_transaction")?;
        validate_positive::<D::Error>(&raw.max_daily_spend, "max_daily_spend")?;
        validate_positive::<D::Error>(&raw.max_weekly_spend, "max_weekly_spend")?;
        validate_positive::<D::Error>(&raw.max_monthly_spend, "max_monthly_spend")?;

        if let Some(ref threshold) = raw.escalation_threshold {
            if *threshold <= Decimal::ZERO {
                return Err(serde::de::Error::custom(format!(
                    "escalation_threshold must be positive when set, got {threshold}"
                )));
            }
        }

        Ok(AgentProfile {
            id: raw.id,
            name: raw.name,
            version: raw.version,
            max_per_transaction: raw.max_per_transaction,
            max_daily_spend: raw.max_daily_spend,
            max_weekly_spend: raw.max_weekly_spend,
            max_monthly_spend: raw.max_monthly_spend,
            allowed_categories: raw.allowed_categories,
            allowed_rails: raw.allowed_rails,
            geographic_restrictions: raw.geographic_restrictions,
            escalation_threshold: raw.escalation_threshold,
            timezone: raw.timezone,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
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

    // -----------------------------------------------------------------------
    // Phase 7.1: AgentProfile spending limit validation
    // -----------------------------------------------------------------------

    fn sample_profile_json() -> serde_json::Value {
        let id = AgentProfileId::new();
        serde_json::json!({
            "id": id.to_string(),
            "name": "test_profile",
            "version": 1,
            "max_per_transaction": "500.00",
            "max_daily_spend": "2000.00",
            "max_weekly_spend": "10000.00",
            "max_monthly_spend": "30000.00",
            "allowed_categories": [],
            "allowed_rails": [],
            "geographic_restrictions": [],
            "created_at": "2026-04-01T00:00:00Z",
            "updated_at": "2026-04-01T00:00:00Z"
        })
    }

    #[test]
    fn agent_profile_valid_limits_accepted() {
        let json = sample_profile_json();
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_ok());
    }

    #[test]
    fn agent_profile_rejects_zero_max_per_transaction() {
        let mut json = sample_profile_json();
        json["max_per_transaction"] = serde_json::json!("0");
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("max_per_transaction"));
        assert!(err.contains("positive"));
    }

    #[test]
    fn agent_profile_rejects_negative_daily_spend() {
        let mut json = sample_profile_json();
        json["max_daily_spend"] = serde_json::json!("-100");
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("max_daily_spend"));
    }

    #[test]
    fn agent_profile_rejects_zero_weekly_spend() {
        let mut json = sample_profile_json();
        json["max_weekly_spend"] = serde_json::json!("0");
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_weekly_spend"));
    }

    #[test]
    fn agent_profile_rejects_negative_monthly_spend() {
        let mut json = sample_profile_json();
        json["max_monthly_spend"] = serde_json::json!("-1");
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_monthly_spend"));
    }

    #[test]
    fn agent_profile_rejects_zero_escalation_threshold() {
        let mut json = sample_profile_json();
        json["escalation_threshold"] = serde_json::json!("0");
        let result: Result<AgentProfile, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("escalation_threshold"));
    }

    #[test]
    fn agent_profile_accepts_none_escalation_threshold() {
        let json = sample_profile_json(); // no escalation_threshold field
        let profile: AgentProfile = serde_json::from_value(json).unwrap();
        assert!(profile.escalation_threshold.is_none());
    }
}
