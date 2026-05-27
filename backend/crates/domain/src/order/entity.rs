//! `Order` entity — one row from the `orders` table as seen by the viz API.
//!
//! Only the columns needed to render markers on the chart are exposed here.
//! The full row (session_id, strategy_kind, exchange, symbol, is_paper, …)
//! is not part of the read model: the front-end already knows which strategy
//! it is monitoring, so we keep the wire format compact.
//!
//! All numeric fields use [`rust_decimal::Decimal`] so we never lose precision
//! at the I/O boundary; the HTTP DTO is the only place that down-casts to
//! `f64` for Lightweight Charts.
//!
//! `price` is intentionally `Option<Decimal>`: for `market` orders without
//! `expected_price`, the database has nothing to surface as a "best-effort
//! price" and we forward the `null` to the caller rather than fabricating
//! a placeholder (e.g. `0`) that would mislead the chart.
//!
//! No `serde::Serialize` is derived here on purpose — this is a pure domain
//! entity, serialisation lives in the inbound HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::{OrderSide, OrderStatus};

/// One order row returned by the viz API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// Primary key (`orders.id`).
    pub order_id: Uuid,
    /// Foreign key to `strategy_decisions(id)`.
    pub decision_id: Uuid,
    /// Buy or sell.
    pub side: OrderSide,
    /// Best-effort price: `COALESCE(limit_price, expected_price)`.
    ///
    /// - For `limit` / `amend` orders, this is the explicit `limit_price`.
    /// - For `market` orders, this is the `expected_price` recorded by the
    ///   strategy engine just before submission, when available.
    /// - When neither is known (e.g. `market` without expected price), this
    ///   stays `None` and the DTO forwards a JSON `null`.
    pub price: Option<Decimal>,
    /// Base-asset quantity (`orders.quantity`).
    pub quantity: Decimal,
    /// Order status (one of the seven values in [`OrderStatus`]).
    pub status: OrderStatus,
    /// Wall-clock time the row was inserted (`orders.created_at`).
    pub created_at: DateTime<Utc>,
}
