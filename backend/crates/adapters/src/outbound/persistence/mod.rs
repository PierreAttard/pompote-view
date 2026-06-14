//! Persistence adapter — sqlx-based implementations of `application` ports.
//!
//! Only a read-only pool against the `pompote_viz_reader` Postgres role is
//! expected here. No migration, no mutation: any `INSERT` / `UPDATE` /
//! `DELETE` would be rejected by the database itself thanks to `GRANT SELECT`
//! only.
//!
//! Layout:
//!
//! - [`health::SqlxHealthChecker`]      — `SELECT 1` probe for `/readyz`
//! - [`candles::SqlxCandleRepository`]  — `time_bucket()` aggregation over
//!   the `candles_5s` hypertable (issue #8)
//! - [`orders::SqlxOrderRepository`]    — bounded `SELECT` on the `orders`
//!   table filtered by `strategy_id` and `created_at` (issue #10)
//! - [`backtests::SqlxBacktestRepository`] — read-only `SELECT`s on
//!   `backtest_runs` and its `*_backtest` mirror tables, on the `market_ts`
//!   axis (Lot 10, #47 / #48)

pub mod backtests;
pub mod candles;
pub mod health;
pub mod orders;
pub mod strategies;

pub use backtests::SqlxBacktestRepository;
pub use candles::SqlxCandleRepository;
pub use health::SqlxHealthChecker;
pub use orders::SqlxOrderRepository;
pub use strategies::SqlxStrategyRepository;
