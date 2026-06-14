//! `StrategyRepository` port — read-only access to the `strategies` table and
//! the live `fills` table.
//!
//! Implemented by `adapters::outbound::persistence::SqlxStrategyRepository`;
//! tests substitute an in-memory fake. The strategy listing is the UI selector
//! (no window); the fills query is bounded by `[from, to)` on `executed_at`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::fill::LiveFill;
use domain::strategy::Strategy;
use uuid::Uuid;

use super::RepositoryError;

/// Validated parameters of a live-fills query.
///
/// Built by the `GetStrategyFills` use case after validating `from < to` and
/// `0 < limit <= MAX_ORDER_ROWS`. The repository can trust the contents.
#[derive(Debug, Clone)]
pub struct StrategyFillQuery {
    /// Filter on `fills.strategy_id`.
    pub strategy_id: Uuid,
    /// Inclusive lower bound on `executed_at`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `executed_at`.
    pub to: DateTime<Utc>,
    /// Row cap (already validated `<= MAX_ORDER_ROWS`).
    pub limit: usize,
}

/// Port: read-only access to strategies and their live fills.
#[async_trait]
pub trait StrategyRepository: Send + Sync {
    /// Lists every strategy (the UI selector), ordered by `name`.
    async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError>;

    /// Fetches the strategy's live fills in `[from, to)`, ordered by ascending
    /// `executed_at`, capped at `query.limit`.
    async fn fetch_fills_for_strategy(
        &self,
        query: &StrategyFillQuery,
    ) -> Result<Vec<LiveFill>, RepositoryError>;
}
