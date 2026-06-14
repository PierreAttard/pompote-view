//! `Strategy` read model — one row of the `strategies` table as seen by the
//! viz API.
//!
//! This is a **read-only re-implementation** independent of robot_rust's
//! control-plane API: the viz backend reads the `strategies` table directly to
//! populate the UI selector. Only the columns the selector needs are exposed
//! (no `config` JSON, no version/timestamps).
//!
//! `enabled_paper` / `enabled_live` surface the trading mode the strategy is
//! enabled for, so the UI can badge/filter paper vs live runs. The viz backend
//! never acts on these flags — it only displays them.
//!
//! No `serde::Serialize` is derived here — serialisation lives in the inbound
//! HTTP adapter as a DTO.

use uuid::Uuid;

/// One strategy row returned by the viz API selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strategy {
    /// Primary key (`strategies.id`).
    pub id: Uuid,
    /// Unique human-readable name (`strategies.name`).
    pub name: String,
    /// Strategy kind/family (`strategies.kind`).
    pub kind: String,
    /// Whether the strategy is enabled for paper trading.
    pub enabled_paper: bool,
    /// Whether the strategy is enabled for live trading.
    pub enabled_live: bool,
}
