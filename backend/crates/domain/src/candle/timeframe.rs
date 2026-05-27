//! `Timeframe` value object — strict whitelist of supported aggregation widths.
//!
//! The viz backend never trusts a raw `&str` from the HTTP layer. Every
//! timeframe value flowing into the application use case must round-trip
//! through `Timeframe::try_from(&str)`, which both validates the input and
//! materialises the Postgres `INTERVAL` literal used by `time_bucket()`.
//!
//! The whitelist is intentionally narrow: the underlying hypertable
//! (`candles_5s`) has a 5-second base, so anything finer than `5s` would
//! aggregate a single row per bucket (pointless), and anything coarser than
//! `1d` exceeds the 5000-point cap on any reasonable window (the front-end
//! would have to ask for years of data to see a useful chart). See issue #8.

use std::fmt;

use thiserror::Error;

/// Supported aggregation widths for the `/api/v1/monitoring/candles` endpoint.
///
/// The discriminant order matches the input string order in
/// [`Timeframe::ALLOWED`] and intentionally goes from finest (`5s`) to
/// coarsest (`1d`) so error messages list the values in a predictable order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeframe {
    /// 5 seconds — equal to the base hypertable resolution.
    S5,
    /// 15 seconds.
    S15,
    /// 30 seconds.
    S30,
    /// 1 minute.
    M1,
    /// 3 minutes.
    M3,
    /// 5 minutes.
    M5,
    /// 15 minutes.
    M15,
    /// 30 minutes.
    M30,
    /// 1 hour.
    H1,
    /// 2 hours.
    H2,
    /// 4 hours.
    H4,
    /// 6 hours.
    H6,
    /// 12 hours.
    H12,
    /// 1 day.
    D1,
}

impl Timeframe {
    /// Ordered list of accepted timeframe input strings.
    ///
    /// Exposed publicly so the HTTP error handler can include it in the
    /// `400 invalid_timeframe` body without re-declaring the literals.
    pub const ALLOWED: &'static [&'static str] = &[
        "5s", "15s", "30s", "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "12h", "1d",
    ];

    /// Returns the canonical input string for this variant.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::S5 => "5s",
            Self::S15 => "15s",
            Self::S30 => "30s",
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::H6 => "6h",
            Self::H12 => "12h",
            Self::D1 => "1d",
        }
    }

    /// Returns the Postgres `INTERVAL` literal used by `time_bucket()` for
    /// this timeframe.
    ///
    /// Returned as `&'static str` so the persistence adapter binds a fully
    /// owned value (no allocation per request, no lifetime entanglement).
    pub fn to_pg_interval(&self) -> &'static str {
        match self {
            Self::S5 => "5 seconds",
            Self::S15 => "15 seconds",
            Self::S30 => "30 seconds",
            Self::M1 => "1 minute",
            Self::M3 => "3 minutes",
            Self::M5 => "5 minutes",
            Self::M15 => "15 minutes",
            Self::M30 => "30 minutes",
            Self::H1 => "1 hour",
            Self::H2 => "2 hours",
            Self::H4 => "4 hours",
            Self::H6 => "6 hours",
            Self::H12 => "12 hours",
            Self::D1 => "1 day",
        }
    }

    /// Width of one bucket, expressed in seconds.
    ///
    /// Used by the application use case to compute the theoretical bucket
    /// count `(to - from) / width` before issuing the SQL query.
    pub fn width_seconds(&self) -> i64 {
        match self {
            Self::S5 => 5,
            Self::S15 => 15,
            Self::S30 => 30,
            Self::M1 => 60,
            Self::M3 => 180,
            Self::M5 => 300,
            Self::M15 => 900,
            Self::M30 => 1_800,
            Self::H1 => 3_600,
            Self::H2 => 7_200,
            Self::H4 => 14_400,
            Self::H6 => 21_600,
            Self::H12 => 43_200,
            Self::D1 => 86_400,
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an input string does not match any [`Timeframe`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid timeframe `{input}` (allowed: {})", Timeframe::ALLOWED.join(", "))]
pub struct InvalidTimeframe {
    /// The offending input string (echoed back to help the caller debug).
    pub input: String,
}

impl TryFrom<&str> for Timeframe {
    type Error = InvalidTimeframe;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "5s" => Ok(Self::S5),
            "15s" => Ok(Self::S15),
            "30s" => Ok(Self::S30),
            "1m" => Ok(Self::M1),
            "3m" => Ok(Self::M3),
            "5m" => Ok(Self::M5),
            "15m" => Ok(Self::M15),
            "30m" => Ok(Self::M30),
            "1h" => Ok(Self::H1),
            "2h" => Ok(Self::H2),
            "4h" => Ok(Self::H4),
            "6h" => Ok(Self::H6),
            "12h" => Ok(Self::H12),
            "1d" => Ok(Self::D1),
            other => Err(InvalidTimeframe {
                input: other.to_string(),
            }),
        }
    }
}

impl std::str::FromStr for Timeframe {
    type Err = InvalidTimeframe;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_list_round_trips_through_try_from() {
        for raw in Timeframe::ALLOWED {
            let tf = Timeframe::try_from(*raw).expect("allowed literal must parse");
            assert_eq!(tf.as_str(), *raw);
        }
    }

    #[test]
    fn rejects_unknown_input() {
        let err = Timeframe::try_from("7s").unwrap_err();
        assert_eq!(err.input, "7s");
        assert!(err.to_string().contains("invalid timeframe"));
        assert!(err.to_string().contains("5s"));
    }

    #[test]
    fn pg_interval_matches_human_string() {
        assert_eq!(Timeframe::S5.to_pg_interval(), "5 seconds");
        assert_eq!(Timeframe::M1.to_pg_interval(), "1 minute");
        assert_eq!(Timeframe::H1.to_pg_interval(), "1 hour");
        assert_eq!(Timeframe::D1.to_pg_interval(), "1 day");
    }

    #[test]
    fn width_seconds_is_strictly_increasing() {
        let widths: Vec<i64> = Timeframe::ALLOWED
            .iter()
            .map(|raw| Timeframe::try_from(*raw).unwrap().width_seconds())
            .collect();
        for pair in widths.windows(2) {
            assert!(pair[0] < pair[1], "widths must be strictly increasing");
        }
    }

    #[test]
    fn display_matches_canonical_str() {
        assert_eq!(format!("{}", Timeframe::H4), "4h");
    }

    #[test]
    fn from_str_delegates_to_try_from() {
        let tf: Timeframe = "5m".parse().unwrap();
        assert_eq!(tf, Timeframe::M5);
    }
}
