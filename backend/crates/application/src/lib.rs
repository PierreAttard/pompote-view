//! Application crate — use cases and ports (traits) for `pompote-view`.
//!
//! Ports are interfaces that adapters implement. The application layer
//! orchestrates the `domain` crate via these ports and never imports
//! `axum` or `sqlx` directly.

#![forbid(unsafe_code)]

pub mod ports;
pub mod use_cases;
