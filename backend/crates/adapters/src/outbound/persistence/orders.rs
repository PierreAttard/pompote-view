//! `OrderRepository` adapter — bounded `SELECT` against the `orders` table.
//!
//! Issues a single read-only `SELECT` filtered by `strategy_id` and
//! `created_at`, ordered ASC by `created_at` (the front-end plots markers
//! in chronological order, so ordering at the DB layer avoids a second
//! sort in JS).
//!
//! `side` and `status` arrive from the DB as `TEXT` and are re-parsed
//! against the domain whitelists at the adapter boundary. The DB already
//! enforces these via `CHECK` constraints, so a parse failure here is a
//! defence-in-depth signal of schema drift (mapped to
//! [`RepositoryError::Internal`] → `500`).
//!
//! `price` is derived as `COALESCE(limit_price, expected_price)`:
//! - non-`NULL` for `limit` / `amend` orders (explicit limit price)
//! - non-`NULL` for `market` orders when the strategy engine recorded an
//!   `expected_price` just before submission
//! - `NULL` otherwise (forwarded as a JSON `null` by the DTO).
//!
//! The SQL `LIMIT $4` already comes capped by the use case at
//! [`MAX_ORDER_ROWS`]; the cast `usize -> i64` is therefore safe.

use application::ports::{OrderQuery, OrderRepository, RepositoryError};
use async_trait::async_trait;
use domain::order::{InvalidOrderSide, InvalidOrderStatus, Order, OrderSide, OrderStatus};
use rust_decimal::Decimal;
use sqlx::PgPool;

/// `OrderRepository` implementation backed by a Postgres connection pool.
#[derive(Clone)]
pub struct SqlxOrderRepository {
    pool: PgPool,
}

impl SqlxOrderRepository {
    /// Wraps a Postgres pool in an `OrderRepository`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderRepository for SqlxOrderRepository {
    async fn fetch_orders_for_strategy(
        &self,
        query: &OrderQuery,
    ) -> Result<Vec<Order>, RepositoryError> {
        // The use case has already validated `limit <= MAX_ORDER_ROWS`, so the
        // `i64` cast is lossless on any realistic target. We still go through
        // `try_from` to keep the conversion explicit.
        let limit: i64 = i64::try_from(query.limit).map_err(|_| {
            RepositoryError::Internal(format!(
                "limit `{}` does not fit in i64 (should have been caught by the use case)",
                query.limit
            ))
        })?;

        // `id`, `decision_id`, `quantity`, `side`, `status`, `created_at` are
        // all `NOT NULL` in the schema, so we annotate them with `!`. Only
        // `price` is nullable (COALESCE of two nullable columns) — `?` tells
        // sqlx to map it to `Option<Decimal>`.
        let rows = sqlx::query!(
            r#"
            SELECT
                id           AS "id!: uuid::Uuid",
                decision_id  AS "decision_id!: uuid::Uuid",
                side         AS "side!: String",
                COALESCE(limit_price, expected_price) AS "price?: Decimal",
                quantity     AS "quantity!: Decimal",
                status       AS "status!: String",
                created_at   AS "created_at!: chrono::DateTime<chrono::Utc>"
            FROM orders
            WHERE strategy_id = $1
              AND created_at >= $2
              AND created_at < $3
            ORDER BY created_at ASC
            LIMIT $4
            "#,
            query.strategy_id,
            query.from,
            query.to,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(Order {
                    order_id: row.id,
                    decision_id: row.decision_id,
                    side: parse_side(&row.side)?,
                    price: row.price,
                    quantity: row.quantity,
                    status: parse_status(&row.status)?,
                    created_at: row.created_at,
                })
            })
            .collect()
    }
}

/// Parses a DB-sourced side string into the domain whitelist.
///
/// Exposed at module scope so the parse logic stays unit-testable without
/// spinning up a Postgres instance.
fn parse_side(raw: &str) -> Result<OrderSide, RepositoryError> {
    OrderSide::try_from(raw).map_err(|InvalidOrderSide { input }| {
        RepositoryError::Internal(format!(
            "unexpected `orders.side` value `{input}` (schema drift?)"
        ))
    })
}

/// Parses a DB-sourced status string into the domain whitelist.
fn parse_status(raw: &str) -> Result<OrderStatus, RepositoryError> {
    OrderStatus::try_from(raw).map_err(|InvalidOrderStatus { input }| {
        RepositoryError::Internal(format!(
            "unexpected `orders.status` value `{input}` (schema drift?)"
        ))
    })
}

/// Maps an `sqlx::Error` to a [`RepositoryError`].
///
/// Same policy as the candles adapter: transport-layer failures (PoolTimedOut,
/// Io, Tls…) collapse to `Unavailable` (`503`); anything else (`Database`,
/// schema mismatch decoded as `Decode` / `ColumnNotFound`) becomes
/// `Internal` (`500`).
fn map_sqlx_error(err: sqlx::Error) -> RepositoryError {
    use sqlx::Error::*;
    match err {
        PoolClosed | PoolTimedOut | Io(_) | Tls(_) | WorkerCrashed => {
            RepositoryError::Unavailable(err.to_string())
        }
        _ => RepositoryError::Internal(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_side_round_trips_for_allowed_values() {
        assert_eq!(parse_side("buy").unwrap(), OrderSide::Buy);
        assert_eq!(parse_side("sell").unwrap(), OrderSide::Sell);
    }

    #[test]
    fn parse_side_reports_schema_drift_as_internal() {
        let err = parse_side("hodl").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("hodl"));
        assert!(msg.contains("schema drift"));
    }

    #[test]
    fn parse_status_round_trips_for_all_seven_values() {
        for raw in OrderStatus::ALLOWED {
            let parsed = parse_status(raw).expect("allowed literal must parse");
            assert_eq!(parsed.as_str(), *raw);
        }
    }

    #[test]
    fn parse_status_reports_schema_drift_as_internal() {
        let err = parse_status("partial").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("partial"));
        assert!(msg.contains("schema drift"));
    }
}
