use cream_models::prelude::AuditEntryId;
use thiserror::Error;

/// Errors specific to the audit crate.
///
/// These cover infrastructure failures (database, serialization) rather than
/// domain violations — domain errors live in `cream_models::DomainError`.
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("audit entry not found: {0}")]
    NotFound(AuditEntryId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_entry_id() {
        let id = AuditEntryId::new();
        let err = AuditError::NotFound(id);
        let msg = err.to_string();
        assert!(msg.starts_with("audit entry not found: aud_"));
    }

    #[test]
    fn serialization_error_from_serde() {
        let bad_json: Result<String, _> = serde_json::from_str("{invalid");
        let serde_err = bad_json.unwrap_err();
        let err = AuditError::from(serde_err);
        assert!(err.to_string().contains("serialization error"));
    }
}
