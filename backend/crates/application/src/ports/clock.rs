//! Clock port — abstracts "now" semantics so use cases stay deterministic.
//!
//! The HTTP handler defaults the optional `to` query parameter to the current
//! wall-clock time via this port. Tests inject a fixed-time fake.

use chrono::{DateTime, Utc};

/// Port: return the current UTC instant.
///
/// Intentionally synchronous (no `async_trait`): reading the system clock is
/// non-blocking and we don't want to force adapters into async wrappers.
pub trait Clock: Send + Sync {
    /// Returns the current UTC instant.
    fn now(&self) -> DateTime<Utc>;
}
