//! `LiveDecision` entity — one live `strategy_decisions` row plus its optional
//! market-context snapshot.
//!
//! Distinct from the shared [`super::Decision`] (backtest, market-clock axis):
//! live decisions are ordered on the **wall clock** `created_at` and carry the
//! owning `session_id`. The `market_context` is an opaque, display-only `jsonb`
//! value joined from `decision_market_context` (`LEFT JOIN`, so `None` when no
//! context was recorded). The viz backend never interprets it.
//!
//! No `serde::Serialize` is derived here — serialisation lives in the inbound
//! HTTP adapter as a DTO.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// One live strategy decision as seen by the viz API.
///
/// No `Eq`: `market_context` is a `serde_json::Value` (carries `f64`).
#[derive(Debug, Clone, PartialEq)]
pub struct LiveDecision {
    /// Primary key (`strategy_decisions.id`).
    pub decision_id: Uuid,
    /// Owning trading session (`session_id`).
    pub session_id: Uuid,
    /// Wall-clock time the decision was recorded (`created_at`).
    pub created_at: DateTime<Utc>,
    /// Free-text rationale recorded by the strategy engine.
    pub reason: String,
    /// Number of orders emitted by this decision (`orders_count`).
    pub orders_count: i32,
    /// Opaque market-context snapshot (`LEFT JOIN`; `None` when absent).
    pub market_context: Option<Value>,
}
