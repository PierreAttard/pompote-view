//! `Decision` entity — one `strategy_decisions[_backtest]` row plus its
//! optional market-context snapshot.
//!
//! The `snapshot` is an opaque, display-only `jsonb` value joined from
//! `decision_market_context[_backtest]`. It is `Option` because the join is a
//! `LEFT JOIN`: a decision may have no recorded context. The viz backend never
//! interprets its contents — it forwards them to the front-end unchanged.
//!
//! Ordered on the **market clock** (`market_ts`) per the run-centric decision.
//!
//! No `serde::Serialize` is derived here — serialisation lives in the inbound
//! HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// One strategy decision as seen by the viz API.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// Primary key (`strategy_decisions[_backtest].id`).
    pub decision_id: Uuid,
    /// Market-clock timestamp of the decision (`market_ts`).
    pub market_ts: DateTime<Utc>,
    /// Free-text rationale recorded by the strategy engine.
    pub reason: String,
    /// Number of orders emitted by this decision (`orders_count`).
    pub orders_count: i32,
    /// Opaque market-context snapshot (`LEFT JOIN`; `None` when absent).
    pub snapshot: Option<Value>,
}
