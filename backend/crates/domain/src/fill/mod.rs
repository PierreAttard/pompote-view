//! Fill domain — trade fill read model, shared by the live and backtest
//! monitoring endpoints.
//!
//! A "fill" is one row of `fills[_backtest]`: an order that (partially or
//! fully) executed, with its price, quantity and fee.
//!
//! Introduced with the backtest lot (#10); the live fills endpoint (#11) will
//! reuse it unchanged.

pub mod entity;

pub use entity::Fill;
