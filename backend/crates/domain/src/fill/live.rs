//! `LiveFill` entity — one row of the live `fills` table.
//!
//! Distinct from the shared [`super::Fill`] (backtest, market-clock axis): live
//! fills are ordered on the **wall clock** `executed_at` and carry `is_paper`,
//! so the live monitoring view can place execution markers in real time and
//! badge/filter paper vs live trades.
//!
//! Monetary fields use [`rust_decimal::Decimal`] (the `NUMERIC(20,8)` columns);
//! the HTTP DTO is the only place that down-casts to `f64`. `side` keeps its DB
//! `CHECK (buy|sell)` and is typed as [`crate::order::OrderSide`].

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::order::OrderSide;

/// One live trade fill as seen by the viz API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFill {
    /// Primary key (`fills.id`).
    pub fill_id: Uuid,
    /// Foreign key to the order that produced this fill.
    pub order_id: Uuid,
    /// Buy or sell (DB-enforced `CHECK`).
    pub side: OrderSide,
    /// Execution price (`fills.price`).
    pub price: Decimal,
    /// Executed base-asset quantity.
    pub quantity: Decimal,
    /// Fee charged for this fill.
    pub fee: Decimal,
    /// Asset the fee was charged in (`fee_asset`).
    pub fee_asset: String,
    /// Wall-clock execution time (`fills.executed_at`).
    pub executed_at: DateTime<Utc>,
    /// Whether this fill is paper trading (`fills.is_paper`).
    pub is_paper: bool,
}
