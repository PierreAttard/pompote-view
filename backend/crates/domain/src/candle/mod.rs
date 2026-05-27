//! Candle domain — entities, value objects and invariants for OHLCV data.
//!
//! Layout:
//!
//! - [`entity::Candle`]            — one bucket of OHLCV data
//! - [`timeframe::Timeframe`]      — whitelist of supported bucket widths
//! - [`MAX_CANDLE_POINTS`]         — hard cap on the number of buckets we ever
//!   return on a single HTTP response (mirrored as a SQL `LIMIT` safety net)
//! - [`CandleQueryError`]          — domain-level validation errors for the
//!   `GetCandles` use case (mapped to HTTP `400` at the inbound boundary)

pub mod entity;
pub mod timeframe;

use thiserror::Error;

pub use entity::Candle;
pub use timeframe::{InvalidTimeframe, Timeframe};

/// Hard upper bound on the number of OHLCV points returned by a single
/// `/api/v1/monitoring/candles` request.
///
/// Enforced at three layers:
///
/// 1. The application use case validates `(to - from) / timeframe` **before**
///    issuing any SQL, rejecting the request with [`CandleQueryError::TooManyPoints`].
/// 2. The persistence adapter still wires `LIMIT (MAX_CANDLE_POINTS + 1)` on
///    the query as a defence-in-depth safety net.
/// 3. The HTTP layer maps the error to `400 too_many_points`, telling the
///    caller both the cap and what they requested.
///
/// `+1` in the SQL `LIMIT` lets us *detect* (in future work) that the cap was
/// actually saturating; right now the use-case validation catches it first.
pub const MAX_CANDLE_POINTS: usize = 5_000;

/// Validation errors raised by the `GetCandles` use case.
///
/// All variants are mapped to HTTP `400 Bad Request` by the inbound adapter,
/// each with a distinct machine-readable `error` discriminator in the JSON
/// body so the front-end can localise the message.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CandleQueryError {
    /// `timeframe` query string did not match any [`Timeframe`] variant.
    #[error(transparent)]
    InvalidTimeframe(#[from] InvalidTimeframe),

    /// `from >= to`, or one of the bounds is not a valid RFC3339 timestamp.
    #[error("invalid range: `from` must be strictly before `to`")]
    InvalidRange,

    /// `(to - from) / timeframe` would exceed [`MAX_CANDLE_POINTS`].
    ///
    /// The use case **never** truncates silently: it always rejects the
    /// request and asks the caller to narrow the window or coarsen the
    /// timeframe. This keeps the contract explicit (no silent data loss).
    #[error("too many points: requested {requested}, max {max}")]
    TooManyPoints {
        /// The number of buckets that would be returned for the requested
        /// `(from, to, timeframe)` triple.
        requested: usize,
        /// The hard cap, equal to [`MAX_CANDLE_POINTS`].
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_timeframe_converts_into_query_error() {
        let raw = Timeframe::try_from("7s").unwrap_err();
        let err: CandleQueryError = raw.into();
        assert!(matches!(err, CandleQueryError::InvalidTimeframe(_)));
    }

    #[test]
    fn too_many_points_carries_requested_and_max() {
        let err = CandleQueryError::TooManyPoints {
            requested: 10_000,
            max: MAX_CANDLE_POINTS,
        };
        let msg = err.to_string();
        assert!(msg.contains("10000"));
        assert!(msg.contains("5000"));
    }
}
