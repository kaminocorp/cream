use axum::http::{HeaderName, HeaderValue, Request};
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use uuid::Uuid;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Generates UUIDv7 request IDs (time-sortable, unique).
#[derive(Clone)]
pub struct UuidV7RequestId;

impl MakeRequestId for UuidV7RequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = Uuid::now_v7().to_string();
        Some(RequestId::new(HeaderValue::from_str(&id).ok()?))
    }
}

/// Returns a layer that sets `X-Request-Id` on incoming requests (if absent)
/// and propagates it to responses.
pub fn set_request_id_layer() -> SetRequestIdLayer<UuidV7RequestId> {
    SetRequestIdLayer::new(X_REQUEST_ID.clone(), UuidV7RequestId)
}

/// Returns a layer that copies `X-Request-Id` from the request to the response.
pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(X_REQUEST_ID.clone())
}
