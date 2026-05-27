//! `OrderStatus` value object — strict whitelist of order lifecycle states.
//!
//! Mirrors the seven values enforced by the Postgres `CHECK` constraint on
//! `orders.status`:
//!
//! ```text
//! ('submitted','rejected','error','deleted','filled','canceled','expired')
//! ```
//!
//! See [`OrderSide`](super::OrderSide) for the defence-in-depth rationale
//! on re-parsing DB-sourced strings.

use std::fmt;

use thiserror::Error;

/// Lifecycle state of an order, mirroring the DB `CHECK` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    /// Order was sent to the exchange.
    Submitted,
    /// Exchange rejected the order (e.g. insufficient balance).
    Rejected,
    /// Submission failed before reaching the exchange (network, validation).
    Error,
    /// Order was logically deleted on our side (soft delete).
    Deleted,
    /// Order was filled, partially or fully.
    Filled,
    /// Order was canceled (by the user or the engine).
    Canceled,
    /// Order expired without filling.
    Expired,
}

impl OrderStatus {
    /// Returns the canonical lowercase string (matches the DB `CHECK` set).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Rejected => "rejected",
            Self::Error => "error",
            Self::Deleted => "deleted",
            Self::Filled => "filled",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
        }
    }

    /// Ordered list of accepted input strings (for error messages).
    pub const ALLOWED: &'static [&'static str] = &[
        "submitted",
        "rejected",
        "error",
        "deleted",
        "filled",
        "canceled",
        "expired",
    ];
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an input string does not match a known [`OrderStatus`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid order status `{input}` (allowed: {})", OrderStatus::ALLOWED.join(", "))]
pub struct InvalidOrderStatus {
    /// The offending input string (echoed back for debugging).
    pub input: String,
}

impl TryFrom<&str> for OrderStatus {
    // Fully-qualified to disambiguate from the `Error` variant of `OrderStatus`.
    type Error = InvalidOrderStatus;

    fn try_from(value: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        match value {
            "submitted" => Ok(Self::Submitted),
            "rejected" => Ok(Self::Rejected),
            "error" => Ok(Self::Error),
            "deleted" => Ok(Self::Deleted),
            "filled" => Ok(Self::Filled),
            "canceled" => Ok(Self::Canceled),
            "expired" => Ok(Self::Expired),
            other => Err(InvalidOrderStatus {
                input: other.to_string(),
            }),
        }
    }
}

impl std::str::FromStr for OrderStatus {
    type Err = InvalidOrderStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_list_round_trips_through_try_from() {
        for raw in OrderStatus::ALLOWED {
            let status = OrderStatus::try_from(*raw).expect("allowed literal must parse");
            assert_eq!(status.as_str(), *raw);
        }
    }

    #[test]
    fn rejects_unknown_input() {
        let err = OrderStatus::try_from("partial").unwrap_err();
        assert_eq!(err.input, "partial");
        assert!(err.to_string().contains("invalid order status"));
        assert!(err.to_string().contains("submitted"));
    }

    #[test]
    fn allowed_set_has_exactly_seven_values() {
        // Guardrail: if `robot_rust` grows the CHECK constraint, this test
        // forces us to acknowledge it explicitly.
        assert_eq!(OrderStatus::ALLOWED.len(), 7);
    }

    #[test]
    fn display_matches_canonical_str() {
        assert_eq!(format!("{}", OrderStatus::Filled), "filled");
        assert_eq!(format!("{}", OrderStatus::Canceled), "canceled");
    }

    #[test]
    fn from_str_delegates_to_try_from() {
        let s: OrderStatus = "expired".parse().unwrap();
        assert_eq!(s, OrderStatus::Expired);
    }
}
