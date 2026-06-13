//! Shared application state injected into axum handlers via `State`.

use std::sync::Arc;

use application::use_cases::{
    GetBacktestRun, GetBacktestSeries, GetCandles, GetOrders, ListBacktestRuns, ReadinessProbe,
};

/// Immutable runtime state shared by every HTTP handler.
///
/// Cloning is cheap because every field is an `Arc`. `AppState` is wired by
/// `bootstrap` at startup and passed to `Router::with_state`.
#[derive(Clone)]
pub struct AppState {
    /// Readiness probe use case, used by `GET /readyz`.
    pub readiness: Arc<ReadinessProbe>,
    /// Expected `X-API-Key` value for the `/api/v1/*` middleware.
    ///
    /// Stored as raw bytes to allow constant-time comparison via `subtle`.
    pub api_key: Arc<Vec<u8>>,
    /// `GET /api/v1/monitoring/candles` use case.
    pub get_candles: Arc<GetCandles>,
    /// `GET /api/v1/monitoring/strategies/:id/orders` use case.
    pub get_orders: Arc<GetOrders>,
    /// `GET /api/v1/monitoring/backtests` use case (run listing).
    pub list_backtest_runs: Arc<ListBacktestRuns>,
    /// `GET /api/v1/monitoring/backtests/:run_id` use case (run detail).
    pub get_backtest_run: Arc<GetBacktestRun>,
    /// `GET /api/v1/monitoring/backtests/:run_id/{orders,fills,decisions}` use case.
    pub get_backtest_series: Arc<GetBacktestSeries>,
}

/// Test-only stubs shared across the HTTP module's unit tests.
///
/// Centralised here so each handler/router test does not re-declare no-op
/// repositories for the growing `AppState`. Two entry points:
///
/// - [`stub_backtest_use_cases`] — the three backtest use cases wired to an
///   empty repository, for tests that only exercise candles/orders/router.
/// - [`state_with_backtests`] — a full `AppState` with everything stubbed
///   except the caller-supplied backtest use cases, for the backtest handler
///   tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use application::ports::{
        BacktestRepository, BacktestRunListQuery, BacktestSeriesQuery, CandleQuery,
        CandleRepository, Clock, HealthCheckError, HealthChecker, OrderQuery, OrderRepository,
        RepositoryError,
    };
    use application::use_cases::{
        GetBacktestRun, GetBacktestSeries, GetCandles, GetOrders, ListBacktestRuns, ReadinessProbe,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use domain::backtest::{BacktestOrder, BacktestRun, BacktestRunDetail};
    use domain::candle::Candle;
    use domain::decision::Decision;
    use domain::fill::Fill;
    use domain::order::Order;
    use uuid::Uuid;

    use super::AppState;

    struct AlwaysOk;
    #[async_trait]
    impl HealthChecker for AlwaysOk {
        async fn check(&self) -> Result<(), HealthCheckError> {
            Ok(())
        }
    }

    struct EmptyCandles;
    #[async_trait]
    impl CandleRepository for EmptyCandles {
        async fn fetch_aggregated(&self, _q: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct EmptyOrders;
    #[async_trait]
    impl OrderRepository for EmptyOrders {
        async fn fetch_orders_for_strategy(
            &self,
            _q: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct EmptyBacktests;
    #[async_trait]
    impl BacktestRepository for EmptyBacktests {
        async fn list_runs(
            &self,
            _q: &BacktestRunListQuery,
        ) -> Result<Vec<BacktestRun>, RepositoryError> {
            Ok(vec![])
        }
        async fn get_run(&self, _id: Uuid) -> Result<Option<BacktestRunDetail>, RepositoryError> {
            Ok(None)
        }
        async fn fetch_orders(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<BacktestOrder>, RepositoryError> {
            Ok(vec![])
        }
        async fn fetch_fills(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<Fill>, RepositoryError> {
            Ok(vec![])
        }
        async fn fetch_decisions(
            &self,
            _q: &BacktestSeriesQuery,
        ) -> Result<Vec<Decision>, RepositoryError> {
            Ok(vec![])
        }
    }

    struct NowClock;
    impl Clock for NowClock {
        fn now(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    /// Builds the three backtest use cases over an empty repository.
    pub(crate) fn stub_backtest_use_cases() -> (
        Arc<ListBacktestRuns>,
        Arc<GetBacktestRun>,
        Arc<GetBacktestSeries>,
    ) {
        let repo = Arc::new(EmptyBacktests);
        (
            Arc::new(ListBacktestRuns::new(repo.clone())),
            Arc::new(GetBacktestRun::new(repo.clone())),
            Arc::new(GetBacktestSeries::new(repo, Arc::new(NowClock))),
        )
    }

    /// Builds an `AppState` with everything stubbed except the supplied
    /// backtest use cases.
    pub(crate) fn state_with_backtests(
        list_backtest_runs: Arc<ListBacktestRuns>,
        get_backtest_run: Arc<GetBacktestRun>,
        get_backtest_series: Arc<GetBacktestSeries>,
    ) -> AppState {
        AppState {
            readiness: Arc::new(ReadinessProbe::new(Arc::new(AlwaysOk))),
            api_key: Arc::new(b"unused".to_vec()),
            get_candles: Arc::new(GetCandles::new(Arc::new(EmptyCandles), Arc::new(NowClock))),
            get_orders: Arc::new(GetOrders::new(Arc::new(EmptyOrders), Arc::new(NowClock))),
            list_backtest_runs,
            get_backtest_run,
            get_backtest_series,
        }
    }
}
