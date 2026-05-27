//! HTTP handlers for the unauthenticated probes.

use axum::{extract::State, http::StatusCode};

use application::use_cases::ReadinessOutcome;

use super::state::AppState;

/// Liveness probe — always returns `200 ok` once the process is up.
pub async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// Readiness probe — `200 ok` when the database is reachable, `503` otherwise.
pub async fn readyz(State(state): State<AppState>) -> (StatusCode, &'static str) {
    match state.readiness.run().await {
        ReadinessOutcome::Ready => (StatusCode::OK, "ok"),
        ReadinessOutcome::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "not ready"),
    }
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
        routing::get,
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

    struct AlwaysDown;

    #[async_trait]
    impl HealthChecker for AlwaysDown {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Err(HealthCheckError::Unavailable("nope".into()))
        }
    }

    fn router_for(probe: ReadinessProbe) -> Router {
        let state = AppState {
            readiness: Arc::new(probe),
            api_key: Arc::new(b"unused".to_vec()),
        };
        Router::new()
            .route("/healthz", get(healthz))
            .route("/readyz", get(readyz))
            .with_state(state)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn healthz_returns_200() {
        let app = router_for(ReadinessProbe::new(Arc::new(AlwaysOk)));
        let response = app
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn readyz_returns_200_when_dependency_ok() {
        let app = router_for(ReadinessProbe::new(Arc::new(AlwaysOk)));
        let response = app
            .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn readyz_returns_503_when_dependency_down() {
        let app = router_for(ReadinessProbe::new(Arc::new(AlwaysDown)));
        let response = app
            .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
