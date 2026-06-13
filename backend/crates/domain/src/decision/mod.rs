//! Decision domain — strategy decision read model, shared by the live and
//! backtest monitoring endpoints.
//!
//! A "decision" is one row of `strategy_decisions[_backtest]`: the strategy
//! engine evaluated the market and recorded a `reason` plus the number of
//! orders it emitted. For charting we also carry the optional market-context
//! `snapshot` (`decision_market_context[_backtest].snapshot`) so the UI can
//! render a tooltip.
//!
//! Introduced with the backtest lot (#10); the live decisions endpoint (#9)
//! will reuse it unchanged.

pub mod entity;

pub use entity::Decision;
