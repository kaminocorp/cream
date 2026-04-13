use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

// ---------------------------------------------------------------------------
// Macro: generates a newtype wrapper around Uuid with prefixed Display/FromStr
// and Serde that serializes as "prefix_<uuid>" strings.
// ---------------------------------------------------------------------------

macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new ID with a random UUIDv7 (time-sortable).
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wrap an existing UUID.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Return the inner UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// The string prefix for this ID type.
            pub const PREFIX: &str = $prefix;
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}_{}", $prefix, self.0.as_hyphenated())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self)
            }
        }

        impl FromStr for $name {
            type Err = DomainError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let expected_prefix = concat!($prefix, "_");
                let uuid_str = s.strip_prefix(expected_prefix).ok_or_else(|| {
                    DomainError::InvalidIdFormat(format!(
                        "expected prefix '{}' but got '{}'",
                        expected_prefix, s
                    ))
                })?;
                let uuid = Uuid::from_str(uuid_str).map_err(|e| {
                    DomainError::InvalidIdFormat(format!("invalid UUID in '{}': {}", s, e))
                })?;
                Ok(Self(uuid))
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Self::from_str(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Typed IDs
// ---------------------------------------------------------------------------

typed_id!(PaymentId, "pay");
typed_id!(AgentId, "agt");
typed_id!(AgentProfileId, "prof");
typed_id!(PolicyRuleId, "rule");
typed_id!(AuditEntryId, "aud");
typed_id!(VirtualCardId, "card");
typed_id!(WebhookEndpointId, "whk");
typed_id!(OperatorId, "opr");

// ---------------------------------------------------------------------------
// IdempotencyKey — String-based, not UUID-based
// ---------------------------------------------------------------------------

/// Maximum allowed length for an [`IdempotencyKey`].
/// Idempotency keys are indexed in Redis and persisted to the database —
/// unbounded keys would bloat both stores.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IdempotencyKey(String);

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Err(serde::de::Error::custom(
                "idempotency_key must not be empty",
            ));
        }
        if s.len() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(serde::de::Error::custom(format!(
                "idempotency_key exceeds maximum length of {MAX_IDEMPOTENCY_KEY_LEN} (got {})",
                s.len()
            )));
        }
        Ok(Self(s))
    }
}

impl IdempotencyKey {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        assert!(!key.is_empty(), "IdempotencyKey must not be empty");
        assert!(
            key.len() <= MAX_IDEMPOTENCY_KEY_LEN,
            "IdempotencyKey exceeds maximum length of {MAX_IDEMPOTENCY_KEY_LEN} (got {})",
            key.len()
        );
        Self(key)
    }

    /// Fallible constructor for untrusted input.
    pub fn try_new(key: impl Into<String>) -> Result<Self, DomainError> {
        let key = key.into();
        if key.is_empty() {
            return Err(DomainError::InvalidIdFormat(
                "IdempotencyKey must not be empty".to_string(),
            ));
        }
        if key.len() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(DomainError::InvalidIdFormat(format!(
                "IdempotencyKey exceeds maximum length of {MAX_IDEMPOTENCY_KEY_LEN} (got {})",
                key.len()
            )));
        }
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "idem_{}", self.0)
    }
}

impl FromStr for IdempotencyKey {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = s.strip_prefix("idem_").ok_or_else(|| {
            DomainError::InvalidIdFormat(format!("expected prefix 'idem_' but got '{}'", s))
        })?;
        if key.is_empty() {
            return Err(DomainError::InvalidIdFormat(
                "IdempotencyKey must not be empty after prefix".to_string(),
            ));
        }
        if key.len() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(DomainError::InvalidIdFormat(format!(
                "IdempotencyKey exceeds maximum length of {MAX_IDEMPOTENCY_KEY_LEN} (got {})",
                key.len()
            )));
        }
        Ok(Self(key.to_owned()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_id_roundtrip() {
        let id = PaymentId::new();
        let s = id.to_string();
        assert!(s.starts_with("pay_"));
        let parsed: PaymentId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn agent_id_roundtrip() {
        let id = AgentId::new();
        let s = id.to_string();
        assert!(s.starts_with("agt_"));
        let parsed: AgentId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn wrong_prefix_rejected() {
        let id = PaymentId::new();
        let s = id.to_string();
        let bad = s.replace("pay_", "agt_");
        assert!(bad.parse::<PaymentId>().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let id = PaymentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: PaymentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn idempotency_key_roundtrip() {
        let key = IdempotencyKey::new("abc-123");
        assert_eq!(key.to_string(), "idem_abc-123");
        let parsed: IdempotencyKey = "idem_abc-123".parse().unwrap();
        assert_eq!(key, parsed);
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn idempotency_key_rejects_empty_new() {
        let _ = IdempotencyKey::new("");
    }

    #[test]
    fn idempotency_key_try_new_rejects_empty() {
        let result = IdempotencyKey::try_new("");
        assert!(result.is_err());
    }

    #[test]
    fn idempotency_key_deserialize_rejects_empty() {
        let json = serde_json::json!("");
        let result: Result<IdempotencyKey, _> = serde_json::from_value(json);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must not be empty"));
    }

    // -----------------------------------------------------------------------
    // Phase 7.5: IdempotencyKey FromStr rejects empty after prefix strip
    // -----------------------------------------------------------------------

    #[test]
    fn idempotency_key_from_str_rejects_prefix_only() {
        let result: Result<IdempotencyKey, _> = "idem_".parse();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn idempotency_key_from_str_accepts_valid() {
        let result: Result<IdempotencyKey, _> = "idem_abc-123".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "abc-123");
    }

    // -----------------------------------------------------------------------
    // Phase 7.8: IdempotencyKey max length validation
    // -----------------------------------------------------------------------

    #[test]
    fn idempotency_key_try_new_rejects_oversized() {
        let long = "x".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1);
        let result = IdempotencyKey::try_new(long);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    fn idempotency_key_try_new_at_limit_accepted() {
        let exact = "y".repeat(MAX_IDEMPOTENCY_KEY_LEN);
        let result = IdempotencyKey::try_new(exact);
        assert!(result.is_ok());
    }

    #[test]
    fn idempotency_key_deserialize_rejects_oversized() {
        let long = "z".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1);
        let json = serde_json::Value::String(long);
        let result: Result<IdempotencyKey, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    fn idempotency_key_from_str_rejects_oversized() {
        let long = format!("idem_{}", "k".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1));
        let result: Result<IdempotencyKey, _> = long.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("maximum length"));
    }

    #[test]
    #[should_panic(expected = "maximum length")]
    fn idempotency_key_new_panics_on_oversized() {
        let long = "p".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1);
        let _ = IdempotencyKey::new(long);
    }
}
