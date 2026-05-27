//! Domain crate — pure business entities, value objects and rules.
//!
//! Currently exposes:
//!
//! - [`candle`] — OHLCV bucket aggregate, [`candle::Timeframe`] whitelist,
//!   the [`candle::MAX_CANDLE_POINTS`] cap, and [`candle::CandleQueryError`].
//! - [`order`]  — `orders` row entity, [`order::OrderSide`] / [`order::OrderStatus`]
//!   whitelists, the [`order::MAX_ORDER_ROWS`] cap, and [`order::OrderQueryError`].
//!
//! Decisions and indicators land with later issues (#9, #13).
//!
//! # Dependency rule
//!
//! This crate MUST NOT depend on `axum`, `sqlx`, `tokio` runtime, or any
//! other I/O crate. Allowed dependencies are limited to `serde`, `chrono`,
//! `thiserror`, `rust_decimal` and `uuid` (a `no_std`-friendly pure-Rust
//! 128-bit identifier). See `CLAUDE.md` for the full hexagonal architecture
//! contract.

#![forbid(unsafe_code)]

pub mod candle;
pub mod order;
