use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;
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

/// Middleware that reads the `X-Request-Id` header (set by the layer above)
/// and records it as a field on the current tracing span. This causes every
/// log line emitted during the request to include `request_id=<uuid>` in
/// structured output, enabling correlation across the full request lifecycle.
pub async fn inject_request_id(
    request: Request<Body>,
    next: Next,
) -> Response {
    let request_id = request
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Record into the current span so all child spans and log events inherit it.
    tracing::Span::current().record("request_id", request_id.as_str());

    next.run(request).await
}
