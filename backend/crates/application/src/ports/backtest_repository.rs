//! `BacktestRepository` port — read-only access to the `backtest_runs` table
//! and its run-scoped mirror tables.
//!
//! A single port covers the whole run-centric model: list/detail of runs, and
//! the three market-clock series (orders / fills / decisions). Implemented by
//! `adapters::outbound::persistence::SqlxBacktestRepository`; tests substitute
//! an in-memory fake.
//!
//! All series are filtered by `run_id` and bounded by a `[from, to)` window on
//! `market_ts`, ordered ascending — the front-end plots markers chronologically
//! so ordering at the DB layer avoids a second sort in JS.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::backtest::{BacktestOrder, BacktestRun, BacktestRunDetail, BacktestStatus};
use domain::decision::Decision;
use domain::fill::Fill;
use uuid::Uuid;

use super::RepositoryError;

/// Validated parameters of a run-listing query.
///
/// Built by the `ListBacktestRuns` use case after validating `limit`. Filters
/// are optional and AND-combined; `None` means "no filter on this field".
#[derive(Debug, Clone, Default)]
pub struct BacktestRunListQuery {
    /// Optional filter on `backtest_runs.status`.
    pub status: Option<BacktestStatus>,
    /// Optional filter on `backtest_runs.strategy_kind`.
    pub strategy_kind: Option<String>,
    /// Optional filter on `backtest_runs.exchange`.
    pub exchange: Option<String>,
    /// Optional filter on `backtest_runs.symbol`.
    pub symbol: Option<String>,
    /// Row cap (already validated `<= MAX_BACKTEST_RUNS`).
    pub limit: usize,
}

/// Validated parameters of a run-scoped series query (orders / fills /
/// decisions).
///
/// Built by the series use cases after validating `from < to` and
/// `0 < limit <= MAX_ORDER_ROWS`. The repository can trust the contents.
#[derive(Debug, Clone)]
pub struct BacktestSeriesQuery {
    /// Filter on the mirror table's `run_id`.
    pub run_id: Uuid,
    /// Inclusive lower bound on `market_ts`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `market_ts`.
    pub to: DateTime<Utc>,
    /// Row cap (already validated `<= MAX_ORDER_ROWS`).
    pub limit: usize,
}

/// Port: read-only access to backtest runs and their run-scoped series.
#[async_trait]
pub trait BacktestRepository: Send + Sync {
    /// Lists runs matching `query`, capped at `query.limit`.
    ///
    /// Ordered by wall-clock `started_at DESC` **by design**: a run listing is
    /// a selector, not a market-clock series, so the natural ordering is "most
    /// recently launched first" — this is the one place `created_at`-style
    /// wall-clock ordering is appropriate, distinct from the `market_ts` axis
    /// used by every series query.
    async fn list_runs(
        &self,
        query: &BacktestRunListQuery,
    ) -> Result<Vec<BacktestRun>, RepositoryError>;

    /// Fetches a single run with its `config_snapshot`, or `None` when no run
    /// has that id.
    async fn get_run(&self, run_id: Uuid) -> Result<Option<BacktestRunDetail>, RepositoryError>;

    /// Fetches the run's orders in `[from, to)`, ordered by ascending
    /// `market_ts`, capped at `query.limit`.
    async fn fetch_orders(
        &self,
        query: &BacktestSeriesQuery,
    ) -> Result<Vec<BacktestOrder>, RepositoryError>;

    /// Fetches the run's fills in `[from, to)`, ordered by ascending
    /// `market_ts`, capped at `query.limit`.
    async fn fetch_fills(&self, query: &BacktestSeriesQuery) -> Result<Vec<Fill>, RepositoryError>;

    /// Fetches the run's decisions in `[from, to)`, ordered by ascending
    /// `market_ts`, capped at `query.limit`.
    async fn fetch_decisions(
        &self,
        query: &BacktestSeriesQuery,
    ) -> Result<Vec<Decision>, RepositoryError>;
}
