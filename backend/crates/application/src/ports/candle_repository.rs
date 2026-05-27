//! `CandleRepository` port — read-only access to bucketed OHLCV data.
//!
//! Implemented by `adapters::outbound::persistence::SqlxCandleRepository`,
//! which issues a `time_bucket()` aggregation against the `candles_5s`
//! hypertable. Tests substitute an in-memory fake.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::candle::{Candle, Timeframe};
use thiserror::Error;

/// Validated parameters of a candles query.
///
/// Constructed by the `GetCandles` use case after all domain-level validation
/// (timeframe whitelist, `from < to`, bucket count under cap). The repository
/// can therefore trust the contents and issue SQL directly.
#[derive(Debug, Clone)]
pub struct CandleQuery {
    /// Exchange identifier as stored in `candles_5s.exchange` (e.g. `binance`).
    pub exchange: String,
    /// Symbol identifier as stored in `candles_5s.symbol` (e.g. `BTCUSDT`).
    pub symbol: String,
    /// Aggregation width.
    pub timeframe: Timeframe,
    /// Inclusive lower bound on `open_time`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `open_time`.
    pub to: DateTime<Utc>,
}

/// Repository-level errors surfaced by adapters.
///
/// Kept opaque on purpose: the use case decides how to map these to domain
/// errors (today: pass-through to the HTTP layer, which renders `503` for
/// `Unavailable`).
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The underlying datastore is unreachable or returned a transport error.
    #[error("repository unavailable: {0}")]
    Unavailable(String),

    /// The datastore answered but the query failed for an unexpected reason
    /// (e.g. schema drift). Always logged at `error` level by the adapter.
    #[error("repository internal error: {0}")]
    Internal(String),
}

/// Port: read-only access to aggregated OHLCV buckets.
#[async_trait]
pub trait CandleRepository: Send + Sync {
    /// Fetches the OHLCV buckets matching `query`, ordered by ascending
    /// `open_time`.
    ///
    /// Implementations MUST honour the [`super::super::use_cases::candles::GetCandles`]
    /// invariants and never return more than
    /// `domain::candle::MAX_CANDLE_POINTS + 1` rows (safety net `LIMIT`).
    async fn fetch_aggregated(&self, query: &CandleQuery) -> Result<Vec<Candle>, RepositoryError>;
}
