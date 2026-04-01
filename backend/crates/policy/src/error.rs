use thiserror::Error;

/// Errors specific to the policy crate.
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("unknown rule type: {0}")]
    UnknownRuleType(String),

    #[error("condition evaluation error: {0}")]
    ConditionError(String),
}
