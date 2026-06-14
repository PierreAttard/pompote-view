//! Fill domain — trade fill read model, shared by the live and backtest
//! monitoring endpoints.
//!
//! A "fill" is one row of `fills[_backtest]`: an order that (partially or
//! fully) executed, with its price, quantity and fee.
//!
//! Introduced with the backtest lot (#10). The live fills endpoint (#11) uses a
//! sibling [`live::LiveFill`] read model: live fills share the monetary fields
//! but are ordered on the wall clock (`executed_at`) and carry `is_paper`.

pub mod entity;
pub mod live;

pub use entity::Fill;
pub use live::LiveFill;
