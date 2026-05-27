//! `GET /api/v1/monitoring/strategies/:id/orders` handler.
//!
//! Validates the path/query parameters, hands the typed input to the
//! [`application::use_cases::GetOrders`] use case, and serialises the
//! resulting [`domain::order::Order`] series as a JSON array of `OrderDto`.
//!
//! The DTO shape is compact on purpose — only the columns the chart layer
//! needs to render `buy` / `sell` markers. The `price` field is the
//! `COALESCE(limit_price, expected_price)` derivation explained on the
//! [`OrderDto::price`] doc-comment.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use domain::order::{MAX_ORDER_ROWS, OrderQueryError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use application::ports::RepositoryError;
use application::use_cases::{GetOrdersError, GetOrdersInput};

use super::state::AppState;

/// Raw query parameters captured by axum's `Query` extractor.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct OrderQueryParams {
    /// Inclusive lower bound on `created_at` (RFC3339).
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `created_at` (RFC3339). Defaults to `Clock::now()`.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// Row cap (defaults to [`MAX_ORDER_ROWS`], rejected above [`MAX_ORDER_ROWS`]).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// HTTP DTO returned in the JSON array for the orders endpoint.
///
/// `Decimal -> f64` happens at this boundary only (see candles handler
/// rationale). For OHLC magnitudes the worst-case relative error
/// (~2.2e-16) is invisible at chart pixel resolution; the full-precision
/// `Decimal` stays in the domain and persistence layers.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrderDto {
    /// Primary key.
    pub order_id: Uuid,
    /// Foreign key to `strategy_decisions(id)`.
    pub decision_id: Uuid,
    /// `buy` or `sell` (DB enum).
    #[schema(example = "buy")]
    pub side: String,
    /// Best-effort price: `COALESCE(limit_price, expected_price)`.
    ///
    /// - For `limit` / `amend` orders, this is the explicit limit price.
    /// - For `market` orders, this is the `expected_price` recorded by the
    ///   strategy engine just before submission, when available.
    /// - `null` when neither is known (e.g. `market` without expected price).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    /// Base-asset quantity.
    pub quantity: f64,
    /// Lifecycle status (`submitted`, `filled`, `canceled`, …).
    #[schema(example = "filled")]
    pub status: String,
    /// Wall-clock time the row was inserted (RFC3339).
    pub created_at: DateTime<Utc>,
}

impl From<domain::order::Order> for OrderDto {
    fn from(o: domain::order::Order) -> Self {
        // `Decimal::try_into::<f64>()` can fail only on absurd magnitudes
        // (>= 2^308). For exchange-sourced prices/quantities this is
        // unreachable; we still fall back to `0.0` (consistent with candles).
        let cast = |d: rust_decimal::Decimal| -> f64 {
            <rust_decimal::Decimal as TryInto<f64>>::try_into(d).unwrap_or(0.0)
        };
        Self {
            order_id: o.order_id,
            decision_id: o.decision_id,
            side: o.side.as_str().to_string(),
            price: o.price.map(cast),
            quantity: cast(o.quantity),
            status: o.status.as_str().to_string(),
            created_at: o.created_at,
        }
    }
}

/// JSON error body returned on 4xx / 5xx by the orders endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrderErrorBody {
    /// Machine-readable error discriminator (e.g. `invalid_range`).
    pub error: &'static str,
    /// Human-readable explanation (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Echoes the caller-supplied row count for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<usize>,
    /// Maximum row count accepted by the endpoint for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// Whitelisted `side` values, surfaced for `invalid_side` (currently
    /// unused — the handler does not parse `side` from input, but kept for
    /// schema symmetry with the candles error body).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<&'static [&'static str]>,
}

impl OrderErrorBody {
    fn simple(error: &'static str, message: impl Into<String>) -> Self {
        Self {
            error,
            message: Some(message.into()),
            requested: None,
            max: None,
            allowed: None,
        }
    }
}

/// All error responses produced by the orders handler.
#[derive(Debug)]
pub enum OrdersApiError {
    /// `from >= to` (after defaulting `to` to `Clock::now()`).
    InvalidRange,
    /// `limit` was zero.
    InvalidLimit,
    /// `limit` exceeded [`MAX_ORDER_ROWS`].
    TooManyRows { requested: usize },
    /// Downstream datastore unreachable.
    DbUnavailable(String),
    /// Unexpected internal error (schema drift, decode failure, …).
    Internal(String),
}

impl IntoResponse for OrdersApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRange => (
                StatusCode::BAD_REQUEST,
                Json(OrderErrorBody::simple(
                    "invalid_range",
                    "`from` must be strictly before `to`",
                )),
            )
                .into_response(),
            Self::InvalidLimit => (
                StatusCode::BAD_REQUEST,
                Json(OrderErrorBody::simple(
                    "invalid_limit",
                    "`limit` must be strictly positive",
                )),
            )
                .into_response(),
            Self::TooManyRows { requested } => (
                StatusCode::BAD_REQUEST,
                Json(OrderErrorBody {
                    error: "too_many_rows",
                    message: Some(format!("requested {requested} rows, max {MAX_ORDER_ROWS}")),
                    requested: Some(requested),
                    max: Some(MAX_ORDER_ROWS),
                    allowed: None,
                }),
            )
                .into_response(),
            Self::DbUnavailable(detail) => {
                tracing::warn!(error = %detail, "orders repository unavailable");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(OrderErrorBody::simple(
                        "service_unavailable",
                        "orders datastore is temporarily unreachable",
                    )),
                )
                    .into_response()
            }
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "orders handler internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(OrderErrorBody::simple(
                        "internal_error",
                        "internal server error",
                    )),
                )
                    .into_response()
            }
        }
    }
}

impl From<GetOrdersError> for OrdersApiError {
    fn from(err: GetOrdersError) -> Self {
        match err {
            GetOrdersError::Domain(OrderQueryError::InvalidRange) => Self::InvalidRange,
            GetOrdersError::Domain(OrderQueryError::InvalidLimit) => Self::InvalidLimit,
            GetOrdersError::Domain(OrderQueryError::TooManyRows { requested, .. }) => {
                Self::TooManyRows { requested }
            }
            GetOrdersError::Repository(RepositoryError::Unavailable(d)) => Self::DbUnavailable(d),
            GetOrdersError::Repository(RepositoryError::Internal(d)) => Self::Internal(d),
        }
    }
}

/// Handler for `GET /api/v1/monitoring/strategies/{id}/orders`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/strategies/{id}/orders",
    tag = "monitoring",
    params(
        ("id" = Uuid, Path, description = "Strategy identifier"),
        OrderQueryParams,
    ),
    responses(
        (status = 200, description = "Orders for the strategy on the requested window, ordered by ascending `created_at`.", body = [OrderDto]),
        (status = 400, description = "Invalid range, limit out of bounds, or non-UUID path parameter.", body = OrderErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = OrderErrorBody),
        (status = 500, description = "Unexpected internal error (e.g. schema drift).", body = OrderErrorBody),
    ),
    security(
        ("x_api_key" = [])
    ),
)]
pub async fn get_orders(
    State(state): State<AppState>,
    Path(strategy_id): Path<Uuid>,
    Query(params): Query<OrderQueryParams>,
) -> Result<Json<Vec<OrderDto>>, OrdersApiError> {
    let input = GetOrdersInput {
        strategy_id,
        from: params.from,
        to: params.to,
        limit: params.limit,
    };

    let series = state.get_orders.run(input).await?;
    let dtos = series.orders.into_iter().map(OrderDto::from).collect();
    Ok(Json(dtos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use application::ports::{
        Clock, HealthCheckError, HealthChecker, OrderQuery, OrderRepository, RepositoryError,
    };
    use application::use_cases::{GetCandles, GetOrders, ReadinessProbe};
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use chrono::TimeZone;
    use domain::order::{Order, OrderSide, OrderStatus};
    use rust_decimal_macros::dec;
    use tower::ServiceExt;

    use crate::inbound::http::AppState;

    struct DummyHealth;

    #[async_trait]
    impl HealthChecker for DummyHealth {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    struct DummyCandleRepo;

    #[async_trait]
    impl application::ports::CandleRepository for DummyCandleRepo {
        async fn fetch_aggregated(
            &self,
            _q: &application::ports::CandleQuery,
        ) -> Result<Vec<domain::candle::Candle>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct StubRepo {
        orders: Vec<Order>,
    }

    #[async_trait]
    impl OrderRepository for StubRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _q: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Ok(self.orders.clone())
        }
    }

    struct DownRepo;

    #[async_trait]
    impl OrderRepository for DownRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _q: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    struct BrokenRepo;

    #[async_trait]
    impl OrderRepository for BrokenRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _q: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Err(RepositoryError::Internal("simulated schema drift".into()))
        }
    }

    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn state_with(repo: Arc<dyn OrderRepository>, clock: Arc<dyn Clock>) -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(DummyHealth))),
            api_key: Arc::new(b"unused".to_vec()),
            get_candles: Arc::new(GetCandles::new(Arc::new(DummyCandleRepo), clock.clone())),
            get_orders: Arc::new(GetOrders::new(repo, clock)),
        }
    }

    fn router_for(state: AppState) -> Router {
        // Mount the handler on its real path so assertions are explicit.
        Router::new()
            .route("/api/v1/monitoring/strategies/{id}/orders", get(get_orders))
            .with_state(state)
    }

    fn sample_order(side: OrderSide, price: Option<rust_decimal::Decimal>) -> Order {
        Order {
            order_id: Uuid::from_u128(0x1111_1111_1111_1111_1111_1111_1111_1111),
            decision_id: Uuid::from_u128(0x2222_2222_2222_2222_2222_2222_2222_2222),
            side,
            price,
            quantity: dec!(0.5),
            status: OrderStatus::Filled,
            created_at: Utc.with_ymd_and_hms(2026, 5, 27, 14, 0, 0).unwrap(),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn happy_path_returns_200_with_dtos() {
        let state = state_with(
            Arc::new(StubRepo {
                orders: vec![
                    sample_order(OrderSide::Buy, Some(dec!(75000.50))),
                    sample_order(OrderSide::Sell, None),
                ],
            }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 18, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = body.as_array().expect("response should be a JSON array");
        assert_eq!(arr.len(), 2);

        // First order: buy with explicit price.
        let buy = &arr[0];
        assert_eq!(buy["side"], "buy");
        assert_eq!(buy["status"], "filled");
        assert!((buy["price"].as_f64().unwrap() - 75000.50).abs() < 1e-6);
        assert!((buy["quantity"].as_f64().unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(buy["created_at"].as_str().unwrap(), "2026-05-27T14:00:00Z");
        assert_eq!(
            buy["order_id"].as_str().unwrap(),
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(
            buy["decision_id"].as_str().unwrap(),
            "22222222-2222-2222-2222-222222222222"
        );

        // Second order: sell with `price = None` → field absent from JSON.
        let sell = &arr[1];
        assert_eq!(sell["side"], "sell");
        assert!(
            sell.get("price").is_none() || sell["price"].is_null(),
            "price should be absent or null, got {:?}",
            sell.get("price")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_inverted_range_with_400() {
        let state = state_with(
            Arc::new(StubRepo { orders: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T05:00:00Z&to=2026-05-27T00:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "invalid_range");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_over_cap_with_400() {
        let state = state_with(
            Arc::new(StubRepo { orders: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z&limit=5001",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "too_many_rows");
        assert_eq!(body["max"], 5000);
        assert_eq!(body["requested"], 5001);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_zero_limit_with_400() {
        let state = state_with(
            Arc::new(StubRepo { orders: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z&limit=0",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "invalid_limit");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_non_uuid_path_with_400() {
        let state = state_with(
            Arc::new(StubRepo { orders: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/not-a-uuid/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn surfaces_503_when_repo_unavailable() {
        let state = state_with(
            Arc::new(DownRepo),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "service_unavailable");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn surfaces_500_when_repo_internal() {
        let state = state_with(
            Arc::new(BrokenRepo),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z&to=2026-05-27T23:59:59Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "internal_error");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn defaults_to_now_when_to_is_omitted() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 18, 0, 0).unwrap();
        let state = state_with(
            Arc::new(StubRepo {
                orders: vec![sample_order(OrderSide::Buy, Some(dec!(1)))],
            }),
            Arc::new(FixedClock(now)),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-05-27T00:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
