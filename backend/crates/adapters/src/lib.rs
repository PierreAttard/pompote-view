//! Adapters crate — wires the application ports to concrete I/O technologies.
//!
//! Layout (option `a` from issue #7):
//!
//! - [`inbound::http`]   — axum router, handlers, DTOs, `X-API-Key` middleware
//! - [`outbound::persistence`] — sqlx `HealthChecker` implementation
//! - [`outbound::clock`] — `SystemClock` placeholder (no domain port yet)

#![forbid(unsafe_code)]

pub mod inbound;
pub mod outbound;
