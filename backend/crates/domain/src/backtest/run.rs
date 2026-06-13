//! `BacktestRun` entity — one row from the `backtest_runs` table.
//!
//! The viz model is **run-centric**: every backtest artefact is keyed by
//! `run_id` and the UI selector is a list of runs (not a strategy/symbol
//! pair). `strategy_id` is therefore `Option<Uuid>`: `backtest_runs.strategy_id`
//! is nullable / synthetic and has no foreign key to `strategies`.
//!
//! Two views are exposed:
//!
//! - [`BacktestRun`] — the lightweight summary returned by the list endpoint
//!   (`GET /backtests`). It deliberately omits the potentially large
//!   `config_snapshot` JSONB so a 500-run listing stays cheap.
//! - [`BacktestRunDetail`] — the summary plus `config_snapshot`, returned by
//!   the detail endpoint (`GET /backtests/{run_id}`).
//!
//! Timestamps follow the locked decision: the **market clock** axis is
//! `data_range_start` / `data_range_end` (the replayed window), while
//! `started_at` / `ended_at` are wall-clock bookkeeping of the run itself.
//!
//! No `serde::Serialize` is derived here on purpose — this is a pure domain
//! entity; serialisation lives in the inbound HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::BacktestStatus;

/// Summary of a backtest run (list view).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestRun {
    /// Primary key (`backtest_runs.id`).
    pub id: Uuid,
    /// Strategy family label (`backtest_runs.strategy_kind`).
    pub strategy_kind: String,
    /// Optional synthetic strategy id (nullable, no FK to `strategies`).
    pub strategy_id: Option<Uuid>,
    /// Exchange identifier (e.g. `binance`).
    pub exchange: String,
    /// Symbol identifier (e.g. `BTCUSDT`).
    pub symbol: String,
    /// Inclusive start of the replayed market window (market clock).
    pub data_range_start: DateTime<Utc>,
    /// Exclusive end of the replayed market window (market clock).
    pub data_range_end: DateTime<Utc>,
    /// Wall-clock time the run started (`backtest_runs.started_at`).
    pub started_at: DateTime<Utc>,
    /// Wall-clock time the run ended; `None` while `status = running`.
    pub ended_at: Option<DateTime<Utc>>,
    /// Lifecycle state (one of the four values in [`BacktestStatus`]).
    pub status: BacktestStatus,
    /// Simulated slippage in basis points.
    pub slippage_bps: i32,
    /// Simulated execution latency in milliseconds.
    pub latency_ms: i32,
    /// Simulated Kraken taker fee in basis points.
    pub fee_kraken_bps: i32,
    /// Simulated Binance taker fee in basis points.
    pub fee_binance_bps: i32,
    /// Failure description when `status = failed`; `None` otherwise.
    pub error: Option<String>,
}

/// A backtest run plus its full configuration snapshot (detail view).
///
/// `config_snapshot` is an opaque, display-only `jsonb` object (the DB enforces
/// `jsonb_typeof = 'object'`). It is forwarded to the front-end unchanged; the
/// viz backend never interprets its contents (e.g. a `scenario_name` for C0
/// synthetic runs lives inside this object, not as a dedicated column).
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestRunDetail {
    /// The run summary.
    pub run: BacktestRun,
    /// Opaque configuration snapshot (`backtest_runs.config_snapshot`).
    pub config_snapshot: Value,
}
