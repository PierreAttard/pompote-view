//! Order domain — entities, enums and invariants for the `orders` table.
//!
//! Layout:
//!
//! - [`entity::Order`]        — one order row as seen by the viz backend
//! - [`side::OrderSide`]      — `buy` / `sell` whitelist
//! - [`status::OrderStatus`]  — the seven status values enforced by the
//!   Postgres `CHECK` constraint on `orders.status`
//! - [`MAX_ORDER_ROWS`]       — hard cap on the number of rows we ever
//!   return on a single HTTP response (mirrored as a SQL `LIMIT` safety
//!   net by the persistence adapter)
//! - [`OrderQueryError`]      — domain-level validation errors for the
//!   `GetOrders` use case (mapped to HTTP `400` at the inbound boundary)

pub mod entity;
pub mod side;
pub mod status;

use thiserror::Error;

pub use entity::Order;
pub use side::{InvalidOrderSide, OrderSide};
pub use status::{InvalidOrderStatus, OrderStatus};

/// Hard upper bound on the number of order rows returned by a single
/// `/api/v1/monitoring/strategies/:id/orders` request.
///
/// Enforced at three layers, mirroring the candles endpoint:
///
/// 1. The application use case validates `limit <= MAX_ORDER_ROWS` **before**
///    issuing any SQL, rejecting the request with [`OrderQueryError::TooManyRows`].
/// 2. The persistence adapter still wires `LIMIT $4` on the query with
///    `$4 = limit.min(MAX_ORDER_ROWS)` as defence-in-depth.
/// 3. The HTTP layer maps the error to `400 too_many_rows`, telling the
///    caller both the cap and what they requested.
pub const MAX_ORDER_ROWS: usize = 5_000;

/// Validation errors raised by the `GetOrders` use case.
///
/// All variants are mapped to HTTP `400 Bad Request` by the inbound adapter,
/// each with a distinct machine-readable `error` discriminator in the JSON
/// body so the front-end can localise the message.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum OrderQueryError {
    /// `from >= to`, or one of the bounds is not a valid RFC3339 timestamp.
    #[error("invalid range: `from` must be strictly before `to`")]
    InvalidRange,

    /// `limit` was zero or negative.
    #[error("invalid limit: must be strictly positive")]
    InvalidLimit,

    /// `limit` exceeded [`MAX_ORDER_ROWS`].
    ///
    /// The use case **never** truncates silently: it always rejects the
    /// request and asks the caller to lower the limit or narrow the window.
    /// This keeps the contract explicit (no silent data loss).
    #[error("too many rows: requested {requested}, max {max}")]
    TooManyRows {
        /// The number of rows the caller asked for.
        requested: usize,
        /// The hard cap, equal to [`MAX_ORDER_ROWS`].
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_many_rows_carries_requested_and_max() {
        let err = OrderQueryError::TooManyRows {
            requested: 10_000,
            max: MAX_ORDER_ROWS,
        };
        let msg = err.to_string();
        assert!(msg.contains("10000"));
        assert!(msg.contains("5000"));
    }

    #[test]
    fn invalid_limit_message_is_self_explanatory() {
        let err = OrderQueryError::InvalidLimit;
        assert!(err.to_string().contains("strictly positive"));
    }
}
