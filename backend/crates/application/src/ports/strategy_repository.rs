//! `StrategyRepository` port — read-only access to the `strategies` table and
//! the live `fills` / `strategy_decisions` tables.
//!
//! Implemented by `adapters::outbound::persistence::SqlxStrategyRepository`;
//! tests substitute an in-memory fake. The strategy listing is the UI selector
//! (no window); the fills/decisions series are bounded by `[from, to)` on the
//! wall clock (`executed_at` / `created_at`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::decision::LiveDecision;
use domain::fill::LiveFill;
use domain::strategy::Strategy;
use uuid::Uuid;

use super::RepositoryError;

/// Validated parameters of a windowed strategy series query (fills/decisions).
///
/// Built by the `GetStrategyFills` / `GetStrategyDecisions` use cases after
/// validating `from < to` and `0 < limit <= MAX_ORDER_ROWS`. The repository can
/// trust the contents. The `[from, to)` bound applies to the wall-clock column
/// of the queried table (`executed_at` for fills, `created_at` for decisions).
#[derive(Debug, Clone)]
pub struct StrategyWindowQuery {
    /// Filter on the table's `strategy_id`.
    pub strategy_id: Uuid,
    /// Inclusive lower bound on the wall-clock column.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on the wall-clock column.
    pub to: DateTime<Utc>,
    /// Row cap (already validated `<= MAX_ORDER_ROWS`).
    pub limit: usize,
}

/// Port: read-only access to strategies and their live fills/decisions.
#[async_trait]
pub trait StrategyRepository: Send + Sync {
    /// Lists every strategy (the UI selector), ordered by `name`.
    async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError>;

    /// Fetches the strategy's live fills in `[from, to)`, ordered by ascending
    /// `executed_at`, capped at `query.limit`.
    async fn fetch_fills_for_strategy(
        &self,
        query: &StrategyWindowQuery,
    ) -> Result<Vec<LiveFill>, RepositoryError>;

    /// Fetches the strategy's live decisions in `[from, to)` (with their
    /// optional market-context snapshot), ordered by ascending `created_at`,
    /// capped at `query.limit`.
    async fn fetch_decisions_for_strategy(
        &self,
        query: &StrategyWindowQuery,
    ) -> Result<Vec<LiveDecision>, RepositoryError>;
}
