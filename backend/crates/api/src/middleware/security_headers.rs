//! Security response headers middleware (Phase 17-D).
//!
//! Adds defense-in-depth HTTP headers to every response. Although Cream is
//! primarily an API service (not serving HTML), these headers protect any
//! browser-facing surface (dashboard, error pages) and satisfy security
//! scanners and compliance checklists.

use axum::http::{HeaderValue, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Tower layer that wraps a service with [`SecurityHeadersService`].
#[derive(Clone)]
pub struct SecurityHeadersLayer {
    hsts_value: HeaderValue,
}

impl SecurityHeadersLayer {
    /// Create a new layer with the given HSTS `max-age` in seconds.
    pub fn new(hsts_max_age: u64) -> Self {
        let hsts_value = HeaderValue::from_str(&format!(
            "max-age={hsts_max_age}; includeSubDomains"
        ))
        .expect("HSTS header value is always valid ASCII");
        Self { hsts_value }
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            hsts_value: self.hsts_value.clone(),
        }
    }
}

/// Service that adds security headers to every response.
#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    hsts_value: HeaderValue,
}

impl<S, ReqBody, ResBody> Service<axum::http::Request<ReqBody>> for SecurityHeadersService<S>
where
    S: Service<axum::http::Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let hsts_value = self.hsts_value.clone();
        let mut inner = self.inner.clone();
        // https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            let mut response = inner.call(req).await?;
            let headers = response.headers_mut();

            headers.insert(
                axum::http::header::STRICT_TRANSPORT_SECURITY,
                hsts_value,
            );
            headers.insert(
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                axum::http::header::X_FRAME_OPTIONS,
                HeaderValue::from_static("DENY"),
            );
            // X-XSS-Protection: 0 disables the legacy XSS auditor in older
            // browsers which can itself introduce vulnerabilities.
            headers.insert(
                axum::http::header::X_XSS_PROTECTION,
                HeaderValue::from_static("0"),
            );
            headers.insert(
                axum::http::header::REFERRER_POLICY,
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );
            headers.insert(
                axum::http::header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
            );
            headers.insert(
                "permissions-policy",
                HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
            );

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    fn test_app() -> Router {
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(SecurityHeadersLayer::new(31_536_000))
    }

    #[tokio::test]
    async fn security_headers_present_on_response() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        let headers = response.headers();

        assert_eq!(
            headers.get("strict-transport-security").unwrap(),
            "max-age=31536000; includeSubDomains"
        );
        assert_eq!(
            headers.get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
        assert_eq!(headers.get("x-xss-protection").unwrap(), "0");
        assert_eq!(
            headers.get("referrer-policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            headers.get("content-security-policy").unwrap(),
            "default-src 'none'; frame-ancestors 'none'"
        );
        assert_eq!(
            headers.get("permissions-policy").unwrap(),
            "camera=(), microphone=(), geolocation=()"
        );
    }

    #[tokio::test]
    async fn hsts_max_age_configurable() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(SecurityHeadersLayer::new(86400));

        let req = axum::http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.headers().get("strict-transport-security").unwrap(),
            "max-age=86400; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn security_headers_on_404() {
        let app = test_app();
        let req = axum::http::Request::builder()
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        // Headers should be present even on error responses.
        assert!(response.headers().contains_key("x-content-type-options"));
        assert!(response.headers().contains_key("x-frame-options"));
    }
}
