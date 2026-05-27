//! `OrderSide` value object — strict whitelist of order directions.
//!
//! The DB enforces `side IN ('buy', 'sell')` via a `CHECK` constraint, so
//! values reaching the domain *should* already be in that set. We still
//! re-validate at the adapter boundary (defence-in-depth) and surface
//! [`InvalidOrderSide`] when an unexpected value leaks through (typically
//! schema drift between `robot_rust` and the viz backend).

use std::fmt;

use thiserror::Error;

/// Supported order directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderSide {
    /// Buy / long entry / short exit.
    Buy,
    /// Sell / long exit / short entry.
    Sell,
}

impl OrderSide {
    /// Returns the canonical lowercase string (matches the DB `CHECK` set).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }

    /// Ordered list of accepted input strings (for error messages).
    pub const ALLOWED: &'static [&'static str] = &["buy", "sell"];
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an input string does not match a known [`OrderSide`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid order side `{input}` (allowed: {})", OrderSide::ALLOWED.join(", "))]
pub struct InvalidOrderSide {
    /// The offending input string (echoed back for debugging).
    pub input: String,
}

impl TryFrom<&str> for OrderSide {
    type Error = InvalidOrderSide;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            other => Err(InvalidOrderSide {
                input: other.to_string(),
            }),
        }
    }
}

impl std::str::FromStr for OrderSide {
    type Err = InvalidOrderSide;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_list_round_trips_through_try_from() {
        for raw in OrderSide::ALLOWED {
            let side = OrderSide::try_from(*raw).expect("allowed literal must parse");
            assert_eq!(side.as_str(), *raw);
        }
    }

    #[test]
    fn rejects_unknown_input() {
        let err = OrderSide::try_from("hodl").unwrap_err();
        assert_eq!(err.input, "hodl");
        assert!(err.to_string().contains("invalid order side"));
        assert!(err.to_string().contains("buy"));
        assert!(err.to_string().contains("sell"));
    }

    #[test]
    fn display_matches_canonical_str() {
        assert_eq!(format!("{}", OrderSide::Buy), "buy");
        assert_eq!(format!("{}", OrderSide::Sell), "sell");
    }

    #[test]
    fn from_str_delegates_to_try_from() {
        let s: OrderSide = "buy".parse().unwrap();
        assert_eq!(s, OrderSide::Buy);
    }

    #[test]
    fn rejects_uppercase_input() {
        // The DB stores lowercase; we mirror that contract strictly so a
        // schema-drift bug (e.g. someone uppercases the column) surfaces
        // as `RepositoryError::Internal` rather than silently passing.
        assert!(OrderSide::try_from("BUY").is_err());
    }
}
