//! Axum router composition.
//!
//! Layout:
//!
//! - `GET /healthz` and `GET /readyz` are exposed **without** the
//!   `X-API-Key` middleware so that Kubernetes liveness/readiness probes
//!   (which do not set custom headers) can reach them.
//! - Everything under `/api/v1/monitoring` is wrapped by the api-key
//!   middleware. The monitoring router itself is empty for now; concrete
//!   endpoints arrive with issues #8 and following.

use axum::{Router, http::StatusCode, middleware, routing::get};

use super::{api_key::require_api_key, handlers, state::AppState};

/// Builds the top-level axum router with the shared [`AppState`] attached.
///
/// Routes under `/api/v1/monitoring` are wrapped by the `X-API-Key`
/// middleware. The monitoring sub-router currently holds only a catch-all
/// fallback that returns `404 Not Found` — concrete endpoints land with
/// issues #8 and following. The fallback is required so axum can attach the
/// middleware to actual route entries (a `route_layer` over an empty router
/// is a no-op in axum 0.8 and panics at runtime).
pub fn build_router(state: AppState) -> Router {
    let monitoring_router: Router<AppState> =
        Router::new()
            .fallback(monitoring_not_found)
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_api_key,
            ));

    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/readyz", get(handlers::readyz))
        .nest("/api/v1/monitoring", monitoring_router)
        .with_state(state)
}

/// Fallback for unmapped `/api/v1/monitoring/*` paths.
///
/// Runs **after** the api-key middleware, so callers with an invalid key
/// get `401` rather than `404` — exactly what we want to avoid leaking the
/// existence (or not) of a future endpoint to unauthenticated clients.
async fn monitoring_not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "not found")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use application::ports::{HealthCheckError, HealthChecker};
    use application::use_cases::ReadinessProbe;
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    struct AlwaysOk;

    #[async_trait]
    impl HealthChecker for AlwaysOk {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    fn test_state() -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(AlwaysOk))),
            api_key: Arc::new(b"dev-key-please-change-0123".to_vec()),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn healthz_is_unauthenticated() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn monitoring_without_key_is_401() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::get("/api/v1/monitoring/anything")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn monitoring_with_wrong_key_is_401() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::get("/api/v1/monitoring/anything")
                    .header("X-API-Key", "wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
