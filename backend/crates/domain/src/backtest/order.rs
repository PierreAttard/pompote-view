//! `BacktestOrder` entity — one row from the `orders_backtest` table.
//!
//! Mirrors the live [`crate::order::Order`] read model but on the **market
//! clock** axis (`market_ts` instead of `created_at`), per the locked
//! run-centric / replayed-time decision.
//!
//! Two intentional differences from the live `Order`:
//!
//! - **`status` is a free `String`, not [`crate::order::OrderStatus`].** The
//!   live `orders.status` column has a 7-value `CHECK` constraint; the backtest
//!   mirror `orders_backtest.status` has **no `CHECK`**. Re-parsing it against
//!   the live whitelist would turn any legitimate backtest-only status into a
//!   false "schema drift" `500`. We therefore forward it verbatim as a
//!   display-only string.
//! - `side` keeps its `CHECK (buy|sell)`, so it is still typed as
//!   [`crate::order::OrderSide`] and re-validated at the adapter boundary.
//!
//! `price` follows the same `COALESCE(limit_price, expected_price)` derivation
//! as the live read model (see [`crate::order::Order::price`]).
//!
//! No `serde::Serialize` is derived here — serialisation lives in the inbound
//! HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::order::OrderSide;

/// One `orders_backtest` row returned by the viz API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestOrder {
    /// Primary key (`orders_backtest.id`).
    pub order_id: Uuid,
    /// Foreign key to `strategy_decisions_backtest(id)`.
    pub decision_id: Uuid,
    /// Buy or sell (DB-enforced `CHECK`).
    pub side: OrderSide,
    /// Best-effort price: `COALESCE(limit_price, expected_price)`; `None` when
    /// neither is set (e.g. a `market` order without expected price).
    pub price: Option<Decimal>,
    /// Base-asset quantity (`orders_backtest.quantity`).
    pub quantity: Decimal,
    /// Raw status string. Free text: `orders_backtest.status` has no `CHECK`,
    /// so we never re-parse it against the live [`crate::order::OrderStatus`]
    /// whitelist (it is display-only here).
    pub status: String,
    /// Market-clock timestamp of the order (`orders_backtest.market_ts`).
    pub market_ts: DateTime<Utc>,
}
