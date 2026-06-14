//! Application use cases.

pub mod backtests;
pub mod candles;
pub mod orders;
pub mod readiness;
pub mod strategies;

pub use backtests::{
    BacktestCandleSource, BacktestCandles, GetBacktestCandlesError, GetBacktestError,
    GetBacktestMetricsError, GetBacktestRun, GetBacktestRunCandles, GetBacktestRunMetrics,
    GetBacktestSeries, GetBacktestSeriesInput, ListBacktestRuns, ListBacktestRunsInput,
};
pub use candles::{CandleSeries, GetCandles, GetCandlesError, GetCandlesInput};
pub use orders::{GetOrders, GetOrdersError, GetOrdersInput, OrderSeries};
pub use readiness::{ReadinessOutcome, ReadinessProbe};
pub use strategies::{
    GetStrategyDecisions, GetStrategyDecisionsInput, GetStrategyFills, GetStrategyFillsInput,
    ListStrategies, StrategyError,
};
