//! Ports — traits implemented by outbound adapters.

pub mod health;

pub use health::{HealthCheckError, HealthChecker};
