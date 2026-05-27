//! Readiness use case.
//!
//! Probes the configured [`HealthChecker`] port and maps the result to a
//! transport-agnostic [`ReadinessOutcome`]. The HTTP adapter converts that
//! outcome to an HTTP status code (200 / 503).

use std::sync::Arc;

use crate::ports::HealthChecker;

/// Outcome of a readiness probe, independent of any transport layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessOutcome {
    /// All probed dependencies answered successfully.
    Ready,
    /// At least one dependency is unreachable.
    NotReady,
}

/// Use case: probe outbound dependencies for readiness.
pub struct ReadinessProbe {
    health: Arc<dyn HealthChecker>,
}

impl ReadinessProbe {
    /// Builds a new probe over the given health-check port.
    pub fn new(health: Arc<dyn HealthChecker>) -> Self {
        Self { health }
    }

    /// Runs the probe. Returns [`ReadinessOutcome::Ready`] on success,
    /// [`ReadinessOutcome::NotReady`] when the dependency fails.
    pub async fn run(&self) -> ReadinessOutcome {
        match self.health.check().await {
            Ok(()) => ReadinessOutcome::Ready,
            Err(_) => ReadinessOutcome::NotReady,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::HealthCheckError;
    use async_trait::async_trait;

    struct AlwaysOk;

    #[async_trait]
    impl HealthChecker for AlwaysOk {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    struct AlwaysDown;

    #[async_trait]
    impl HealthChecker for AlwaysDown {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Err(HealthCheckError::Unavailable("simulated outage".into()))
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ready_when_dependency_ok() {
        let probe = ReadinessProbe::new(Arc::new(AlwaysOk));
        assert_eq!(probe.run().await, ReadinessOutcome::Ready);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn not_ready_when_dependency_down() {
        let probe = ReadinessProbe::new(Arc::new(AlwaysDown));
        assert_eq!(probe.run().await, ReadinessOutcome::NotReady);
    }
}
