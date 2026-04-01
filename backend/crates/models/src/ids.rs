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

// ---------------------------------------------------------------------------
// IdempotencyKey — String-based, not UUID-based
// ---------------------------------------------------------------------------

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
        Ok(Self(s))
    }
}

impl IdempotencyKey {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        assert!(!key.is_empty(), "IdempotencyKey must not be empty");
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
}
