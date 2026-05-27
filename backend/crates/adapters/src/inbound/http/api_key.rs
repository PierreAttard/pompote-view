//! `X-API-Key` middleware (constant-time comparison).
//!
//! Applied to every route under `/api/v1/*`. The `/healthz` and `/readyz`
//! probes are exempted because Kubernetes liveness/readiness checks do not
//! send custom headers.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header::HeaderName},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use super::state::AppState;

/// Header name read by the middleware. Match must be case-insensitive
/// (axum's `HeaderMap` already normalises header names).
pub const API_KEY_HEADER: HeaderName = HeaderName::from_static("x-api-key");

/// Body returned on `401` — kept short to avoid leaking implementation details.
const UNAUTHORIZED_BODY: &str = "unauthorized";

/// Axum middleware function. Reject requests whose `X-API-Key` header is
/// missing or differs from the configured value.
pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(header_value) = req.headers().get(&API_KEY_HEADER) else {
        return unauthorized();
    };
    let provided = header_value.as_bytes();
    let expected = state.api_key.as_slice();

    // `ConstantTimeEq` short-circuits to `false` when lengths differ but still
    // performs a full comparison otherwise — no information leak via timing.
    if provided.ct_eq(expected).into() {
        next.run(req).await
    } else {
        unauthorized()
    }
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, UNAUTHORIZED_BODY).into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use application::ports::{HealthCheckError, HealthChecker};
    use application::use_cases::ReadinessProbe;
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    use super::*;

    struct DummyHealth;

    #[async_trait]
    impl HealthChecker for DummyHealth {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    fn test_state(api_key: &str) -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(DummyHealth))),
            api_key: Arc::new(api_key.as_bytes().to_vec()),
        }
    }

    fn protected_router(state: AppState) -> Router {
        Router::new()
            .route("/protected", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_api_key,
            ))
            .with_state(state)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_missing_header() {
        let app = protected_router(test_state("dev-key-please-change-0123"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_wrong_header() {
        let app = protected_router(test_state("dev-key-please-change-0123"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("X-API-Key", "wrong-value")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accepts_correct_header() {
        let app = protected_router(test_state("dev-key-please-change-0123"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("X-API-Key", "dev-key-please-change-0123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
