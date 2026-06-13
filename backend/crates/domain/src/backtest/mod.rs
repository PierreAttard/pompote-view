//! Backtest domain — entities, value objects and invariants for the
//! `backtest_runs` table and its run-scoped mirror tables.
//!
//! The viz model is **run-centric**: every artefact is keyed by `run_id` and
//! ordered on the **market clock** (`market_ts`), never on `created_at`.
//!
//! Layout:
//!
//! - [`run::BacktestRun`] / [`run::BacktestRunDetail`] — one `backtest_runs`
//!   row (summary, and summary + `config_snapshot`)
//! - [`status::BacktestStatus`] — the four lifecycle states enforced by the
//!   `CHECK` on `backtest_runs.status`
//! - [`order::BacktestOrder`] — one `orders_backtest` row (market-clock axis)
//! - [`MAX_BACKTEST_RUNS`] — hard cap on a single run-listing response
//! - [`BacktestQueryError`] — domain-level validation errors for the backtest
//!   use cases (mapped to HTTP `400` at the inbound boundary)
//!
//! The decision and fill read models live in the shared [`crate::decision`]
//! and [`crate::fill`] modules: they are not backtest-specific concepts and
//! will be reused by the live monitoring endpoints (#9 / #11).

pub mod order;
pub mod run;
pub mod status;

use thiserror::Error;

pub use order::BacktestOrder;
pub use run::{BacktestRun, BacktestRunDetail};
pub use status::{BacktestStatus, InvalidBacktestStatus};

/// Hard upper bound on the number of runs returned by a single
/// `GET /api/v1/monitoring/backtests` request.
///
/// A run listing is a selector, not a data series: a few hundred entries is
/// already more than a human scrolls through. Enforced by the
/// `ListBacktestRuns` use case (rejects `limit > MAX_BACKTEST_RUNS` with
/// [`BacktestQueryError::TooManyRows`]) and mirrored as a SQL `LIMIT` safety
/// net by the persistence adapter.
pub const MAX_BACKTEST_RUNS: usize = 500;

/// Validation errors raised by the backtest use cases.
///
/// All variants map to HTTP `400 Bad Request` at the inbound adapter, each
/// with a distinct machine-readable discriminator in the JSON body.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BacktestQueryError {
    /// `from >= to` on a series query (orders / fills / decisions).
    #[error("invalid range: `from` must be strictly before `to`")]
    InvalidRange,

    /// `limit` was zero.
    #[error("invalid limit: must be strictly positive")]
    InvalidLimit,

    /// `limit` exceeded the applicable cap (run listing or series).
    ///
    /// The use case **never** truncates silently: it rejects the request and
    /// asks the caller to lower the limit or narrow the window.
    #[error("too many rows: requested {requested}, max {max}")]
    TooManyRows {
        /// The number of rows the caller asked for.
        requested: usize,
        /// The applicable hard cap.
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_many_rows_carries_requested_and_max() {
        let err = BacktestQueryError::TooManyRows {
            requested: 10_000,
            max: MAX_BACKTEST_RUNS,
        };
        let msg = err.to_string();
        assert!(msg.contains("10000"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn invalid_limit_message_is_self_explanatory() {
        assert!(
            BacktestQueryError::InvalidLimit
                .to_string()
                .contains("strictly positive")
        );
    }
}
