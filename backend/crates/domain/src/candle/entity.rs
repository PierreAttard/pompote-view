//! `Candle` entity — bucketed OHLCV aggregate over the `candles_5s` hypertable.
//!
//! All numeric fields use [`rust_decimal::Decimal`] so we never lose precision
//! at the I/O boundary. The original `NUMERIC(20,8)` column in Postgres is
//! preserved end-to-end through the persistence adapter; only the HTTP DTO
//! down-casts to `f64` (with an explicit lossy conversion, documented at the
//! adapter site) for Lightweight Charts consumption.
//!
//! No `serde::Serialize` is derived here on purpose: this is a pure domain
//! entity. Serialisation lives in the inbound HTTP adapter as a dedicated DTO.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// One aggregated bucket of OHLCV data returned by `time_bucket()`.
///
/// `open_time` is the **start** of the bucket (e.g. for a `1h` timeframe,
/// `2026-05-27T14:00:00Z` covers the window `[14:00, 15:00)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candle {
    /// Bucket start (inclusive lower bound of the window).
    pub open_time: DateTime<Utc>,
    /// First trade price in the bucket.
    pub open: Decimal,
    /// Highest trade price in the bucket.
    pub high: Decimal,
    /// Lowest trade price in the bucket.
    pub low: Decimal,
    /// Last trade price in the bucket.
    pub close: Decimal,
    /// Total traded volume (base asset) within the bucket.
    pub volume: Decimal,
}
