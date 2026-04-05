use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::http::Request;
use serde::de::DeserializeOwned;

use crate::error::ApiError;

/// A JSON body extractor that returns `ApiError::ValidationError` on
/// deserialization failure instead of Axum's default plain-text rejection.
///
/// This ensures all error responses conform to the `{ error_code, message, details }`
/// JSON contract.
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(ValidatedJson(value)),
            Err(rejection) => Err(ApiError::ValidationError(rejection.body_text())),
        }
    }
}
