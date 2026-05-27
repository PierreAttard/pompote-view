//! Domain crate — pure business entities, value objects and rules.
//!
//! This crate is intentionally minimal for now: the hexagonal skeleton
//! introduced by issue #7 only wires up `/healthz`, `/readyz` and the
//! `X-API-Key` middleware. Real domain entities (candles, decisions, markers)
//! land with the monitoring endpoints in issues #8 and following.
//!
//! # Dependency rule
//!
//! This crate MUST NOT depend on `axum`, `sqlx`, `tokio` runtime, or any
//! other I/O crate. Allowed dependencies are limited to `serde`, `chrono`
//! and `thiserror`. See `CLAUDE.md` for the full hexagonal architecture
//! contract.

#![forbid(unsafe_code)]
