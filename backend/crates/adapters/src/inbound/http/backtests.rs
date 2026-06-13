//! Backtest monitoring HTTP handlers (Lot 10).
//!
//! Mounted under `/api/v1/monitoring/backtests` (behind the `X-API-Key`
//! middleware). Endpoints:
//!
//! - `GET /backtests` — run selector (optional `status`/`strategy_kind`/
//!   `exchange`/`symbol` filters), ordered `started_at DESC`.
//! - `GET /backtests/{run_id}` — one run + its opaque `config_snapshot`.
//! - `GET /backtests/{run_id}/orders|fills|decisions` — run-scoped series on
//!   the **`market_ts`** axis, `[from, to)` window, ordered ascending.
//!
//! `Decimal -> f64` happens at this boundary only (same rationale as the
//! candles/orders handlers). The `config_snapshot` and decision `snapshot`
//! JSONB blobs are forwarded as opaque JSON.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use application::ports::RepositoryError;
use application::use_cases::{GetBacktestError, GetBacktestSeriesInput, ListBacktestRunsInput};
use domain::backtest::{BacktestQueryError, BacktestStatus};

use super::state::AppState;

/// `Decimal -> f64` down-cast used by every DTO in this module.
///
/// Failure is unreachable for exchange-sourced magnitudes (`>= 2^308`); we
/// fall back to `0.0`, consistent with the candles/orders handlers.
fn cast(d: rust_decimal::Decimal) -> f64 {
    <rust_decimal::Decimal as TryInto<f64>>::try_into(d).unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Summary of a backtest run (list item).
#[derive(Debug, Serialize, ToSchema)]
pub struct BacktestRunSummaryDto {
    /// Primary key.
    pub id: Uuid,
    /// Strategy family label.
    #[schema(example = "ma_cross")]
    pub strategy_kind: String,
    /// Optional synthetic strategy id (nullable, no FK).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_id: Option<Uuid>,
    /// Exchange identifier.
    #[schema(example = "binance")]
    pub exchange: String,
    /// Symbol identifier.
    #[schema(example = "BTCUSDT")]
    pub symbol: String,
    /// Inclusive start of the replayed market window (RFC3339, market clock).
    pub data_range_start: DateTime<Utc>,
    /// Exclusive end of the replayed market window (RFC3339, market clock).
    pub data_range_end: DateTime<Utc>,
    /// Wall-clock time the run started (RFC3339).
    pub started_at: DateTime<Utc>,
    /// Wall-clock time the run ended; absent while `status = running`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Lifecycle state (`running`, `completed`, `failed`, `aborted`).
    #[schema(example = "completed")]
    pub status: String,
    /// Simulated slippage in basis points.
    pub slippage_bps: i32,
    /// Simulated execution latency in milliseconds.
    pub latency_ms: i32,
    /// Simulated Kraken taker fee in basis points.
    pub fee_kraken_bps: i32,
    /// Simulated Binance taker fee in basis points.
    pub fee_binance_bps: i32,
    /// Failure description when `status = failed`; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<domain::backtest::BacktestRun> for BacktestRunSummaryDto {
    fn from(r: domain::backtest::BacktestRun) -> Self {
        Self {
            id: r.id,
            strategy_kind: r.strategy_kind,
            strategy_id: r.strategy_id,
            exchange: r.exchange,
            symbol: r.symbol,
            data_range_start: r.data_range_start,
            data_range_end: r.data_range_end,
            started_at: r.started_at,
            ended_at: r.ended_at,
            status: r.status.as_str().to_string(),
            slippage_bps: r.slippage_bps,
            latency_ms: r.latency_ms,
            fee_kraken_bps: r.fee_kraken_bps,
            fee_binance_bps: r.fee_binance_bps,
            error: r.error,
        }
    }
}

/// A run with its opaque configuration snapshot (detail view).
#[derive(Debug, Serialize, ToSchema)]
pub struct BacktestRunDetailDto {
    /// The run summary.
    pub run: BacktestRunSummaryDto,
    /// Opaque, display-only configuration snapshot (`jsonb` object).
    #[schema(value_type = Object)]
    pub config_snapshot: Value,
}

impl From<domain::backtest::BacktestRunDetail> for BacktestRunDetailDto {
    fn from(d: domain::backtest::BacktestRunDetail) -> Self {
        Self {
            run: d.run.into(),
            config_snapshot: d.config_snapshot,
        }
    }
}

/// One backtest order (chart marker).
#[derive(Debug, Serialize, ToSchema)]
pub struct BacktestOrderDto {
    /// Primary key.
    pub order_id: Uuid,
    /// Foreign key to the decision.
    pub decision_id: Uuid,
    /// `buy` or `sell`.
    #[schema(example = "buy")]
    pub side: String,
    /// Best-effort price `COALESCE(limit_price, expected_price)`; `null` when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    /// Base-asset quantity.
    pub quantity: f64,
    /// Raw status string (free text — `orders_backtest.status` has no `CHECK`).
    #[schema(example = "filled")]
    pub status: String,
    /// Market-clock timestamp (RFC3339).
    pub market_ts: DateTime<Utc>,
}

impl From<domain::backtest::BacktestOrder> for BacktestOrderDto {
    fn from(o: domain::backtest::BacktestOrder) -> Self {
        Self {
            order_id: o.order_id,
            decision_id: o.decision_id,
            side: o.side.as_str().to_string(),
            price: o.price.map(cast),
            quantity: cast(o.quantity),
            status: o.status,
            market_ts: o.market_ts,
        }
    }
}

/// One trade fill.
#[derive(Debug, Serialize, ToSchema)]
pub struct FillDto {
    /// Primary key.
    pub fill_id: Uuid,
    /// Order that produced this fill.
    pub order_id: Uuid,
    /// `buy` or `sell`.
    #[schema(example = "sell")]
    pub side: String,
    /// Execution price.
    pub price: f64,
    /// Executed base-asset quantity.
    pub quantity: f64,
    /// Fee charged for this fill.
    pub fee: f64,
    /// Asset the fee was charged in.
    #[schema(example = "USDT")]
    pub fee_asset: String,
    /// Market-clock timestamp (RFC3339).
    pub market_ts: DateTime<Utc>,
}

impl From<domain::fill::Fill> for FillDto {
    fn from(f: domain::fill::Fill) -> Self {
        Self {
            fill_id: f.fill_id,
            order_id: f.order_id,
            side: f.side.as_str().to_string(),
            price: cast(f.price),
            quantity: cast(f.quantity),
            fee: cast(f.fee),
            fee_asset: f.fee_asset,
            market_ts: f.market_ts,
        }
    }
}

/// One strategy decision (marker + optional context snapshot).
#[derive(Debug, Serialize, ToSchema)]
pub struct DecisionDto {
    /// Primary key.
    pub decision_id: Uuid,
    /// Market-clock timestamp (RFC3339).
    pub market_ts: DateTime<Utc>,
    /// Free-text rationale recorded by the strategy engine.
    pub reason: String,
    /// Number of orders emitted by this decision.
    pub orders_count: i32,
    /// Opaque market-context snapshot (`jsonb`); absent when no context recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub snapshot: Option<Value>,
}

impl From<domain::decision::Decision> for DecisionDto {
    fn from(d: domain::decision::Decision) -> Self {
        Self {
            decision_id: d.decision_id,
            market_ts: d.market_ts,
            reason: d.reason,
            orders_count: d.orders_count,
            snapshot: d.snapshot,
        }
    }
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

/// Query parameters for the run listing.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RunListParams {
    /// Optional `status` filter (`running`/`completed`/`failed`/`aborted`).
    #[serde(default)]
    pub status: Option<String>,
    /// Optional `strategy_kind` filter.
    #[serde(default)]
    pub strategy_kind: Option<String>,
    /// Optional `exchange` filter.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Optional `symbol` filter.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Row cap (defaults to and rejected above `MAX_BACKTEST_RUNS`).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Query parameters for a run-scoped series (orders / fills / decisions).
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SeriesParams {
    /// Inclusive lower bound on `market_ts` (RFC3339).
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `market_ts` (RFC3339). Defaults to `Clock::now()`.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// Row cap (defaults to and rejected above `MAX_ORDER_ROWS`).
    #[serde(default)]
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// JSON error body for the backtest endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct BacktestErrorBody {
    /// Machine-readable discriminator (e.g. `invalid_range`).
    pub error: &'static str,
    /// Human-readable explanation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Echoes the requested row count for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<usize>,
    /// Applicable row cap for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// Whitelisted values for `invalid_status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<&'static [&'static str]>,
}

impl BacktestErrorBody {
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

/// All error responses produced by the backtest handlers.
#[derive(Debug)]
pub enum BacktestApiError {
    /// No run has the requested id.
    NotFound,
    /// `from >= to` (after defaulting `to` to `Clock::now()`).
    InvalidRange,
    /// `limit` was zero.
    InvalidLimit,
    /// `limit` exceeded the applicable cap.
    TooManyRows { requested: usize, max: usize },
    /// The `status` filter value is not a valid [`BacktestStatus`].
    InvalidStatus,
    /// Downstream datastore unreachable.
    DbUnavailable(String),
    /// Unexpected internal error (schema drift, decode failure, …).
    Internal(String),
}

impl IntoResponse for BacktestApiError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(BacktestErrorBody::simple(
                    "not_found",
                    "no backtest run with that id",
                )),
            )
                .into_response(),
            Self::InvalidRange => (
                StatusCode::BAD_REQUEST,
                Json(BacktestErrorBody::simple(
                    "invalid_range",
                    "`from` must be strictly before `to`",
                )),
            )
                .into_response(),
            Self::InvalidLimit => (
                StatusCode::BAD_REQUEST,
                Json(BacktestErrorBody::simple(
                    "invalid_limit",
                    "`limit` must be strictly positive",
                )),
            )
                .into_response(),
            Self::TooManyRows { requested, max } => (
                StatusCode::BAD_REQUEST,
                Json(BacktestErrorBody {
                    error: "too_many_rows",
                    message: Some(format!("requested {requested} rows, max {max}")),
                    requested: Some(requested),
                    max: Some(max),
                    allowed: None,
                }),
            )
                .into_response(),
            Self::InvalidStatus => (
                StatusCode::BAD_REQUEST,
                Json(BacktestErrorBody {
                    error: "invalid_status",
                    message: Some("unknown `status` filter value".into()),
                    requested: None,
                    max: None,
                    allowed: Some(BacktestStatus::ALLOWED),
                }),
            )
                .into_response(),
            Self::DbUnavailable(detail) => {
                tracing::warn!(error = %detail, "backtest repository unavailable");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(BacktestErrorBody::simple(
                        "service_unavailable",
                        "backtest datastore is temporarily unreachable",
                    )),
                )
                    .into_response()
            }
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "backtest handler internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(BacktestErrorBody::simple(
                        "internal_error",
                        "internal server error",
                    )),
                )
                    .into_response()
            }
        }
    }
}

impl From<GetBacktestError> for BacktestApiError {
    fn from(err: GetBacktestError) -> Self {
        match err {
            GetBacktestError::Domain(BacktestQueryError::InvalidRange) => Self::InvalidRange,
            GetBacktestError::Domain(BacktestQueryError::InvalidLimit) => Self::InvalidLimit,
            GetBacktestError::Domain(BacktestQueryError::TooManyRows { requested, max }) => {
                Self::TooManyRows { requested, max }
            }
            GetBacktestError::Repository(RepositoryError::Unavailable(d)) => Self::DbUnavailable(d),
            GetBacktestError::Repository(RepositoryError::Internal(d)) => Self::Internal(d),
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Handler for `GET /api/v1/monitoring/backtests`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/backtests",
    tag = "monitoring",
    params(RunListParams),
    responses(
        (status = 200, description = "Backtest runs ordered by descending `started_at`.", body = [BacktestRunSummaryDto]),
        (status = 400, description = "Invalid `status` filter or `limit` out of bounds.", body = BacktestErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = BacktestErrorBody),
        (status = 500, description = "Unexpected internal error.", body = BacktestErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn list_backtests(
    State(state): State<AppState>,
    Query(params): Query<RunListParams>,
) -> Result<Json<Vec<BacktestRunSummaryDto>>, BacktestApiError> {
    let status = match params.status.as_deref() {
        None => None,
        Some(s) => Some(BacktestStatus::try_from(s).map_err(|_| BacktestApiError::InvalidStatus)?),
    };
    let input = ListBacktestRunsInput {
        status,
        strategy_kind: params.strategy_kind,
        exchange: params.exchange,
        symbol: params.symbol,
        limit: params.limit,
    };
    let runs = state.list_backtest_runs.run(input).await?;
    Ok(Json(runs.into_iter().map(Into::into).collect()))
}

/// Handler for `GET /api/v1/monitoring/backtests/{run_id}`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/backtests/{run_id}",
    tag = "monitoring",
    params(("run_id" = Uuid, Path, description = "Backtest run identifier")),
    responses(
        (status = 200, description = "The run and its configuration snapshot.", body = BacktestRunDetailDto),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 404, description = "No run with that id.", body = BacktestErrorBody),
        (status = 503, description = "Datastore temporarily unreachable.", body = BacktestErrorBody),
        (status = 500, description = "Unexpected internal error.", body = BacktestErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_backtest(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<BacktestRunDetailDto>, BacktestApiError> {
    match state.get_backtest_run.run(run_id).await? {
        Some(detail) => Ok(Json(detail.into())),
        None => Err(BacktestApiError::NotFound),
    }
}

/// Handler for `GET /api/v1/monitoring/backtests/{run_id}/orders`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/backtests/{run_id}/orders",
    tag = "monitoring",
    params(
        ("run_id" = Uuid, Path, description = "Backtest run identifier"),
        SeriesParams,
    ),
    responses(
        (status = 200, description = "Run orders on the window, ordered by ascending `market_ts`.", body = [BacktestOrderDto]),
        (status = 400, description = "Invalid range or limit out of bounds.", body = BacktestErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = BacktestErrorBody),
        (status = 500, description = "Unexpected internal error.", body = BacktestErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_backtest_orders(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Query(params): Query<SeriesParams>,
) -> Result<Json<Vec<BacktestOrderDto>>, BacktestApiError> {
    let series = state
        .get_backtest_series
        .orders(series_input(run_id, params))
        .await?;
    Ok(Json(series.into_iter().map(Into::into).collect()))
}

/// Handler for `GET /api/v1/monitoring/backtests/{run_id}/fills`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/backtests/{run_id}/fills",
    tag = "monitoring",
    params(
        ("run_id" = Uuid, Path, description = "Backtest run identifier"),
        SeriesParams,
    ),
    responses(
        (status = 200, description = "Run fills on the window, ordered by ascending `market_ts`.", body = [FillDto]),
        (status = 400, description = "Invalid range or limit out of bounds.", body = BacktestErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = BacktestErrorBody),
        (status = 500, description = "Unexpected internal error.", body = BacktestErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_backtest_fills(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Query(params): Query<SeriesParams>,
) -> Result<Json<Vec<FillDto>>, BacktestApiError> {
    let series = state
        .get_backtest_series
        .fills(series_input(run_id, params))
        .await?;
    Ok(Json(series.into_iter().map(Into::into).collect()))
}

/// Handler for `GET /api/v1/monitoring/backtests/{run_id}/decisions`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/backtests/{run_id}/decisions",
    tag = "monitoring",
    params(
        ("run_id" = Uuid, Path, description = "Backtest run identifier"),
        SeriesParams,
    ),
    responses(
        (status = 200, description = "Run decisions on the window, ordered by ascending `market_ts`.", body = [DecisionDto]),
        (status = 400, description = "Invalid range or limit out of bounds.", body = BacktestErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = BacktestErrorBody),
        (status = 500, description = "Unexpected internal error.", body = BacktestErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_backtest_decisions(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Query(params): Query<SeriesParams>,
) -> Result<Json<Vec<DecisionDto>>, BacktestApiError> {
    let series = state
        .get_backtest_series
        .decisions(series_input(run_id, params))
        .await?;
    Ok(Json(series.into_iter().map(Into::into).collect()))
}

/// Builds a [`GetBacktestSeriesInput`] from a run id and query params.
fn series_input(run_id: Uuid, params: SeriesParams) -> GetBacktestSeriesInput {
    GetBacktestSeriesInput {
        run_id,
        from: params.from,
        to: params.to,
        limit: params.limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use application::ports::{
        BacktestRepository, BacktestRunListQuery, BacktestSeriesQuery, Clock, RepositoryError,
    };
    use application::use_cases::{GetBacktestRun, GetBacktestSeries, ListBacktestRuns};
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use chrono::TimeZone;
    use domain::backtest::{BacktestOrder, BacktestRun, BacktestRunDetail, BacktestStatus};
    use domain::decision::Decision;
    use domain::fill::Fill;
    use domain::order::OrderSide;
    use rust_decimal_macros::dec;
    use tower::ServiceExt;

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 1, h, 0, 0).unwrap()
    }

    fn sample_run() -> BacktestRun {
        BacktestRun {
            id: Uuid::from_u128(1),
            strategy_kind: "ma_cross".into(),
            strategy_id: None,
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            data_range_start: t(0),
            data_range_end: t(1),
            started_at: t(9),
            ended_at: Some(t(10)),
            status: BacktestStatus::Completed,
            slippage_bps: 5,
            latency_ms: 100,
            fee_kraken_bps: 26,
            fee_binance_bps: 10,
            error: None,
        }
    }

    #[derive(Default)]
    struct StubRepo {
        runs: Vec<BacktestRun>,
        detail: Option<BacktestRunDetail>,
        orders: Vec<BacktestOrder>,
    }

    #[async_trait]
    impl BacktestRepository for StubRepo {
        async fn list_runs(
            &self,
            _q: &BacktestRunListQuery,
        ) -> Result<Vec<BacktestRun>, RepositoryError> {
            Ok(self.runs.clone())
        }
        async fn get_run(&self, _id: Uuid) -> Result<Option<BacktestRunDetail>, RepositoryError> {
            Ok(self.detail.clone())
        }
        async fn fetch_orders(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<BacktestOrder>, RepositoryError> {
            Ok(self.orders.clone())
        }
        async fn fetch_fills(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<Fill>, RepositoryError> {
            Ok(vec![])
        }
        async fn fetch_decisions(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<Decision>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct FixedClock;
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            t(23)
        }
    }

    fn app(repo: Arc<StubRepo>) -> Router {
        let list = Arc::new(ListBacktestRuns::new(repo.clone()));
        let detail = Arc::new(GetBacktestRun::new(repo.clone()));
        let series = Arc::new(GetBacktestSeries::new(repo, Arc::new(FixedClock)));
        let state = super::super::state::test_support::state_with_backtests(list, detail, series);
        Router::new()
            .route("/api/v1/monitoring/backtests", get(list_backtests))
            .route("/api/v1/monitoring/backtests/{run_id}", get(get_backtest))
            .route(
                "/api/v1/monitoring/backtests/{run_id}/orders",
                get(get_backtest_orders),
            )
            .with_state(state)
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_returns_200_with_runs() {
        let app = app(Arc::new(StubRepo {
            runs: vec![sample_run()],
            ..Default::default()
        }));
        let resp = app
            .oneshot(
                Request::get("/api/v1/monitoring/backtests")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["status"], "completed");
        assert_eq!(arr[0]["strategy_kind"], "ma_cross");
        assert!(arr[0].get("strategy_id").is_none() || arr[0]["strategy_id"].is_null());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_rejects_invalid_status_with_400() {
        let app = app(Arc::new(StubRepo::default()));
        let resp = app
            .oneshot(
                Request::get("/api/v1/monitoring/backtests?status=paused")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = body_json(resp).await;
        assert_eq!(body["error"], "invalid_status");
        assert!(
            body["allowed"]
                .as_array()
                .unwrap()
                .contains(&"running".into())
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn detail_returns_200_with_snapshot() {
        let app = app(Arc::new(StubRepo {
            detail: Some(BacktestRunDetail {
                run: sample_run(),
                config_snapshot: serde_json::json!({"scenario_name": "c0"}),
            }),
            ..Default::default()
        }));
        let resp = app
            .oneshot(
                Request::get("/api/v1/monitoring/backtests/00000000-0000-0000-0000-000000000001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["run"]["status"], "completed");
        assert_eq!(body["config_snapshot"]["scenario_name"], "c0");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn detail_returns_404_when_absent() {
        let app = app(Arc::new(StubRepo::default()));
        let resp = app
            .oneshot(
                Request::get("/api/v1/monitoring/backtests/00000000-0000-0000-0000-000000000009")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_json(resp).await["error"], "not_found");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn orders_happy_path_and_market_ts_field() {
        let order = BacktestOrder {
            order_id: Uuid::from_u128(0x11),
            decision_id: Uuid::from_u128(0x22),
            side: OrderSide::Buy,
            price: Some(dec!(75000.5)),
            quantity: dec!(0.5),
            status: "filled".into(),
            market_ts: t(12),
        };
        let app = app(Arc::new(StubRepo {
            orders: vec![order],
            ..Default::default()
        }));
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/backtests/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-06-01T00:00:00Z&to=2026-06-01T20:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body[0]["side"], "buy");
        assert_eq!(
            body[0]["market_ts"].as_str().unwrap(),
            "2026-06-01T12:00:00Z"
        );
        assert!((body[0]["price"].as_f64().unwrap() - 75000.5).abs() < 1e-6);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn orders_reject_inverted_range_with_400() {
        let app = app(Arc::new(StubRepo::default()));
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/backtests/00000000-0000-0000-0000-000000000001/orders\
                     ?from=2026-06-01T20:00:00Z&to=2026-06-01T00:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "invalid_range");
    }
}
