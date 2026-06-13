//! `Fill` entity — one `fills[_backtest]` row.
//!
//! All monetary fields use [`rust_decimal::Decimal`] so we never lose precision
//! at the I/O boundary (`NUMERIC(20,8)` columns); the HTTP DTO is the only
//! place that down-casts to `f64` for Lightweight Charts.
//!
//! `side` keeps its DB `CHECK (buy|sell)` and is therefore typed as
//! [`crate::order::OrderSide`]. Ordered on the **market clock** (`market_ts`)
//! per the run-centric decision.
//!
//! No `serde::Serialize` is derived here — serialisation lives in the inbound
//! HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::order::OrderSide;

/// One trade fill as seen by the viz API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    /// Primary key (`fills[_backtest].id`).
    pub fill_id: Uuid,
    /// Foreign key to the order that produced this fill.
    pub order_id: Uuid,
    /// Buy or sell (DB-enforced `CHECK`).
    pub side: OrderSide,
    /// Execution price (`fills[_backtest].price`).
    pub price: Decimal,
    /// Executed base-asset quantity.
    pub quantity: Decimal,
    /// Fee charged for this fill.
    pub fee: Decimal,
    /// Asset the fee was charged in (`fee_asset`).
    pub fee_asset: String,
    /// Market-clock timestamp of the fill (`market_ts`).
    pub market_ts: DateTime<Utc>,
}
