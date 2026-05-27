//! `GetOrders` use case — validates an orders query then delegates to the
//! [`OrderRepository`] port.
//!
//! Domain invariants enforced here:
//!
//! - `from` strictly before `to` (after defaulting `to` to `Clock::now()`
//!   when the caller omitted it).
//! - `limit` strictly positive.
//! - `limit` must not exceed [`MAX_ORDER_ROWS`].

use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::order::{MAX_ORDER_ROWS, Order, OrderQueryError};
use uuid::Uuid;

use crate::ports::{Clock, OrderQuery, OrderRepository, RepositoryError};

/// Outcome of a successful orders query.
///
/// Wraps `Vec<Order>` in a newtype so we can attach metadata (cap saturation,
/// pagination cursors…) later without breaking the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSeries {
    /// The order rows, ordered by ascending `created_at`.
    pub orders: Vec<Order>,
}

/// Input parameters for [`GetOrders::run`], expressed as raw domain types
/// (no string parsing, no HTTP concerns).
#[derive(Debug, Clone)]
pub struct GetOrdersInput {
    /// Strategy identifier (UUID parsed by the inbound layer).
    pub strategy_id: Uuid,
    /// Inclusive lower bound on `created_at`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `created_at`. `None` means "use `Clock::now()`".
    pub to: Option<DateTime<Utc>>,
    /// Row cap. `None` means "use [`MAX_ORDER_ROWS`]".
    pub limit: Option<usize>,
}

/// Use case: fetch order rows for a strategy on a time window.
pub struct GetOrders {
    repo: Arc<dyn OrderRepository>,
    clock: Arc<dyn Clock>,
}

impl GetOrders {
    /// Builds a new use case over the given repository and clock ports.
    pub fn new(repo: Arc<dyn OrderRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Validates the input and queries the repository.
    ///
    /// Returns:
    ///
    /// - `Ok(OrderSeries)` on success
    /// - `Err(GetOrdersError::Domain(OrderQueryError::*))` for client-side
    ///   validation failures (mapped to HTTP `400` by the adapter)
    /// - `Err(GetOrdersError::Repository(_))` for downstream I/O errors
    ///   (mapped to HTTP `503` or `500` by the adapter)
    pub async fn run(&self, input: GetOrdersInput) -> Result<OrderSeries, GetOrdersError> {
        let to = input.to.unwrap_or_else(|| self.clock.now());
        if input.from >= to {
            return Err(GetOrdersError::Domain(OrderQueryError::InvalidRange));
        }

        let limit = match input.limit {
            None => MAX_ORDER_ROWS,
            Some(0) => return Err(GetOrdersError::Domain(OrderQueryError::InvalidLimit)),
            Some(n) if n > MAX_ORDER_ROWS => {
                return Err(GetOrdersError::Domain(OrderQueryError::TooManyRows {
                    requested: n,
                    max: MAX_ORDER_ROWS,
                }));
            }
            Some(n) => n,
        };

        let query = OrderQuery {
            strategy_id: input.strategy_id,
            from: input.from,
            to,
            limit,
        };

        let orders = self.repo.fetch_orders_for_strategy(&query).await?;
        Ok(OrderSeries { orders })
    }
}

/// Top-level error of the `GetOrders` use case.
#[derive(Debug, thiserror::Error)]
pub enum GetOrdersError {
    /// A domain invariant was violated (mapped to HTTP `400`).
    #[error(transparent)]
    Domain(#[from] OrderQueryError),

    /// The repository port reported an I/O error.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::TimeZone;
    use domain::order::{OrderSide, OrderStatus};

    /// Fake repository that records the query and returns a canned response.
    struct FakeRepo {
        response: Vec<Order>,
    }

    #[async_trait]
    impl OrderRepository for FakeRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _query: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Ok(self.response.clone())
        }
    }

    /// Fake repository that always reports the datastore as unavailable.
    struct UnavailableRepo;

    #[async_trait]
    impl OrderRepository for UnavailableRepo {
        async fn fetch_orders_for_strategy(
            &self,
            _query: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            Err(RepositoryError::Unavailable("simulated".into()))
        }
    }

    /// Records the query passed to the repository so tests can inspect the
    /// `limit` actually propagated to the SQL layer.
    struct SpyRepo {
        captured: tokio::sync::Mutex<Option<OrderQuery>>,
    }

    #[async_trait]
    impl OrderRepository for SpyRepo {
        async fn fetch_orders_for_strategy(
            &self,
            query: &OrderQuery,
        ) -> Result<Vec<Order>, RepositoryError> {
            *self.captured.lock().await = Some(query.clone());
            Ok(vec![])
        }
    }

    /// Fake clock pinned to a fixed instant.
    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn t(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
    }

    fn sample_order() -> Order {
        Order {
            order_id: Uuid::nil(),
            decision_id: Uuid::nil(),
            side: OrderSide::Buy,
            price: None,
            quantity: rust_decimal::Decimal::ZERO,
            status: OrderStatus::Submitted,
            created_at: t(2026, 5, 27, 12),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accepts_exactly_max_order_rows() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        uc.run(GetOrdersInput {
            strategy_id: Uuid::nil(),
            from: t(2026, 5, 27, 10),
            to: Some(t(2026, 5, 27, 11)),
            limit: Some(MAX_ORDER_ROWS),
        })
        .await
        .expect("exactly MAX_ORDER_ROWS must be accepted");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_one_over_max_order_rows() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let err = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 11)),
                limit: Some(MAX_ORDER_ROWS + 1),
            })
            .await
            .unwrap_err();
        match err {
            GetOrdersError::Domain(OrderQueryError::TooManyRows { requested, max }) => {
                assert_eq!(max, MAX_ORDER_ROWS);
                assert_eq!(requested, MAX_ORDER_ROWS + 1);
            }
            other => panic!("expected TooManyRows, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_zero_limit() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let err = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 11)),
                limit: Some(0),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetOrdersError::Domain(OrderQueryError::InvalidLimit)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_inverted_range() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let err = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 8)),
                limit: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetOrdersError::Domain(OrderQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejects_equal_bounds() {
        let repo = Arc::new(FakeRepo { response: vec![] });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let when = t(2026, 5, 27, 10);
        let err = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: when,
                to: Some(when),
                limit: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetOrdersError::Domain(OrderQueryError::InvalidRange)
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn defaults_to_clock_now_when_to_is_absent() {
        let repo = Arc::new(SpyRepo {
            captured: tokio::sync::Mutex::new(None),
        });
        let now = t(2026, 5, 27, 12);
        let clock = Arc::new(FixedClock(now));
        let uc = GetOrders::new(repo.clone(), clock);
        uc.run(GetOrdersInput {
            strategy_id: Uuid::nil(),
            from: t(2026, 5, 27, 10),
            to: None,
            limit: None,
        })
        .await
        .expect("clock-now default must be accepted");
        let captured = repo.captured.lock().await.clone().unwrap();
        assert_eq!(captured.to, now);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn defaults_to_max_when_limit_is_absent() {
        let repo = Arc::new(SpyRepo {
            captured: tokio::sync::Mutex::new(None),
        });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo.clone(), clock);
        uc.run(GetOrdersInput {
            strategy_id: Uuid::nil(),
            from: t(2026, 5, 27, 10),
            to: Some(t(2026, 5, 27, 11)),
            limit: None,
        })
        .await
        .unwrap();
        let captured = repo.captured.lock().await.clone().unwrap();
        assert_eq!(captured.limit, MAX_ORDER_ROWS);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn propagates_repository_unavailable() {
        let repo = Arc::new(UnavailableRepo);
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let err = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 11)),
                limit: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GetOrdersError::Repository(RepositoryError::Unavailable(_))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn returns_orders_from_repository() {
        let repo = Arc::new(FakeRepo {
            response: vec![sample_order()],
        });
        let clock = Arc::new(FixedClock(t(2026, 5, 27, 12)));
        let uc = GetOrders::new(repo, clock);
        let out = uc
            .run(GetOrdersInput {
                strategy_id: Uuid::nil(),
                from: t(2026, 5, 27, 10),
                to: Some(t(2026, 5, 27, 11)),
                limit: None,
            })
            .await
            .unwrap();
        assert_eq!(out.orders.len(), 1);
    }
}
