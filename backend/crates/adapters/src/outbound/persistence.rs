//! Persistence adapter — sqlx-based implementations of `application` ports.
//!
//! Only a read-only pool against the `pompote_viz_reader` Postgres role is
//! expected here. No migration, no mutation: any `INSERT` / `UPDATE` /
//! `DELETE` would be rejected by the database itself thanks to `GRANT SELECT`
//! only.

use application::ports::{HealthCheckError, HealthChecker};
use async_trait::async_trait;
use sqlx::PgPool;

/// `HealthChecker` implementation backed by a Postgres connection pool.
///
/// `check()` issues `SELECT 1` against the pool. Any error is mapped to
/// [`HealthCheckError::Unavailable`] with the underlying message logged at
/// `warn` level by the caller (HTTP handler).
#[derive(Clone)]
pub struct SqlxHealthChecker {
    pool: PgPool,
}

impl SqlxHealthChecker {
    /// Wraps a Postgres pool in a `HealthChecker`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthChecker for SqlxHealthChecker {
    async fn check(&self) -> Result<(), HealthCheckError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| HealthCheckError::Unavailable(e.to_string()))
    }
}
