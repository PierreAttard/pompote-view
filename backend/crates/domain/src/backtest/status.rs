//! `BacktestStatus` value object — strict whitelist of backtest run states.
//!
//! Mirrors the four values enforced by the Postgres `CHECK` constraint on
//! `backtest_runs.status`:
//!
//! ```text
//! ('running','completed','failed','aborted')
//! ```
//!
//! Same defence-in-depth rationale as [`crate::order::OrderStatus`]: the DB
//! already guarantees the set via a `CHECK`, but we re-parse at the adapter
//! boundary so a schema drift between `robot_rust` and the viz backend
//! surfaces as an explicit error rather than silent corruption.

use std::fmt;

use thiserror::Error;

/// Lifecycle state of a backtest run, mirroring the DB `CHECK` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BacktestStatus {
    /// The run is still replaying (no `ended_at` yet).
    Running,
    /// The run finished normally.
    Completed,
    /// The run stopped on an error (`backtest_runs.error` is usually set).
    Failed,
    /// The run was deliberately aborted before completion.
    Aborted,
}

impl BacktestStatus {
    /// Returns the canonical lowercase string (matches the DB `CHECK` set).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
        }
    }

    /// Ordered list of accepted input strings (for error messages).
    pub const ALLOWED: &'static [&'static str] = &["running", "completed", "failed", "aborted"];
}

impl fmt::Display for BacktestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an input string does not match a known [`BacktestStatus`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid backtest status `{input}` (allowed: {})", BacktestStatus::ALLOWED.join(", "))]
pub struct InvalidBacktestStatus {
    /// The offending input string (echoed back for debugging).
    pub input: String,
}

impl TryFrom<&str> for BacktestStatus {
    type Error = InvalidBacktestStatus;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "aborted" => Ok(Self::Aborted),
            other => Err(InvalidBacktestStatus {
                input: other.to_string(),
            }),
        }
    }
}

impl std::str::FromStr for BacktestStatus {
    type Err = InvalidBacktestStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_list_round_trips_through_try_from() {
        for raw in BacktestStatus::ALLOWED {
            let status = BacktestStatus::try_from(*raw).expect("allowed literal must parse");
            assert_eq!(status.as_str(), *raw);
        }
    }

    #[test]
    fn rejects_unknown_input() {
        let err = BacktestStatus::try_from("paused").unwrap_err();
        assert_eq!(err.input, "paused");
        assert!(err.to_string().contains("invalid backtest status"));
        assert!(err.to_string().contains("running"));
    }

    #[test]
    fn allowed_set_has_exactly_four_values() {
        // Guardrail: if `robot_rust` grows the CHECK constraint, this test
        // forces us to acknowledge it explicitly.
        assert_eq!(BacktestStatus::ALLOWED.len(), 4);
    }

    #[test]
    fn display_matches_canonical_str() {
        assert_eq!(format!("{}", BacktestStatus::Completed), "completed");
        assert_eq!(format!("{}", BacktestStatus::Aborted), "aborted");
    }

    #[test]
    fn from_str_delegates_to_try_from() {
        let s: BacktestStatus = "failed".parse().unwrap();
        assert_eq!(s, BacktestStatus::Failed);
    }
}
