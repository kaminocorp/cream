use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use cream_audit::AuditError;
use cream_models::prelude::*;
use cream_policy::PolicyError;
use cream_providers::ProviderError;
use cream_router::RoutingError;

/// Unified error type for the API layer. Each variant maps to a specific
/// HTTP status code and produces a JSON body with a machine-readable
/// `error_code` and human-readable `message`.
#[derive(Debug)]
pub enum ApiError {
    /// 400 — malformed request, validation failure.
    ValidationError(String),
    /// 401 — missing or invalid credentials.
    Unauthorized,
    /// 403 — authenticated but insufficient permissions (e.g., viewer role).
    Forbidden(String),
    /// 403 — policy engine blocked the payment.
    PolicyBlocked {
        rule_ids: Vec<PolicyRuleId>,
        reason: String,
    },
    /// 404 — resource not found.
    NotFound(String),
    /// 409 — duplicate idempotency key.
    IdempotencyConflict(PaymentId),
    /// 422 — justification failed structural checks.
    JustificationInvalid(String),
    /// 429 — rate limit exceeded.
    RateLimited { retry_after_secs: u64 },
    /// 500 — unexpected internal error.
    Internal(anyhow::Error),
    /// 502 — upstream provider returned an error.
    ProviderFailure(ProviderError),
    /// 503 — all providers unavailable or circuit-broken.
    AllProvidersUnavailable,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::PolicyBlocked { .. } => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::IdempotencyConflict(_) => StatusCode::CONFLICT,
            Self::JustificationInvalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ProviderFailure(_) => StatusCode::BAD_GATEWAY,
            Self::AllProvidersUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::PolicyBlocked { .. } => "POLICY_BLOCKED",
            Self::NotFound(_) => "NOT_FOUND",
            Self::IdempotencyConflict(_) => "IDEMPOTENCY_CONFLICT",
            Self::JustificationInvalid(_) => "JUSTIFICATION_INVALID",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::ProviderFailure(_) => "PROVIDER_ERROR",
            Self::AllProvidersUnavailable => "ALL_PROVIDERS_UNAVAILABLE",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::ValidationError(msg) => msg.clone(),
            Self::Unauthorized => "invalid or missing credentials".to_string(),
            Self::Forbidden(msg) => msg.clone(),
            Self::PolicyBlocked { reason, .. } => reason.clone(),
            Self::NotFound(resource) => format!("{resource} not found"),
            Self::IdempotencyConflict(id) => {
                format!("duplicate idempotency key; existing payment: {id}")
            }
            Self::JustificationInvalid(msg) => msg.clone(),
            Self::RateLimited { retry_after_secs } => {
                format!("rate limit exceeded; retry after {retry_after_secs}s")
            }
            Self::Internal(_) => "an internal error occurred".to_string(),
            Self::ProviderFailure(_) => {
                "payment provider error — see server logs for details".to_string()
            }
            Self::AllProvidersUnavailable => {
                "no payment providers are currently available".to_string()
            }
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.error_code(), self.message())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.message();

        // Log server errors at error level; client errors at debug.
        match &self {
            Self::Internal(e) => tracing::error!(error = %e, error_code, "internal server error"),
            Self::Forbidden(msg) => {
                tracing::warn!(error_code, %msg, "forbidden")
            }
            Self::ProviderFailure(e) => {
                tracing::warn!(error = %e, error_code, "provider error")
            }
            Self::AllProvidersUnavailable => {
                tracing::warn!(error_code, "all providers unavailable")
            }
            _ => tracing::debug!(error_code, %message, "client error"),
        }

        let mut details = serde_json::Map::new();
        if let Self::PolicyBlocked { rule_ids, .. } = &self {
            let ids: Vec<String> = rule_ids.iter().map(|id| id.to_string()).collect();
            details.insert("rule_ids".to_string(), serde_json::json!(ids));
        }
        if let Self::RateLimited { retry_after_secs } = &self {
            details.insert(
                "retry_after_secs".to_string(),
                serde_json::json!(retry_after_secs),
            );
        }

        let body = serde_json::json!({
            "error_code": error_code,
            "message": message,
            "details": details,
        });

        // For RateLimited, set the Retry-After header per RFC 7231 §7.1.3.
        // HTTP clients and agent frameworks use this header to schedule retries
        // without needing to parse the JSON body.
        if let Self::RateLimited { retry_after_secs } = &self {
            let mut response = (status, axum::Json(body)).into_response();
            if let Ok(value) =
                axum::http::HeaderValue::from_str(&retry_after_secs.to_string())
            {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
            return response;
        }

        (status, axum::Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// From impls: convert crate-level errors into ApiError
// ---------------------------------------------------------------------------

impl From<PolicyError> for ApiError {
    fn from(e: PolicyError) -> Self {
        Self::Internal(anyhow::anyhow!("policy engine error: {e}"))
    }
}

impl From<RoutingError> for ApiError {
    fn from(e: RoutingError) -> Self {
        match e {
            RoutingError::NoViableProvider | RoutingError::AllProvidersExhausted => {
                Self::AllProvidersUnavailable
            }
            RoutingError::IdempotencyConflict(key) => {
                Self::ValidationError(format!("idempotency conflict for key: {key}"))
            }
            RoutingError::Provider(pe) => Self::ProviderFailure(pe),
            other => Self::Internal(anyhow::anyhow!("routing error: {other}")),
        }
    }
}

impl From<AuditError> for ApiError {
    fn from(e: AuditError) -> Self {
        match e {
            AuditError::NotFound(id) => Self::NotFound(format!("audit entry {id}")),
            other => Self::Internal(anyhow::anyhow!("audit error: {other}")),
        }
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::Unauthorized => Self::Unauthorized,
            DomainError::NotFound(msg) => Self::NotFound(msg),
            DomainError::IdempotencyConflict(_) => {
                Self::ValidationError(format!("domain error: {e}"))
            }
            DomainError::InvalidStateTransition { .. } => {
                Self::ValidationError(format!("invalid state transition: {e}"))
            }
            other => Self::Internal(anyhow::anyhow!("domain error: {other}")),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(anyhow::anyhow!("database error: {e}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_is_400() {
        let err = ApiError::ValidationError("bad input".into());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn unauthorized_is_401() {
        let err = ApiError::Unauthorized;
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn policy_blocked_is_403() {
        let err = ApiError::PolicyBlocked {
            rule_ids: vec![],
            reason: "blocked".into(),
        };
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(err.error_code(), "POLICY_BLOCKED");
    }

    #[test]
    fn forbidden_is_403() {
        let err = ApiError::Forbidden("admin role required".into());
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(err.error_code(), "FORBIDDEN");
    }

    #[test]
    fn not_found_is_404() {
        let err = ApiError::NotFound("payment".into());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn idempotency_conflict_is_409() {
        let err = ApiError::IdempotencyConflict(PaymentId::new());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
    }

    #[test]
    fn justification_invalid_is_422() {
        let err = ApiError::JustificationInvalid("too short".into());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn rate_limited_is_429() {
        let err = ApiError::RateLimited {
            retry_after_secs: 30,
        };
        assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn rate_limited_includes_retry_after_header() {
        let err = ApiError::RateLimited {
            retry_after_secs: 45,
        };
        let response = err.into_response();
        let header = response.headers().get(axum::http::header::RETRY_AFTER);
        assert!(header.is_some(), "Retry-After header must be present on 429");
        assert_eq!(header.unwrap(), "45");
    }

    #[test]
    fn internal_is_500() {
        let err = ApiError::Internal(anyhow::anyhow!("boom"));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn provider_error_is_502() {
        let err = ApiError::ProviderFailure(ProviderError::Timeout(5000));
        assert_eq!(err.status_code(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn all_providers_unavailable_is_503() {
        let err = ApiError::AllProvidersUnavailable;
        assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
