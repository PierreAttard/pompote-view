//! Ports — traits implemented by outbound adapters.

pub mod backtest_repository;
pub mod candle_repository;
pub mod clock;
pub mod health;
pub mod order_repository;

pub use backtest_repository::{BacktestRepository, BacktestRunListQuery, BacktestSeriesQuery};
pub use candle_repository::{CandleQuery, CandleRepository, RepositoryError};
pub use clock::Clock;
pub use health::{HealthCheckError, HealthChecker};
pub use order_repository::{OrderQuery, OrderRepository};
