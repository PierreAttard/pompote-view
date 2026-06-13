//! Domain crate — pure business entities, value objects and rules.
//!
//! Currently exposes:
//!
//! - [`candle`] — OHLCV bucket aggregate, [`candle::Timeframe`] whitelist,
//!   the [`candle::MAX_CANDLE_POINTS`] cap, and [`candle::CandleQueryError`].
//! - [`order`]  — `orders` row entity, [`order::OrderSide`] / [`order::OrderStatus`]
//!   whitelists, the [`order::MAX_ORDER_ROWS`] cap, and [`order::OrderQueryError`].
//! - [`backtest`] — `backtest_runs` row entity, [`backtest::BacktestStatus`]
//!   whitelist, [`backtest::BacktestOrder`], the [`backtest::MAX_BACKTEST_RUNS`]
//!   cap, and [`backtest::BacktestQueryError`] (Lot 10).
//! - [`decision`] / [`fill`] — strategy decision and trade fill read models,
//!   shared by the backtest (#10) and live (#9 / #11) monitoring endpoints.
//!
//! Indicators land with later issues (#13).
//!
//! # Dependency rule
//!
//! This crate MUST NOT depend on `axum`, `sqlx`, `tokio` runtime, or any
//! other I/O crate. Allowed dependencies are limited to `serde`, `serde_json`
//! (pure in-memory `Value` for opaque JSONB snapshots), `chrono`,
//! `thiserror`, `rust_decimal` and `uuid` (a `no_std`-friendly pure-Rust
//! 128-bit identifier). See `CLAUDE.md` for the full hexagonal architecture
//! contract.

#![forbid(unsafe_code)]

pub mod backtest;
pub mod candle;
pub mod decision;
pub mod fill;
pub mod order;
