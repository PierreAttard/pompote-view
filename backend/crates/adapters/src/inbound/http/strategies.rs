//! Strategy/utility monitoring HTTP handlers (Lot 1, issue #11).
//!
//! Three read-only endpoints under `/api/v1/monitoring` (behind `X-API-Key`):
//!
//! - `GET /timeframes` — the static whitelist of aggregation timeframes the
//!   `/candles` endpoint accepts ([`Timeframe::ALLOWED`], the single source of
//!   truth so the UI never offers a timeframe the backend would reject).
//! - `GET /strategies` — the run selector: every `strategies` row with its
//!   `enabled_paper` / `enabled_live` flags (paper-vs-live badge/filter).
//! - `GET /strategies/{id}/fills` — a strategy's live fills on a window
//!   (`executed_at` axis), used for execution markers.
//!
//! `Decimal -> f64` happens at this boundary only (same rationale as the
//! candles/orders handlers).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use application::ports::RepositoryError;
use application::use_cases::{GetStrategyFillsInput, StrategyError};
use domain::candle::Timeframe;
use domain::order::{MAX_ORDER_ROWS, OrderQueryError};

use super::state::AppState;

/// `Decimal -> f64` down-cast used by the fill DTO (see candles handler).
fn cast(d: rust_decimal::Decimal) -> f64 {
    <rust_decimal::Decimal as TryInto<f64>>::try_into(d).unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// A strategy row (selector item).
#[derive(Debug, Serialize, ToSchema)]
pub struct StrategyDto {
    /// Primary key.
    pub id: Uuid,
    /// Unique human-readable name.
    pub name: String,
    /// Strategy kind/family.
    #[schema(example = "directional")]
    pub kind: String,
    /// Whether the strategy is enabled for paper trading.
    pub enabled_paper: bool,
    /// Whether the strategy is enabled for live trading.
    pub enabled_live: bool,
}

impl From<domain::strategy::Strategy> for StrategyDto {
    fn from(s: domain::strategy::Strategy) -> Self {
        Self {
            id: s.id,
            name: s.name,
            kind: s.kind,
            enabled_paper: s.enabled_paper,
            enabled_live: s.enabled_live,
        }
    }
}

/// A live fill (execution marker).
#[derive(Debug, Serialize, ToSchema)]
pub struct LiveFillDto {
    /// Primary key.
    pub fill_id: Uuid,
    /// Order that produced this fill.
    pub order_id: Uuid,
    /// `buy` or `sell`.
    #[schema(example = "buy")]
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
    /// Wall-clock execution time (RFC3339).
    pub executed_at: DateTime<Utc>,
    /// Whether this fill is paper trading.
    pub is_paper: bool,
}

impl From<domain::fill::LiveFill> for LiveFillDto {
    fn from(f: domain::fill::LiveFill) -> Self {
        Self {
            fill_id: f.fill_id,
            order_id: f.order_id,
            side: f.side.as_str().to_string(),
            price: cast(f.price),
            quantity: cast(f.quantity),
            fee: cast(f.fee),
            fee_asset: f.fee_asset,
            executed_at: f.executed_at,
            is_paper: f.is_paper,
        }
    }
}

/// JSON error body for the strategy endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct StrategyErrorBody {
    /// Machine-readable error discriminator.
    pub error: &'static str,
    /// Human-readable explanation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Echoes the requested row count for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<usize>,
    /// Maximum row count for `too_many_rows`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
}

impl StrategyErrorBody {
    fn simple(error: &'static str, message: impl Into<String>) -> Self {
        Self {
            error,
            message: Some(message.into()),
            requested: None,
            max: None,
        }
    }
}

/// All error responses produced by the strategy handlers.
#[derive(Debug)]
pub enum StrategyApiError {
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

impl IntoResponse for StrategyApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRange => (
                StatusCode::BAD_REQUEST,
                Json(StrategyErrorBody::simple(
                    "invalid_range",
                    "`from` must be strictly before `to`",
                )),
            )
                .into_response(),
            Self::InvalidLimit => (
                StatusCode::BAD_REQUEST,
                Json(StrategyErrorBody::simple(
                    "invalid_limit",
                    "`limit` must be strictly positive",
                )),
            )
                .into_response(),
            Self::TooManyRows { requested } => (
                StatusCode::BAD_REQUEST,
                Json(StrategyErrorBody {
                    error: "too_many_rows",
                    message: Some(format!("requested {requested} rows, max {MAX_ORDER_ROWS}")),
                    requested: Some(requested),
                    max: Some(MAX_ORDER_ROWS),
                }),
            )
                .into_response(),
            Self::DbUnavailable(detail) => {
                tracing::warn!(error = %detail, "strategy repository unavailable");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(StrategyErrorBody::simple(
                        "service_unavailable",
                        "strategy datastore is temporarily unreachable",
                    )),
                )
                    .into_response()
            }
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "strategy handler internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(StrategyErrorBody::simple(
                        "internal_error",
                        "internal server error",
                    )),
                )
                    .into_response()
            }
        }
    }
}

impl From<StrategyError> for StrategyApiError {
    fn from(err: StrategyError) -> Self {
        match err {
            StrategyError::Domain(OrderQueryError::InvalidRange) => Self::InvalidRange,
            StrategyError::Domain(OrderQueryError::InvalidLimit) => Self::InvalidLimit,
            StrategyError::Domain(OrderQueryError::TooManyRows { requested, .. }) => {
                Self::TooManyRows { requested }
            }
            StrategyError::Repository(RepositoryError::Unavailable(d)) => Self::DbUnavailable(d),
            StrategyError::Repository(RepositoryError::Internal(d)) => Self::Internal(d),
        }
    }
}

/// Raw query parameters for the fills series.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct FillQueryParams {
    /// Inclusive lower bound on `executed_at` (RFC3339).
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `executed_at` (RFC3339). Defaults to `Clock::now()`.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// Row cap (defaults to [`MAX_ORDER_ROWS`], rejected above it).
    #[serde(default)]
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Handler for `GET /api/v1/monitoring/timeframes`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/timeframes",
    tag = "monitoring",
    responses(
        (status = 200, description = "Allowed aggregation timeframes (finest to coarsest).", body = [String]),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_timeframes() -> Json<Vec<String>> {
    Json(Timeframe::ALLOWED.iter().map(|s| s.to_string()).collect())
}

/// Handler for `GET /api/v1/monitoring/strategies`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/strategies",
    tag = "monitoring",
    responses(
        (status = 200, description = "Every strategy, ordered by name.", body = [StrategyDto]),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = StrategyErrorBody),
        (status = 500, description = "Unexpected internal error.", body = StrategyErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn list_strategies(
    State(state): State<AppState>,
) -> Result<Json<Vec<StrategyDto>>, StrategyApiError> {
    let strategies = state.list_strategies.run().await?;
    Ok(Json(
        strategies.into_iter().map(StrategyDto::from).collect(),
    ))
}

/// Handler for `GET /api/v1/monitoring/strategies/{id}/fills`.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/strategies/{id}/fills",
    tag = "monitoring",
    params(
        ("id" = Uuid, Path, description = "Strategy identifier"),
        FillQueryParams,
    ),
    responses(
        (status = 200, description = "Live fills for the strategy on the window, ordered by ascending `executed_at`.", body = [LiveFillDto]),
        (status = 400, description = "Invalid range or limit out of bounds.", body = StrategyErrorBody),
        (status = 401, description = "Missing or invalid `X-API-Key` header."),
        (status = 503, description = "Datastore temporarily unreachable.", body = StrategyErrorBody),
        (status = 500, description = "Unexpected internal error.", body = StrategyErrorBody),
    ),
    security(("x_api_key" = [])),
)]
pub async fn get_strategy_fills(
    State(state): State<AppState>,
    Path(strategy_id): Path<Uuid>,
    Query(params): Query<FillQueryParams>,
) -> Result<Json<Vec<LiveFillDto>>, StrategyApiError> {
    let fills = state
        .get_strategy_fills
        .run(GetStrategyFillsInput {
            strategy_id,
            from: params.from,
            to: params.to,
            limit: params.limit,
        })
        .await?;
    Ok(Json(fills.into_iter().map(LiveFillDto::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use application::ports::{RepositoryError, StrategyFillQuery, StrategyRepository};
    use application::use_cases::{GetStrategyFills, ListStrategies};
    use async_trait::async_trait;
    use axum::{Router, body::Body, http::Request, routing::get};
    use chrono::TimeZone;
    use domain::fill::LiveFill;
    use domain::order::OrderSide;
    use domain::strategy::Strategy;
    use rust_decimal_macros::dec;
    use tower::ServiceExt;

    #[derive(Default)]
    struct StubRepo {
        strategies: Vec<Strategy>,
        fills: Vec<LiveFill>,
    }

    #[async_trait]
    impl StrategyRepository for StubRepo {
        async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError> {
            Ok(self.strategies.clone())
        }
        async fn fetch_fills_for_strategy(
            &self,
            _q: &StrategyFillQuery,
        ) -> Result<Vec<LiveFill>, RepositoryError> {
            Ok(self.fills.clone())
        }
    }

    struct FixedClock;
    impl application::ports::Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap()
        }
    }

    fn app(repo: Arc<StubRepo>) -> Router {
        let list = Arc::new(ListStrategies::new(repo.clone()));
        let fills = Arc::new(GetStrategyFills::new(repo, Arc::new(FixedClock)));
        let state = super::super::state::test_support::state_with_strategies(list, fills);
        Router::new()
            .route("/api/v1/monitoring/timeframes", get(get_timeframes))
            .route("/api/v1/monitoring/strategies", get(list_strategies))
            .route(
                "/api/v1/monitoring/strategies/{id}/fills",
                get(get_strategy_fills),
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
    async fn timeframes_lists_allowed_values() {
        let resp = app(Arc::new(StubRepo::default()))
            .oneshot(
                Request::get("/api/v1/monitoring/timeframes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let arr = body.as_array().unwrap();
        assert!(arr.contains(&serde_json::json!("1h")));
        assert!(arr.contains(&serde_json::json!("5s")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn strategies_returns_rows_with_mode_flags() {
        let app = app(Arc::new(StubRepo {
            strategies: vec![Strategy {
                id: Uuid::nil(),
                name: "ma_cross".into(),
                kind: "directional".into(),
                enabled_paper: true,
                enabled_live: false,
            }],
            ..Default::default()
        }));
        let resp = app
            .oneshot(
                Request::get("/api/v1/monitoring/strategies")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body[0]["name"], "ma_cross");
        assert_eq!(body[0]["enabled_paper"], true);
        assert_eq!(body[0]["enabled_live"], false);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_returns_rows_with_is_paper() {
        let app = app(Arc::new(StubRepo {
            fills: vec![LiveFill {
                fill_id: Uuid::nil(),
                order_id: Uuid::nil(),
                side: OrderSide::Sell,
                price: dec!(100),
                quantity: dec!(0.5),
                fee: dec!(0.1),
                fee_asset: "USDT".into(),
                executed_at: Utc.with_ymd_and_hms(2026, 6, 1, 10, 0, 0).unwrap(),
                is_paper: true,
            }],
            ..Default::default()
        }));
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000000/fills\
                     ?from=2026-06-01T00:00:00Z&to=2026-06-01T11:00:00Z",
                )
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body[0]["side"], "sell");
        assert_eq!(body[0]["is_paper"], true);
        assert_eq!(body[0]["price"], 100.0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_reject_inverted_range_with_400() {
        let app = app(Arc::new(StubRepo::default()));
        let resp = app
            .oneshot(
                Request::get(
                    "/api/v1/monitoring/strategies/00000000-0000-0000-0000-000000000000/fills\
                     ?from=2026-06-01T11:00:00Z&to=2026-06-01T01:00:00Z",
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
