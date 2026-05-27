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

use super::{api_key::require_api_key, candles, handlers, orders, state::AppState};

/// Builds the top-level axum router with the shared [`AppState`] attached.
///
/// Routes under `/api/v1/monitoring` are wrapped by the `X-API-Key`
/// middleware. Concrete endpoints (currently: `GET /candles` from issue #8,
/// `GET /strategies/{id}/orders` from issue #10) are mounted on the
/// monitoring sub-router; the catch-all fallback returns `404 Not Found`
/// for unknown paths so axum can still attach the middleware to every leaf.
pub fn build_router(state: AppState) -> Router {
    let monitoring_router: Router<AppState> = Router::new()
        .route("/candles", get(candles::get_candles))
        .route("/strategies/{id}/orders", get(orders::get_orders))
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

    use application::ports::{
        CandleQuery, CandleRepository, Clock, HealthCheckError, HealthChecker, OrderQuery,
        OrderRepository, RepositoryError,
    };
    use application::use_cases::{GetCandles, GetOrders, ReadinessProbe};
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::{DateTime, Utc};
    use domain::candle::Candle;
    use domain::order::Order;
    use tower::ServiceExt;

    use super::*;

    struct AlwaysOk;

    #[async_trait]
    impl HealthChecker for AlwaysOk {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    struct EmptyRepo;

    #[async_trait]
    impl CandleRepository for EmptyRepo {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct EmptyOrderRepo;

    #[async_trait]
    impl OrderRepository for EmptyOrderRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _q: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct UtcNowClock;

    impl Clock for UtcNowClock {
        fn now(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn test_state() -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(AlwaysOk))),
            api_key: Arc::new(b"dev-key-please-change-0123".to_vec()),
            get_candles: Arc::new(GetCandles::new(Arc::new(EmptyRepo), Arc::new(UtcNowClock))),
            get_orders: Arc::new(GetOrders::new(
                Arc::new(EmptyOrderRepo),
                Arc::new(UtcNowClock),
            )),
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

    #[tokio::test(flavor = "current_thread")]
    async fn candles_without_key_is_401() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=1h&from=2026-05-27T00:00:00Z&to=2026-05-27T05:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
