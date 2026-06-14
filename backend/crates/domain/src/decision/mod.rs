//! Decision domain — strategy decision read model, shared by the live and
//! backtest monitoring endpoints.
//!
//! A "decision" is one row of `strategy_decisions[_backtest]`: the strategy
//! engine evaluated the market and recorded a `reason` plus the number of
//! orders it emitted. For charting we also carry the optional market-context
//! `snapshot` (`decision_market_context[_backtest].snapshot`) so the UI can
//! render a tooltip.
//!
//! Introduced with the backtest lot (#10). The live decisions endpoint (#9)
//! uses a sibling [`live::LiveDecision`]: live decisions are ordered on the wall
//! clock (`created_at`) and carry the owning `session_id`.

pub mod entity;
pub mod live;

pub use entity::Decision;
pub use live::LiveDecision;
