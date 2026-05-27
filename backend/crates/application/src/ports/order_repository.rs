//! `OrderRepository` port — read-only access to the `orders` table.
//!
//! Implemented by `adapters::outbound::persistence::SqlxOrderRepository`,
//! which issues a single bounded `SELECT` filtered by `strategy_id` and
//! `created_at`. Tests substitute an in-memory fake.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::order::Order;
use uuid::Uuid;

use super::RepositoryError;

/// Validated parameters of an orders query.
///
/// Constructed by the `GetOrders` use case after all domain-level validation
/// (`from < to`, `0 < limit <= MAX_ORDER_ROWS`). The repository can trust the
/// contents and issue SQL directly.
#[derive(Debug, Clone)]
pub struct OrderQuery {
    /// Filter on `orders.strategy_id`.
    pub strategy_id: Uuid,
    /// Inclusive lower bound on `created_at`.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound on `created_at`.
    pub to: DateTime<Utc>,
    /// Row cap (already validated `<= MAX_ORDER_ROWS`).
    pub limit: usize,
}

/// Port: read-only access to order rows for a given strategy.
#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// Fetches the order rows matching `query`, ordered by ascending
    /// `created_at`.
    ///
    /// Implementations MUST honour the [`super::super::use_cases::orders::GetOrders`]
    /// invariants and never return more than
    /// [`domain::order::MAX_ORDER_ROWS`] rows (safety net `LIMIT`).
    async fn fetch_orders_for_strategy(
        &self,
        query: &OrderQuery,
    ) -> Result<Vec<Order>, RepositoryError>;
}
