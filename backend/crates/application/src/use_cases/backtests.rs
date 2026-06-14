//! Backtest use cases — validate inputs then delegate to the
//! [`BacktestRepository`] port.
//!
//! Three use cases cover the run-centric model:
//!
//! - [`ListBacktestRuns`] — the run selector, capped at [`MAX_BACKTEST_RUNS`].
//! - [`GetBacktestRun`] — a single run with its `config_snapshot` (or `None`).
//! - [`GetBacktestSeries`] — the three market-clock series (orders / fills /
//!   decisions), all sharing the same `[from, to)` + `limit` validation and
//!   capped at [`MAX_ORDER_ROWS`] (the same row cap as the live orders
//!   endpoint).
//!
//! Series use cases return the domain `Vec` directly: unlike the live
//! `OrderSeries`/`CandleSeries` newtypes there is no per-series metadata to
//! attach, and the inbound HTTP adapter wraps them in response DTOs anyway.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::backtest::{
    BacktestMetrics, BacktestOrder, BacktestQueryError, BacktestRun, BacktestRunDetail,
    BacktestStatus, MAX_BACKTEST_RUNS,
};
use domain::candle::{Candle, Timeframe};
use domain::decision::Decision;
use domain::fill::Fill;
use domain::order::MAX_ORDER_ROWS;
use uuid::Uuid;

use crate::ports::{
    BacktestRepository, BacktestRunListQuery, BacktestSeriesQuery, Clock, RepositoryError,
};
use crate::use_cases::candles::{GetCandles, GetCandlesError, GetCandlesInput};

/// Top-level error shared by all backtest use cases.
#[derive(Debug, thiserror::Error)]
pub enum GetBacktestError {
    /// A domain invariant was violated (mapped to HTTP `400`).
    #[error(transparent)]
    Domain(#[from] BacktestQueryError),

    /// The repository port reported an I/O error (mapped to `503` / `500`).
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

// ---------------------------------------------------------------------------
// ListBacktestRuns
// ---------------------------------------------------------------------------

/// Input for [`ListBacktestRuns::run`]. Filters are optional and AND-combined.
#[derive(Debug, Clone, Default)]
pub struct ListBacktestRunsInput {
    /// Optional `status` filter.
    pub status: Option<BacktestStatus>,
    /// Optional `strategy_kind` filter.
    pub strategy_kind: Option<String>,
    /// Optional `exchange` filter.
    pub exchange: Option<String>,
    /// Optional `symbol` filter.
    pub symbol: Option<String>,
    /// Row cap. `None` means "use [`MAX_BACKTEST_RUNS`]".
    pub limit: Option<usize>,
}

/// Use case: list backtest runs (the UI selector).
pub struct ListBacktestRuns {
    repo: Arc<dyn BacktestRepository>,
}

impl ListBacktestRuns {
    /// Builds the use case over the given repository port.
    pub fn new(repo: Arc<dyn BacktestRepository>) -> Self {
        Self { repo }
    }

    /// Validates `limit` against [`MAX_BACKTEST_RUNS`] then queries the repo.
    pub async fn run(
        &self,
        input: ListBacktestRunsInput,
    ) -> Result<Vec<BacktestRun>, GetBacktestError> {
        let limit = cap_limit(input.limit, MAX_BACKTEST_RUNS)?;
        let query = BacktestRunListQuery {
            status: input.status,
            strategy_kind: input.strategy_kind,
            exchange: input.exchange,
            symbol: input.symbol,
            limit,
        };
        Ok(self.repo.list_runs(&query).await?)
    }
}

// ---------------------------------------------------------------------------
// GetBacktestRun
// ---------------------------------------------------------------------------

/// Use case: fetch a single run with its `config_snapshot`.
pub struct GetBacktestRun {
    repo: Arc<dyn BacktestRepository>,
}

impl GetBacktestRun {
    /// Builds the use case over the given repository port.
    pub fn new(repo: Arc<dyn BacktestRepository>) -> Self {
        Self { repo }
    }

    /// Returns the run, or `Ok(None)` when no run has that id (the inbound
    /// adapter maps `None` to HTTP `404`).
    pub async fn run(&self, run_id: Uuid) -> Result<Option<BacktestRunDetail>, GetBacktestError> {
        Ok(self.repo.get_run(run_id).await?)
    }
}

// ---------------------------------------------------------------------------
// GetBacktestSeries
// ---------------------------------------------------------------------------

/// Input for the series methods of [`GetBacktestSeries`].
#[derive(Debug, Clone)]
pub struct GetBacktestSeriesInput {
    /// Run identifier.
    pub run_id: Uuid,
    /// Inclusive lower bound on `market_ts`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `market_ts`. `None` means "use `Clock::now()`".
    pub to: Option<DateTime<Utc>>,
    /// Row cap. `None` means "use [`MAX_ORDER_ROWS`]".
    pub limit: Option<usize>,
}

/// Use case: fetch a run's market-clock series (orders / fills / decisions).
///
/// The three methods share the same validation; only the repository call and
/// the element type differ.
pub struct GetBacktestSeries {
    repo: Arc<dyn BacktestRepository>,
    clock: Arc<dyn Clock>,
}

impl GetBacktestSeries {
    /// Builds the use case over the given repository and clock ports.
    pub fn new(repo: Arc<dyn BacktestRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Fetches the run's orders.
    pub async fn orders(
        &self,
        input: GetBacktestSeriesInput,
    ) -> Result<Vec<BacktestOrder>, GetBacktestError> {
        let query = self.build_query(input)?;
        Ok(self.repo.fetch_orders(&query).await?)
    }

    /// Fetches the run's fills.
    pub async fn fills(
        &self,
        input: GetBacktestSeriesInput,
    ) -> Result<Vec<Fill>, GetBacktestError> {
        let query = self.build_query(input)?;
        Ok(self.repo.fetch_fills(&query).await?)
    }

    /// Fetches the run's decisions.
    pub async fn decisions(
        &self,
        input: GetBacktestSeriesInput,
    ) -> Result<Vec<Decision>, GetBacktestError> {
        let query = self.build_query(input)?;
        Ok(self.repo.fetch_decisions(&query).await?)
    }

    /// Validates a series input into a [`BacktestSeriesQuery`].
    ///
    /// - defaults `to` to `Clock::now()` when omitted,
    /// - requires `from < to`,
    /// - defaults `limit` to [`MAX_ORDER_ROWS`], rejects `0` and any value
    ///   above the cap (never truncates silently).
    fn build_query(
        &self,
        input: GetBacktestSeriesInput,
    ) -> Result<BacktestSeriesQuery, BacktestQueryError> {
        let to = input.to.unwrap_or_else(|| self.clock.now());
        if input.from >= to {
            return Err(BacktestQueryError::InvalidRange);
        }
        let limit = cap_limit(input.limit, MAX_ORDER_ROWS)?;
        Ok(BacktestSeriesQuery {
            run_id: input.run_id,
            from: input.from,
            to,
            limit,
        })
    }
}

/// Shared `limit` validation: `None` -> `max`, `0` -> [`BacktestQueryError::InvalidLimit`],
/// `> max` -> [`BacktestQueryError::TooManyRows`].
fn cap_limit(limit: Option<usize>, max: usize) -> Result<usize, BacktestQueryError> {
    match limit {
        None => Ok(max),
        Some(0) => Err(BacktestQueryError::InvalidLimit),
        Some(n) if n > max => Err(BacktestQueryError::TooManyRows { requested: n, max }),
        Some(n) => Ok(n),
    }
}

// ---------------------------------------------------------------------------
// GetBacktestRunCandles
// ---------------------------------------------------------------------------

/// Source of the OHLC background for a backtest run's chart.
///
/// `candles_5s` is the only candle source (no `candles_backtest`, no S3, per
/// the locked decisions). When the run's market window is not covered by
/// `candles_5s` (cold/old windows, synthetic C0 runs), the series is empty and
/// the front-end degrades to a timeline-only view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacktestCandleSource {
    /// Candles came from `candles_5s` (window covered).
    CandlesFiveS,
    /// No candles for the window — degraded, timeline-only view.
    None,
}

impl BacktestCandleSource {
    /// Canonical wire string (`candles_5s` / `none`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CandlesFiveS => "candles_5s",
            Self::None => "none",
        }
    }
}

/// Outcome of [`GetBacktestRunCandles::run`]: the aggregated candles plus a
/// hint telling the front-end whether an OHLC background is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestCandles {
    /// Aggregated OHLC buckets (empty when `source = None`).
    pub candles: Vec<Candle>,
    /// Availability hint.
    pub source: BacktestCandleSource,
}

/// Error of the [`GetBacktestRunCandles`] use case.
#[derive(Debug, thiserror::Error)]
pub enum GetBacktestCandlesError {
    /// No run has the requested id (mapped to HTTP `404`).
    #[error("backtest run not found")]
    RunNotFound,
    /// The run lookup failed (datastore I/O).
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// The candle aggregation failed (range/cap validation or datastore I/O).
    #[error(transparent)]
    Candles(#[from] GetCandlesError),
}

/// Use case: aggregate `candles_5s` over a backtest run's replayed market
/// window, with a source hint.
///
/// Composes the [`BacktestRepository`] (to resolve the run's
/// exchange/symbol/window) with the existing [`GetCandles`] use case (which
/// owns the timeframe whitelist and the [`domain::candle::MAX_CANDLE_POINTS`]
/// cap). The window is the **market clock** `[data_range_start,
/// data_range_end)`; wall-clock `started_at`/`ended_at` are irrelevant to
/// candle selection.
pub struct GetBacktestRunCandles {
    repo: Arc<dyn BacktestRepository>,
    get_candles: Arc<GetCandles>,
}

impl GetBacktestRunCandles {
    /// Builds the use case over the backtest repository and candles use case.
    pub fn new(repo: Arc<dyn BacktestRepository>, get_candles: Arc<GetCandles>) -> Self {
        Self { repo, get_candles }
    }

    /// Resolves the run, aggregates its candles, and computes the source hint.
    pub async fn run(
        &self,
        run_id: Uuid,
        timeframe: Timeframe,
    ) -> Result<BacktestCandles, GetBacktestCandlesError> {
        let detail = self
            .repo
            .get_run(run_id)
            .await?
            .ok_or(GetBacktestCandlesError::RunNotFound)?;
        let run = detail.run;

        let series = self
            .get_candles
            .run(GetCandlesInput {
                exchange: run.exchange,
                symbol: run.symbol,
                timeframe,
                from: run.data_range_start,
                to: Some(run.data_range_end),
            })
            .await?;

        let source = if series.candles.is_empty() {
            BacktestCandleSource::None
        } else {
            BacktestCandleSource::CandlesFiveS
        };
        Ok(BacktestCandles {
            candles: series.candles,
            source,
        })
    }
}

// ---------------------------------------------------------------------------
// GetBacktestRunMetrics
// ---------------------------------------------------------------------------

/// Error of the [`GetBacktestRunMetrics`] use case.
#[derive(Debug, thiserror::Error)]
pub enum GetBacktestMetricsError {
    /// No run has the requested id (mapped to HTTP `404`).
    #[error("backtest run not found")]
    RunNotFound,
    /// The run lookup or aggregation failed (datastore I/O).
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Use case: recompute a run's performance metrics from `fills_backtest`.
///
/// Resolves the run first (so an unknown id is a clean `404` rather than an
/// all-zero body), then folds the per-side fill aggregates into
/// [`BacktestMetrics`]. Read-only; the heavy lifting (the `GROUP BY side`) runs
/// in Postgres under the connection's `statement_timeout`.
pub struct GetBacktestRunMetrics {
    repo: Arc<dyn BacktestRepository>,
}

impl GetBacktestRunMetrics {
    /// Builds the use case over the given repository port.
    pub fn new(repo: Arc<dyn BacktestRepository>) -> Self {
        Self { repo }
    }

    /// Returns the run's metrics, or `RunNotFound` when no run has that id.
    pub async fn run(&self, run_id: Uuid) -> Result<BacktestMetrics, GetBacktestMetricsError> {
        if self.repo.get_run(run_id).await?.is_none() {
            return Err(GetBacktestMetricsError::RunNotFound);
        }
        let aggregates = self.repo.fetch_fill_aggregates(run_id).await?;
        Ok(BacktestMetrics::from_aggregates(&aggregates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{CandleQuery, CandleRepository};
    use async_trait::async_trait;
    use chrono::TimeZone;
    use rust_decimal::Decimal;
    use tokio::sync::Mutex;

    /// Fake repository returning canned responses and capturing the last query.
    use domain::backtest::FillAggregate;

    #[derive(Default)]
    struct FakeRepo {
        runs: Vec<BacktestRun>,
        detail: Option<BacktestRunDetail>,
        orders: Vec<BacktestOrder>,
        fills: Vec<Fill>,
        decisions: Vec<Decision>,
        aggregates: Vec<FillAggregate>,
        last_list: Mutex<Option<BacktestRunListQuery>>,
        last_series: Mutex<Option<BacktestSeriesQuery>>,
    }

    #[async_trait]
    impl BacktestRepository for FakeRepo {
        async fn list_runs(
            &self,
            query: &BacktestRunListQuery,
        ) -> Result<Vec<BacktestRun>, RepositoryError> {
            *self.last_list.lock().await = Some(query.clone());
            Ok(self.runs.clone())
        }
        async fn get_run(
            &self,
            _run_id: Uuid,
        ) -> Result<Option<BacktestRunDetail>, RepositoryError> {
            Ok(self.detail.clone())
        }
        async fn fetch_orders(
            &self,
            query: &BacktestSeriesQuery,
        ) -> Result<Vec<BacktestOrder>, RepositoryError> {
            *self.last_series.lock().await = Some(query.clone());
            Ok(self.orders.clone())
        }
        async fn fetch_fills(
            &self,
            query: &BacktestSeriesQuery,
        ) -> Result<Vec<Fill>, RepositoryError> {
            *self.last_series.lock().await = Some(query.clone());
            Ok(self.fills.clone())
        }
        async fn fetch_decisions(
            &self,
            query: &BacktestSeriesQuery,
        ) -> Result<Vec<Decision>, RepositoryError> {
            *self.last_series.lock().await = Some(query.clone());
            Ok(self.decisions.clone())
        }
        async fn fetch_fill_aggregates(
            &self,
            _run_id: Uuid,
        ) -> Result<Vec<FillAggregate>, RepositoryError> {
            Ok(self.aggregates.clone())
        }
    }

    /// Repository that always reports the datastore as unavailable.
    struct UnavailableRepo;

    #[async_trait]
    impl BacktestRepository for UnavailableRepo {
        async fn list_runs(
            &self,
            _query: &BacktestRunListQuery,
        ) -> Result<Vec<BacktestRun>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn get_run(
            &self,
            _run_id: Uuid,
        ) -> Result<Option<BacktestRunDetail>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn fetch_orders(
            &self,
            _query: &BacktestSeriesQuery,
        ) -> Result<Vec<BacktestOrder>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn fetch_fills(
            &self,
            _query: &BacktestSeriesQuery,
        ) -> Result<Vec<Fill>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn fetch_decisions(
            &self,
            _query: &BacktestSeriesQuery,
        ) -> Result<Vec<Decision>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn fetch_fill_aggregates(
            &self,
            _run_id: Uuid,
        ) -> Result<Vec<FillAggregate>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn t(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
    }

    fn sample_run() -> BacktestRun {
        BacktestRun {
            id: Uuid::nil(),
            strategy_kind: "ma_cross".into(),
            strategy_id: None,
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            data_range_start: t(2026, 5, 1, 0),
            data_range_end: t(2026, 5, 2, 0),
            started_at: t(2026, 6, 1, 9),
            ended_at: Some(t(2026, 6, 1, 9)),
            status: BacktestStatus::Completed,
            slippage_bps: 5,
            latency_ms: 100,
            fee_kraken_bps: 26,
            fee_binance_bps: 10,
            error: None,
        }
    }

    fn series(repo: Arc<FakeRepo>) -> GetBacktestSeries {
        GetBacktestSeries::new(repo, Arc::new(FixedClock(t(2026, 6, 1, 12))))
    }

    fn input(limit: Option<usize>, to: Option<DateTime<Utc>>) -> GetBacktestSeriesInput {
        GetBacktestSeriesInput {
            run_id: Uuid::nil(),
            from: t(2026, 6, 1, 10),
            to,
            limit,
        }
    }

    // --- ListBacktestRuns -------------------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn list_defaults_to_max_backtest_runs() {
        let repo = Arc::new(FakeRepo::default());
        let uc = ListBacktestRuns::new(repo.clone());
        uc.run(ListBacktestRunsInput::default()).await.unwrap();
        let captured = repo.last_list.lock().await.clone().unwrap();
        assert_eq!(captured.limit, MAX_BACKTEST_RUNS);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_rejects_over_cap() {
        let uc = ListBacktestRuns::new(Arc::new(FakeRepo::default()));
        let err = uc
            .run(ListBacktestRunsInput {
                limit: Some(MAX_BACKTEST_RUNS + 1),
                ..Default::default()
            })
            .await
            .unwrap_err();
        match err {
            GetBacktestError::Domain(BacktestQueryError::TooManyRows { requested, max }) => {
                assert_eq!(requested, MAX_BACKTEST_RUNS + 1);
                assert_eq!(max, MAX_BACKTEST_RUNS);
            }
            other => panic!("expected TooManyRows, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_rejects_zero_limit() {
        let uc = ListBacktestRuns::new(Arc::new(FakeRepo::default()));
        let err = uc
            .run(ListBacktestRunsInput {
                limit: Some(0),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetBacktestError::Domain(BacktestQueryError::InvalidLimit)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_forwards_filters() {
        let repo = Arc::new(FakeRepo::default());
        let uc = ListBacktestRuns::new(repo.clone());
        uc.run(ListBacktestRunsInput {
            status: Some(BacktestStatus::Completed),
            symbol: Some("BTCUSDT".into()),
            ..Default::default()
        })
        .await
        .unwrap();
        let captured = repo.last_list.lock().await.clone().unwrap();
        assert_eq!(captured.status, Some(BacktestStatus::Completed));
        assert_eq!(captured.symbol.as_deref(), Some("BTCUSDT"));
        assert_eq!(captured.exchange, None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_propagates_unavailable() {
        let uc = ListBacktestRuns::new(Arc::new(UnavailableRepo));
        let err = uc.run(ListBacktestRunsInput::default()).await.unwrap_err();
        assert!(matches!(
            err,
            GetBacktestError::Repository(RepositoryError::Unavailable(_))
        ));
    }

    // --- GetBacktestRun ---------------------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn get_run_returns_none_when_absent() {
        let uc = GetBacktestRun::new(Arc::new(FakeRepo::default()));
        assert!(uc.run(Uuid::nil()).await.unwrap().is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn get_run_returns_detail_with_snapshot() {
        let snapshot = serde_json::json!({ "scenario_name": "c0", "warmup": 200 });
        let repo = Arc::new(FakeRepo {
            detail: Some(BacktestRunDetail {
                run: sample_run(),
                config_snapshot: snapshot.clone(),
            }),
            ..Default::default()
        });
        let uc = GetBacktestRun::new(repo);
        let got = uc.run(Uuid::nil()).await.unwrap().expect("run present");
        assert_eq!(got.run.status, BacktestStatus::Completed);
        assert_eq!(got.config_snapshot, snapshot);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn get_run_propagates_unavailable() {
        let uc = GetBacktestRun::new(Arc::new(UnavailableRepo));
        let err = uc.run(Uuid::nil()).await.unwrap_err();
        assert!(matches!(
            err,
            GetBacktestError::Repository(RepositoryError::Unavailable(_))
        ));
    }

    // --- GetBacktestSeries (shared validation via `orders`) ---------------

    #[tokio::test(flavor = "current_thread")]
    async fn series_defaults_to_clock_now_and_max_rows() {
        let repo = Arc::new(FakeRepo::default());
        let uc = series(repo.clone());
        uc.orders(input(None, None)).await.unwrap();
        let captured = repo.last_series.lock().await.clone().unwrap();
        assert_eq!(captured.to, t(2026, 6, 1, 12)); // clock now
        assert_eq!(captured.limit, MAX_ORDER_ROWS);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn series_rejects_inverted_range() {
        let uc = series(Arc::new(FakeRepo::default()));
        let err = uc
            .orders(input(None, Some(t(2026, 6, 1, 8))))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetBacktestError::Domain(BacktestQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn series_rejects_over_cap() {
        let uc = series(Arc::new(FakeRepo::default()));
        let err = uc
            .orders(input(Some(MAX_ORDER_ROWS + 1), Some(t(2026, 6, 1, 11))))
            .await
            .unwrap_err();
        match err {
            GetBacktestError::Domain(BacktestQueryError::TooManyRows { requested, max }) => {
                assert_eq!(requested, MAX_ORDER_ROWS + 1);
                assert_eq!(max, MAX_ORDER_ROWS);
            }
            other => panic!("expected TooManyRows, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn series_fills_and_decisions_delegate() {
        // Smoke-test the other two methods share the validated query path.
        let repo = Arc::new(FakeRepo::default());
        let uc = series(repo.clone());
        uc.fills(input(Some(10), Some(t(2026, 6, 1, 11))))
            .await
            .unwrap();
        assert_eq!(repo.last_series.lock().await.clone().unwrap().limit, 10);
        uc.decisions(input(Some(20), Some(t(2026, 6, 1, 11))))
            .await
            .unwrap();
        assert_eq!(repo.last_series.lock().await.clone().unwrap().limit, 20);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn series_propagates_unavailable() {
        let uc = GetBacktestSeries::new(
            Arc::new(UnavailableRepo),
            Arc::new(FixedClock(t(2026, 6, 1, 12))),
        );
        let err = uc
            .orders(input(None, Some(t(2026, 6, 1, 11))))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetBacktestError::Repository(RepositoryError::Unavailable(_))
        ));
    }

    // --- GetBacktestRunCandles --------------------------------------------

    struct CandleStub {
        candles: Vec<Candle>,
    }

    #[async_trait]
    impl CandleRepository for CandleStub {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Ok(self.candles.clone())
        }
    }

    fn candle() -> Candle {
        Candle {
            open_time: t(2026, 5, 1, 1),
            open: Decimal::ZERO,
            high: Decimal::ZERO,
            low: Decimal::ZERO,
            close: Decimal::ZERO,
            volume: Decimal::ZERO,
        }
    }

    fn candles_uc(
        detail: Option<BacktestRunDetail>,
        candles: Vec<Candle>,
    ) -> GetBacktestRunCandles {
        let repo = Arc::new(FakeRepo {
            detail,
            ..Default::default()
        });
        let get_candles = Arc::new(GetCandles::new(
            Arc::new(CandleStub { candles }),
            Arc::new(FixedClock(t(2026, 6, 1, 12))),
        ));
        GetBacktestRunCandles::new(repo, get_candles)
    }

    fn detail() -> BacktestRunDetail {
        BacktestRunDetail {
            run: sample_run(),
            config_snapshot: serde_json::json!({}),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn candles_source_is_candles_5s_when_present() {
        let uc = candles_uc(Some(detail()), vec![candle()]);
        let out = uc.run(Uuid::nil(), Timeframe::H1).await.unwrap();
        assert_eq!(out.source, BacktestCandleSource::CandlesFiveS);
        assert_eq!(out.candles.len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn candles_source_is_none_when_window_uncovered() {
        let uc = candles_uc(Some(detail()), vec![]);
        let out = uc.run(Uuid::nil(), Timeframe::H1).await.unwrap();
        assert_eq!(out.source, BacktestCandleSource::None);
        assert!(out.candles.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn candles_run_not_found() {
        let uc = candles_uc(None, vec![]);
        let err = uc.run(Uuid::nil(), Timeframe::H1).await.unwrap_err();
        assert!(matches!(err, GetBacktestCandlesError::RunNotFound));
    }

    // --- GetBacktestRunMetrics --------------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn metrics_run_not_found_when_absent() {
        let uc = GetBacktestRunMetrics::new(Arc::new(FakeRepo::default()));
        let err = uc.run(Uuid::nil()).await.unwrap_err();
        assert!(matches!(err, GetBacktestMetricsError::RunNotFound));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn metrics_folds_aggregates_for_existing_run() {
        let repo = Arc::new(FakeRepo {
            detail: Some(detail()),
            aggregates: vec![
                FillAggregate {
                    side: domain::order::OrderSide::Buy,
                    fills: 1,
                    quantity: Decimal::from(1),
                    notional: Decimal::from(100),
                    fees: Decimal::new(1, 1),
                },
                FillAggregate {
                    side: domain::order::OrderSide::Sell,
                    fills: 1,
                    quantity: Decimal::from(1),
                    notional: Decimal::from(150),
                    fees: Decimal::new(1, 1),
                },
            ],
            ..Default::default()
        });
        let metrics = GetBacktestRunMetrics::new(repo)
            .run(Uuid::nil())
            .await
            .unwrap();
        assert_eq!(metrics.gross_pnl, Decimal::from(50));
        assert_eq!(metrics.net_pnl, Decimal::new(498, 1)); // 50 - 0.2
    }

    #[tokio::test(flavor = "current_thread")]
    async fn metrics_propagates_unavailable() {
        let uc = GetBacktestRunMetrics::new(Arc::new(UnavailableRepo));
        let err = uc.run(Uuid::nil()).await.unwrap_err();
        assert!(matches!(
            err,
            GetBacktestMetricsError::Repository(RepositoryError::Unavailable(_))
        ));
    }
}
