use thiserror::Error;

/// Errors that can occur during provider operations.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider request failed: {0}")]
    RequestFailed(String),

    #[error("provider returned unexpected response: {0}")]
    UnexpectedResponse(String),

    #[error("provider timeout after {0}ms")]
    Timeout(u64),

    #[error("provider not found: {0}")]
    NotFound(String),

    #[error("card operation failed: {0}")]
    CardError(String),

    #[error("provider unavailable: {0}")]
    Unavailable(String),

    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),
}
