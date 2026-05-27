//! Ports — traits implemented by outbound adapters.

pub mod candle_repository;
pub mod clock;
pub mod health;

pub use candle_repository::{CandleQuery, CandleRepository, RepositoryError};
pub use clock::Clock;
pub use health::{HealthCheckError, HealthChecker};
