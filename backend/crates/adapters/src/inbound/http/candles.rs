//! `GET /api/v1/monitoring/candles` handler.
//!
//! Validates the query string, hands the typed input to the
//! [`application::use_cases::GetCandles`] use case, and serialises the
//! resulting [`domain::candle::Candle`] series as a JSON array of `CandleDto`.
//!
//! The DTO shape (`{ ts, o, h, l, c, v }`) mirrors what TradingView
//! Lightweight Charts consumes directly; `ts` is the bucket `open_time` as
//! RFC3339, and numeric fields are emitted as JSON numbers (`f64`) — see the
//! `Decimal → f64` conversion comment on [`CandleDto`] for the precision
//! trade-off.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use domain::candle::{CandleQueryError, MAX_CANDLE_POINTS, Timeframe};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use application::ports::RepositoryError;
use application::use_cases::{GetCandlesError, GetCandlesInput};

use super::state::AppState;

/// Raw query parameters captured by axum's `Query` extractor.
///
/// Strings (`exchange`, `symbol`, `timeframe`) are not parsed/validated at
/// extraction time — that happens inside the handler so each failure mode
/// maps to a precise JSON error body.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CandleQueryParams {
    /// Exchange identifier as stored in `candles_5s.exchange`.
    pub exchange: String,
    /// Symbol identifier as stored in `candles_5s.symbol`.
    pub symbol: String,
    /// Aggregation width (must be one of [`Timeframe::ALLOWED`]).
    pub timeframe: String,
    /// Inclusive lower bound on `open_time` (RFC3339).
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `open_time` (RFC3339). Defaults to `Clock::now()`.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
}

/// HTTP DTO returned in the JSON array.
///
/// We deliberately down-cast `Decimal` → `f64` at the serialisation boundary:
/// Lightweight Charts requires JS numbers, and the relative error introduced
/// (worst case ≈ 2.2e-16 for typical OHLC magnitudes) is invisible at chart
/// pixel resolution. The full-precision `Decimal` stays in the domain and the
/// persistence layer — only this struct loses precision, and it is the only
/// place that does.
#[derive(Debug, Serialize, ToSchema)]
pub struct CandleDto {
    /// Bucket start, RFC3339 (e.g. `2026-05-27T14:00:00Z`).
    pub ts: DateTime<Utc>,
    /// Open price.
    pub o: f64,
    /// High price.
    pub h: f64,
    /// Low price.
    pub l: f64,
    /// Close price.
    pub c: f64,
    /// Total traded volume in the bucket.
    pub v: f64,
}

impl From<domain::candle::Candle> for CandleDto {
    fn from(c: domain::candle::Candle) -> Self {
        // `Decimal::try_into::<f64>()` can fail only on absurd magnitudes
        // (>= 2^308). For OHLC values produced by an exchange this is
        // unreachable; we still fall back to `0.0` and log nothing here on
        // purpose (this would be a schema-invariant violation, not a
        // user-visible error).
        let cast = |d: rust_decimal::Decimal| -> f64 {
            <rust_decimal::Decimal as TryInto<f64>>::try_into(d).unwrap_or(0.0)
        };
        Self {
            ts: c.open_time,
            o: cast(c.open),
            h: cast(c.high),
            l: cast(c.low),
            c: cast(c.close),
            v: cast(c.volume),
        }
    }
}

/// JSON error body returned on 4xx / 5xx by the candles endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct CandleErrorBody {
    /// Machine-readable error discriminator (e.g. `invalid_timeframe`).
    pub error: &'static str,
    /// Human-readable explanation (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Echoes the caller-supplied bucket count for `too_many_points`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<usize>,
    /// Maximum bucket count accepted by the endpoint for `too_many_points`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// Whitelisted `timeframe` values, surfaced for `invalid_timeframe`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<&'static [&'static str]>,
}

impl CandleErrorBody {
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

/// All error responses produced by the candles handler.
#[derive(Debug)]
pub enum CandlesApiError {
    /// `timeframe` did not match the whitelist.
    InvalidTimeframe { input: String },
    /// `from >= to` (after defaulting `to` to `Clock::now()`).
    InvalidRange,
    /// `(to - from) / timeframe` exceeded [`MAX_CANDLE_POINTS`].
    TooManyPoints { requested: usize },
    /// Downstream datastore unreachable.
    DbUnavailable(String),
    /// Unexpected internal error (schema drift, decode failure, …).
    Internal(String),
}

impl IntoResponse for CandlesApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidTimeframe { input } => (
                StatusCode::BAD_REQUEST,
                Json(CandleErrorBody {
                    error: "invalid_timeframe",
                    message: Some(format!("`{input}` is not a supported timeframe")),
                    requested: None,
                    max: None,
                    allowed: Some(Timeframe::ALLOWED),
                }),
            )
                .into_response(),
            Self::InvalidRange => (
                StatusCode::BAD_REQUEST,
                Json(CandleErrorBody::simple(
                    "invalid_range",
                    "`from` must be strictly before `to`",
                )),
            )
                .into_response(),
            Self::TooManyPoints { requested } => (
                StatusCode::BAD_REQUEST,
                Json(CandleErrorBody {
                    error: "too_many_points",
                    message: Some(format!(
                        "requested {requested} buckets, max {MAX_CANDLE_POINTS}"
                    )),
                    requested: Some(requested),
                    max: Some(MAX_CANDLE_POINTS),
                    allowed: None,
                }),
            )
                .into_response(),
            Self::DbUnavailable(detail) => {
                tracing::warn!(error = %detail, "candles repository unavailable");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(CandleErrorBody::simple(
                        "service_unavailable",
                        "candle datastore is temporarily unreachable",
                    )),
                )
                    .into_response()
            }
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "candles handler internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CandleErrorBody::simple(
                        "internal_error",
                        "internal server error",
                    )),
                )
                    .into_response()
            }
        }
    }
}

impl From<GetCandlesError> for CandlesApiError {
    fn from(err: GetCandlesError) -> Self {
        match err {
            GetCandlesError::Domain(CandleQueryError::InvalidTimeframe(it)) => {
                Self::InvalidTimeframe { input: it.input }
            }
            GetCandlesError::Domain(CandleQueryError::InvalidRange) => Self::InvalidRange,
            GetCandlesError::Domain(CandleQueryError::TooManyPoints { requested, .. }) => {
                Self::TooManyPoints { requested }
            }
            GetCandlesError::Repository(RepositoryError::Unavailable(d)) => Self::DbUnavailable(d),
            GetCandlesError::Repository(RepositoryError::Internal(d)) => Self::Internal(d),
        }
    }
}

/// Handler for `GET /api/v1/monitoring/candles`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/candles",
    tag = "monitoring",
    params(CandleQueryParams),
    responses(
        (status = 200, description = "Aggregated OHLCV buckets ordered by ascending `ts`.", body = [CandleDto]),
        (status = 400, description = "Invalid timeframe, range or too many points.", body = CandleErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = CandleErrorBody),
        (status = 500, description = "Unexpected internal error (e.g. schema drift).", body = CandleErrorBody),
    ),
    security(
        ("x_api_key" = [])
    ),
)]
pub async fn get_candles(
    State(state): State<AppState>,
    Query(params): Query<CandleQueryParams>,
) -> Result<Json<Vec<CandleDto>>, CandlesApiError> {
    let timeframe = Timeframe::try_from(params.timeframe.as_str())
        .map_err(|err| CandlesApiError::InvalidTimeframe { input: err.input })?;

    let input = GetCandlesInput {
        exchange: params.exchange,
        symbol: params.symbol,
        timeframe,
        from: params.from,
        to: params.to,
    };

    let series = state.get_candles.run(input).await?;
    let dtos = series.candles.into_iter().map(CandleDto::from).collect();
    Ok(Json(dtos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use application::ports::{
        CandleQuery, CandleRepository, Clock, HealthCheckError, HealthChecker, OrderQuery,
        OrderRepository, RepositoryError,
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
    use domain::candle::Candle;
    use domain::order::Order;
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

    struct StubRepo {
        candles: Vec<Candle>,
    }

    #[async_trait]
    impl CandleRepository for StubRepo {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Ok(self.candles.clone())
        }
    }

    struct DownRepo;

    #[async_trait]
    impl CandleRepository for DownRepo {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    struct BrokenRepo;

    #[async_trait]
    impl CandleRepository for BrokenRepo {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Err(RepositoryError::Internal("simulated schema drift".into()))
        }
    }

    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
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

    fn state_with(repo: Arc<dyn CandleRepository>, clock: Arc<dyn Clock>) -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(DummyHealth))),
            api_key: Arc::new(b"unused".to_vec()),
            get_candles: Arc::new(GetCandles::new(repo, clock.clone())),
            get_orders: Arc::new(GetOrders::new(Arc::new(EmptyOrderRepo), clock)),
        }
    }

    fn router_for(state: AppState) -> Router {
        // Mount the handler on its real path to make the assertions explicit.
        Router::new()
            .route("/api/v1/monitoring/candles", get(get_candles))
            .with_state(state)
    }

    fn one_candle() -> Candle {
        Candle {
            open_time: Utc.with_ymd_and_hms(2026, 5, 27, 0, 0, 0).unwrap(),
            open: dec!(75000.50),
            high: dec!(75100.00),
            low: dec!(74950.00),
            close: dec!(75050.25),
            volume: dec!(12.345),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn happy_path_returns_200_with_dtos() {
        let state = state_with(
            Arc::new(StubRepo {
                candles: vec![one_candle()],
            }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
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
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        // Deserialise as `serde_json::Value` since `CandleDto` is only `Serialize`.
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = body.as_array().expect("response should be a JSON array");
        assert_eq!(arr.len(), 1);
        let o = arr[0]["o"].as_f64().unwrap();
        let h = arr[0]["h"].as_f64().unwrap();
        let l = arr[0]["l"].as_f64().unwrap();
        let c = arr[0]["c"].as_f64().unwrap();
        let v = arr[0]["v"].as_f64().unwrap();
        assert!((o - 75000.50).abs() < 1e-6);
        assert!((h - 75100.00).abs() < 1e-6);
        assert!((l - 74950.00).abs() < 1e-6);
        assert!((c - 75050.25).abs() < 1e-6);
        assert!((v - 12.345).abs() < 1e-6);
        assert_eq!(arr[0]["ts"].as_str().unwrap(), "2026-05-27T00:00:00Z");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_unknown_timeframe_with_400() {
        let state = state_with(
            Arc::new(StubRepo { candles: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=7s&from=2026-05-27T00:00:00Z&to=2026-05-27T05:00:00Z",
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
        assert_eq!(body["error"], "invalid_timeframe");
        assert!(body["allowed"].is_array());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_over_cap_with_400() {
        let state = state_with(
            Arc::new(StubRepo { candles: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        // 5s timeframe over a whole day → 17280 buckets, far above 5000.
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=5s&from=2026-05-26T00:00:00Z&to=2026-05-27T00:00:00Z",
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
        assert_eq!(body["error"], "too_many_points");
        assert_eq!(body["max"], 5000);
        assert!(body["requested"].as_u64().unwrap() > 5000);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_inverted_range_with_400() {
        let state = state_with(
            Arc::new(StubRepo { candles: vec![] }),
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            )),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=1h&from=2026-05-27T05:00:00Z&to=2026-05-27T00:00:00Z",
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
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=1h&from=2026-05-27T00:00:00Z&to=2026-05-27T05:00:00Z",
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
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=1h&from=2026-05-27T00:00:00Z&to=2026-05-27T05:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn defaults_to_now_when_to_is_omitted() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let state = state_with(
            Arc::new(StubRepo {
                candles: vec![one_candle()],
            }),
            Arc::new(FixedClock(now)),
        );
        let app = router_for(state);
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC\
                     &timeframe=1h&from=2026-05-27T00:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
