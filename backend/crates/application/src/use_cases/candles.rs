//! `GetCandles` use case — validates a candles query then delegates to the
//! [`CandleRepository`] port.
//!
//! Domain invariants enforced here:
//!
//! - `from` strictly before `to` (after defaulting `to` to `Clock::now()`
//!   when the caller omitted it).
//! - `(to - from) / timeframe` must not exceed [`MAX_CANDLE_POINTS`].
//! - Timeframe validation is delegated to [`Timeframe::try_from`] at the
//!   inbound boundary; by the time we reach this use case the value is
//!   already typed.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::candle::{Candle, CandleQueryError, MAX_CANDLE_POINTS, Timeframe};

use crate::ports::{CandleQuery, CandleRepository, Clock, RepositoryError};

/// Outcome of a successful candles query.
///
/// Wraps `Vec<Candle>` in a newtype so we can attach metadata (cap saturation,
/// pagination cursors…) later without breaking the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandleSeries {
    /// The OHLCV buckets, ordered by ascending `open_time`.
    pub candles: Vec<Candle>,
}

/// Input parameters for [`GetCandles::run`], expressed as raw domain types
/// (no string parsing, no HTTP concerns).
#[derive(Debug, Clone)]
pub struct GetCandlesInput {
    /// Exchange identifier (e.g. `binance`).
    pub exchange: String,
    /// Symbol identifier (e.g. `BTCUSDT`).
    pub symbol: String,
    /// Aggregation width.
    pub timeframe: Timeframe,
    /// Inclusive lower bound on `open_time`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `open_time`. `None` means "use `Clock::now()`".
    pub to: Option<DateTime<Utc>>,
}

/// Use case: fetch aggregated OHLCV buckets.
pub struct GetCandles {
    repo: Arc<dyn CandleRepository>,
    clock: Arc<dyn Clock>,
}

impl GetCandles {
    /// Builds a new use case over the given repository and clock ports.
    pub fn new(repo: Arc<dyn CandleRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Validates the input and queries the repository.
    ///
    /// Returns:
    ///
    /// - `Ok(CandleSeries)` on success
    /// - `Err(GetCandlesError::Domain(CandleQueryError::*))` for client-side
    ///   validation failures (mapped to HTTP `400` by the adapter)
    /// - `Err(GetCandlesError::Repository(_))` for downstream I/O errors
    ///   (mapped to HTTP `503` or `500` by the adapter)
    pub async fn run(&self, input: GetCandlesInput) -> Result<CandleSeries, GetCandlesError> {
        let to = input.to.unwrap_or_else(|| self.clock.now());
        if input.from >= to {
            return Err(GetCandlesError::Domain(CandleQueryError::InvalidRange));
        }

        let bucket_count = estimate_bucket_count(input.from, to, input.timeframe);
        if bucket_count > MAX_CANDLE_POINTS {
            return Err(GetCandlesError::Domain(CandleQueryError::TooManyPoints {
                requested: bucket_count,
                max: MAX_CANDLE_POINTS,
            }));
        }

        let query = CandleQuery {
            exchange: input.exchange,
            symbol: input.symbol,
            timeframe: input.timeframe,
            from: input.from,
            to,
        };

        let candles = self.repo.fetch_aggregated(&query).await?;
        Ok(CandleSeries { candles })
    }
}

/// Top-level error of the `GetCandles` use case.
#[derive(Debug, thiserror::Error)]
pub enum GetCandlesError {
    /// A domain invariant was violated (mapped to HTTP `400`).
    #[error(transparent)]
    Domain(#[from] CandleQueryError),

    /// The repository port reported an I/O error.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Computes the number of buckets that would be returned for the given window.
///
/// Pure function exposed for unit tests.
///
/// Uses ceiling division: a partial bucket still counts (Timescale's
/// `time_bucket()` will emit a row aligned to the boundary, partial or not).
fn estimate_bucket_count(from: DateTime<Utc>, to: DateTime<Utc>, timeframe: Timeframe) -> usize {
    let width = timeframe.width_seconds();
    debug_assert!(width > 0);

    // `signed_duration_since` saturates rather than panicking on extreme dates,
    // and `saturating_add` then prevents the ceiling-division offset from
    // overflowing `i64::MAX` if a caller sends adversarial timestamps.
    let delta_secs = to.signed_duration_since(from).num_seconds();
    if delta_secs <= 0 {
        return 0;
    }

    // Ceiling division: `(a + b - 1) / b` for positive `a` and `b`.
    let buckets = delta_secs.saturating_add(width - 1) / width;
    // Buckets are positive and Postgres rows are `usize` here; capping at
    // `usize::MAX` is a no-op on 64-bit targets but keeps the conversion
    // explicit.
    usize::try_from(buckets).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{Duration, TimeZone};

    /// Fake repository that records the query and returns a canned response.
    struct FakeRepo {
        response: Vec<Candle>,
    }

    #[async_trait]
    impl CandleRepository for FakeRepo {
        async fn fetch_aggregated(
            &self,
            _query: &CandleQuery,
        ) -> Result<Vec<Candle>, RepositoryError> {
            Ok(self.response.clone())
        }
    }

    /// Fake repository that always reports the datastore as unavailable.
    struct UnavailableRepo;

    #[async_trait]
    impl CandleRepository for UnavailableRepo {
        async fn fetch_aggregated(
            &self,
            _query: &CandleQuery,
        ) -> Result<Vec<Candle>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    /// Fake clock pinned to a fixed instant.
    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn t(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
    }

    #[test]
    fn estimate_bucket_count_matches_ceiling_division() {
        let from = t(2026, 5, 1, 0);
        let to = from + Duration::hours(2) + Duration::minutes(30);
        // 2h30 of `1h` buckets = ceil(150 / 60) = 3
        assert_eq!(estimate_bucket_count(from, to, Timeframe::H1), 3);
        // 2h30 of `30m` buckets = 5
        assert_eq!(estimate_bucket_count(from, to, Timeframe::M30), 5);
    }

    #[test]
    fn estimate_bucket_count_handles_zero_window() {
        let from = t(2026, 5, 1, 0);
        assert_eq!(estimate_bucket_count(from, from, Timeframe::H1), 0);
    }

    #[test]
    fn estimate_bucket_count_at_cap_boundary() {
        let from = t(2026, 5, 1, 0);
        let to_5000 = from + Duration::hours(MAX_CANDLE_POINTS as i64);
        assert_eq!(
            estimate_bucket_count(from, to_5000, Timeframe::H1),
            MAX_CANDLE_POINTS
        );
        let to_5001 = from + Duration::hours(MAX_CANDLE_POINTS as i64 + 1);
        assert_eq!(
            estimate_bucket_count(from, to_5001, Timeframe::H1),
            MAX_CANDLE_POINTS + 1
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accepts_exactly_max_candle_points() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let from = t(2026, 5, 1, 0);
        let to = from + Duration::hours(MAX_CANDLE_POINTS as i64);
        uc.run(GetCandlesInput {
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            timeframe: Timeframe::H1,
            from,
            to: Some(to),
        })
        .await
        .expect("exactly MAX_CANDLE_POINTS buckets must be accepted");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_one_over_max_candle_points() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let from = t(2026, 5, 1, 0);
        let to = from + Duration::hours(MAX_CANDLE_POINTS as i64 + 1);
        let err = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::H1,
                from,
                to: Some(to),
            })
            .await
            .unwrap_err();
        match err {
            GetCandlesError::Domain(CandleQueryError::TooManyPoints { requested, max }) => {
                assert_eq!(max, MAX_CANDLE_POINTS);
                assert_eq!(requested, MAX_CANDLE_POINTS + 1);
            }
            other => panic!("expected TooManyPoints, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_inverted_range() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let err = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::H1,
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 8)),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetCandlesError::Domain(CandleQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_equal_bounds() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let when = t(2026, 5, 27, 10);
        let err = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::H1,
                from: when,
                to: Some(when),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetCandlesError::Domain(CandleQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_too_many_points() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        // 5s timeframe over 1 day = 17280 buckets — well above the 5000 cap.
        let from = t(2026, 5, 1, 0);
        let to = from + Duration::days(1);
        let err = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::S5,
                from,
                to: Some(to),
            })
            .await
            .unwrap_err();
        match err {
            GetCandlesError::Domain(CandleQueryError::TooManyPoints { requested, max }) => {
                assert_eq!(max, MAX_CANDLE_POINTS);
                assert!(requested > MAX_CANDLE_POINTS);
            }
            other => panic!("expected TooManyPoints, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn defaults_to_clock_now_when_to_is_absent() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let out = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::H1,
                from: t(2026, 5, 27, 10),
                to: None,
            })
            .await
            .expect("clock-now default must be accepted");
        assert!(out.candles.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn propagates_repository_unavailable() {
        let repo = Arc::new(UnavailableRepo);
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetCandles::new(repo, clock);
        let err = uc
            .run(GetCandlesInput {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                timeframe: Timeframe::H1,
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 11)),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetCandlesError::Repository(RepositoryError::Unavailable(_))
        ));
    }
}
