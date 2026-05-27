//! Health-check port.
//!
//! The `HealthChecker` port abstracts the readiness probe over the underlying
//! datastore. The persistence adapter implements it with a `SELECT 1` against
//! the read-only Postgres pool; tests use an in-memory fake.

use async_trait::async_trait;
use thiserror::Error;

/// Errors returned by a [`HealthChecker`] implementation.
///
/// The variant is intentionally opaque: the readiness endpoint only needs to
/// know whether the dependency is reachable, not the exact failure reason.
/// Adapters log the underlying error and surface this variant to the caller.
#[derive(Debug, Error)]
pub enum HealthCheckError {
    /// The downstream dependency (e.g. the Postgres pool) is unreachable.
    #[error("dependency unavailable: {0}")]
    Unavailable(String),
}

/// Port: probe an outbound dependency for readiness.
///
/// Implementations live in `adapters::outbound::persistence` (production) and
/// in the test modules of `application` / `adapters` (fakes).
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Returns `Ok(())` when the dependency is reachable, `Err` otherwise.
    async fn check(&self) -> Result<(), HealthCheckError>;
}
