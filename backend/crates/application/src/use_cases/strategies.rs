//! Strategy use cases — the UI selector (`ListStrategies`) and the live fills
//! series (`GetStrategyFills`).
//!
//! `GetStrategyFills` mirrors [`super::orders::GetOrders`]: it validates the
//! `[from, to)` window (defaulting `to` to `Clock::now()`) and the `limit`
//! against [`MAX_ORDER_ROWS`], then delegates to the [`StrategyRepository`].
//! `ListStrategies` needs no validation (the table is small — a selector, not a
//! series).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::fill::LiveFill;
use domain::order::{MAX_ORDER_ROWS, OrderQueryError};
use domain::strategy::Strategy;
use uuid::Uuid;

use crate::ports::{Clock, RepositoryError, StrategyFillQuery, StrategyRepository};

/// Top-level error shared by the strategy use cases.
#[derive(Debug, thiserror::Error)]
pub enum StrategyError {
    /// A domain invariant was violated (mapped to HTTP `400`).
    #[error(transparent)]
    Domain(#[from] OrderQueryError),
    /// The repository port reported an I/O error (mapped to `503` / `500`).
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Use case: list every strategy (the UI selector).
pub struct ListStrategies {
    repo: Arc<dyn StrategyRepository>,
}

impl ListStrategies {
    /// Builds the use case over the given repository port.
    pub fn new(repo: Arc<dyn StrategyRepository>) -> Self {
        Self { repo }
    }

    /// Returns every strategy, ordered by name.
    pub async fn run(&self) -> Result<Vec<Strategy>, StrategyError> {
        Ok(self.repo.list_strategies().await?)
    }
}

/// Input for [`GetStrategyFills::run`] (raw domain types, no HTTP concerns).
#[derive(Debug, Clone)]
pub struct GetStrategyFillsInput {
    /// Strategy identifier.
    pub strategy_id: Uuid,
    /// Inclusive lower bound on `executed_at`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `executed_at`. `None` means "use `Clock::now()`".
    pub to: Option<DateTime<Utc>>,
    /// Row cap. `None` means "use [`MAX_ORDER_ROWS`]".
    pub limit: Option<usize>,
}

/// Use case: fetch a strategy's live fills on a time window.
pub struct GetStrategyFills {
    repo: Arc<dyn StrategyRepository>,
    clock: Arc<dyn Clock>,
}

impl GetStrategyFills {
    /// Builds the use case over the given repository and clock ports.
    pub fn new(repo: Arc<dyn StrategyRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Validates the window/limit then queries the repository.
    pub async fn run(&self, input: GetStrategyFillsInput) -> Result<Vec<LiveFill>, StrategyError> {
        let to = input.to.unwrap_or_else(|| self.clock.now());
        if input.from >= to {
            return Err(StrategyError::Domain(OrderQueryError::InvalidRange));
        }
        let limit = match input.limit {
            None => MAX_ORDER_ROWS,
            Some(0) => return Err(StrategyError::Domain(OrderQueryError::InvalidLimit)),
            Some(n) if n > MAX_ORDER_ROWS => {
                return Err(StrategyError::Domain(OrderQueryError::TooManyRows {
                    requested: n,
                    max: MAX_ORDER_ROWS,
                }));
            }
            Some(n) => n,
        };
        let query = StrategyFillQuery {
            strategy_id: input.strategy_id,
            from: input.from,
            to,
            limit,
        };
        Ok(self.repo.fetch_fills_for_strategy(&query).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::TimeZone;
    use domain::order::OrderSide;
    use rust_decimal::Decimal;
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct FakeRepo {
        strategies: Vec<Strategy>,
        fills: Vec<LiveFill>,
        last_query: Mutex<Option<StrategyFillQuery>>,
    }

    #[async_trait]
    impl StrategyRepository for FakeRepo {
        async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError> {
            Ok(self.strategies.clone())
        }
        async fn fetch_fills_for_strategy(
            &self,
            query: &StrategyFillQuery,
        ) -> Result<Vec<LiveFill>, RepositoryError> {
            *self.last_query.lock().await = Some(query.clone());
            Ok(self.fills.clone())
        }
    }

    struct UnavailableRepo;
    #[async_trait]
    impl StrategyRepository for UnavailableRepo {
        async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
        async fn fetch_fills_for_strategy(
            &self,
            _query: &StrategyFillQuery,
        ) -> Result<Vec<LiveFill>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 1, h, 0, 0).unwrap()
    }

    fn fill() -> LiveFill {
        LiveFill {
            fill_id: Uuid::nil(),
            order_id: Uuid::nil(),
            side: OrderSide::Buy,
            price: Decimal::ZERO,
            quantity: Decimal::ZERO,
            fee: Decimal::ZERO,
            fee_asset: "USDT".into(),
            executed_at: t(10),
            is_paper: true,
        }
    }

    fn fills_uc(repo: Arc<FakeRepo>) -> GetStrategyFills {
        GetStrategyFills::new(repo, Arc::new(FixedClock(t(12))))
    }

    fn input(limit: Option<usize>, to: Option<DateTime<Utc>>) -> GetStrategyFillsInput {
        GetStrategyFillsInput {
            strategy_id: Uuid::nil(),
            from: t(10),
            to,
            limit,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_returns_strategies() {
        let repo = Arc::new(FakeRepo {
            strategies: vec![Strategy {
                id: Uuid::nil(),
                name: "ma_cross".into(),
                kind: "directional".into(),
                enabled_paper: true,
                enabled_live: false,
            }],
            ..Default::default()
        });
        let out = ListStrategies::new(repo).run().await.unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].enabled_paper);
        assert!(!out[0].enabled_live);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_propagates_unavailable() {
        let err = ListStrategies::new(Arc::new(UnavailableRepo))
            .run()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            StrategyError::Repository(RepositoryError::Unavailable(_))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_default_clock_now_and_max_rows() {
        let repo = Arc::new(FakeRepo::default());
        fills_uc(repo.clone()).run(input(None, None)).await.unwrap();
        let q = repo.last_query.lock().await.clone().unwrap();
        assert_eq!(q.to, t(12));
        assert_eq!(q.limit, MAX_ORDER_ROWS);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_reject_inverted_range() {
        let err = fills_uc(Arc::new(FakeRepo::default()))
            .run(input(None, Some(t(8))))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            StrategyError::Domain(OrderQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_reject_over_cap() {
        let err = fills_uc(Arc::new(FakeRepo::default()))
            .run(input(Some(MAX_ORDER_ROWS + 1), Some(t(11))))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            StrategyError::Domain(OrderQueryError::TooManyRows { .. })
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fills_returned_from_repo() {
        let repo = Arc::new(FakeRepo {
            fills: vec![fill()],
            ..Default::default()
        });
        let out = fills_uc(repo)
            .run(input(Some(10), Some(t(11))))
            .await
            .unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].is_paper);
    }
}
