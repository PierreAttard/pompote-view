//! Domain crate — pure business entities, value objects and rules.
//!
//! Currently exposes:
//!
//! - [`candle`] — OHLCV bucket aggregate, [`candle::Timeframe`] whitelist,
//!   the [`candle::MAX_CANDLE_POINTS`] cap, and [`candle::CandleQueryError`].
//!
//! Decisions, markers and indicators land with later issues (#9, #10, #13).
//!
//! # Dependency rule
//!
//! This crate MUST NOT depend on `axum`, `sqlx`, `tokio` runtime, or any
//! other I/O crate. Allowed dependencies are limited to `serde`, `chrono`,
//! `thiserror` and `rust_decimal` (a `no_std`-friendly fixed-precision type).
//! See `CLAUDE.md` for the full hexagonal architecture contract.

#![forbid(unsafe_code)]

pub mod candle;
