//! `StrategyRepository` adapter — read-only `SELECT`s on `strategies` and the
//! live `fills` table.
//!
//! The strategy listing is a small selector (`ORDER BY name`, no window). The
//! fills query is filtered by `strategy_id` and bounded by a `[from, to)`
//! window on **`executed_at`** (live wall clock), ordered ascending so the
//! front-end plots execution markers chronologically.
//!
//! `fills.side` keeps its `CHECK (buy|sell)` and is re-parsed against
//! [`OrderSide`] at the boundary (defence-in-depth: a parse failure signals
//! schema drift → `500`).

use application::ports::{RepositoryError, StrategyRepository, StrategyWindowQuery};
use async_trait::async_trait;
use domain::decision::LiveDecision;
use domain::fill::LiveFill;
use domain::order::{InvalidOrderSide, OrderSide};
use domain::strategy::Strategy;
use rust_decimal::Decimal;
use sqlx::PgPool;

/// `StrategyRepository` implementation backed by a Postgres connection pool.
#[derive(Clone)]
pub struct SqlxStrategyRepository {
    pool: PgPool,
}

impl SqlxStrategyRepository {
    /// Wraps a Postgres pool in a `StrategyRepository`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StrategyRepository for SqlxStrategyRepository {
    async fn list_strategies(&self) -> Result<Vec<Strategy>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id            AS "id!: uuid::Uuid",
                name          AS "name!",
                kind          AS "kind!",
                enabled_paper AS "enabled_paper!",
                enabled_live  AS "enabled_live!"
            FROM strategies
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|row| Strategy {
                id: row.id,
                name: row.name,
                kind: row.kind,
                enabled_paper: row.enabled_paper,
                enabled_live: row.enabled_live,
            })
            .collect())
    }

    async fn fetch_fills_for_strategy(
        &self,
        query: &StrategyWindowQuery,
    ) -> Result<Vec<LiveFill>, RepositoryError> {
        let limit = i64::try_from(query.limit).map_err(|_| {
            RepositoryError::Internal(format!(
                "limit `{}` does not fit in i64 (should have been capped by the use case)",
                query.limit
            ))
        })?;
        let rows = sqlx::query!(
            r#"
            SELECT
                id          AS "id!: uuid::Uuid",
                order_id    AS "order_id!: uuid::Uuid",
                side        AS "side!",
                price       AS "price!: Decimal",
                quantity    AS "quantity!: Decimal",
                fee         AS "fee!: Decimal",
                fee_asset   AS "fee_asset!",
                executed_at AS "executed_at!: chrono::DateTime<chrono::Utc>",
                is_paper    AS "is_paper!"
            FROM fills
            WHERE strategy_id = $1
              AND executed_at >= $2
              AND executed_at < $3
            ORDER BY executed_at ASC
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
                Ok(LiveFill {
                    fill_id: row.id,
                    order_id: row.order_id,
                    side: parse_side(&row.side)?,
                    price: row.price,
                    quantity: row.quantity,
                    fee: row.fee,
                    fee_asset: row.fee_asset,
                    executed_at: row.executed_at,
                    is_paper: row.is_paper,
                })
            })
            .collect()
    }

    async fn fetch_decisions_for_strategy(
        &self,
        query: &StrategyWindowQuery,
    ) -> Result<Vec<LiveDecision>, RepositoryError> {
        let limit = i64::try_from(query.limit).map_err(|_| {
            RepositoryError::Internal(format!(
                "limit `{}` does not fit in i64 (should have been capped by the use case)",
                query.limit
            ))
        })?;
        // `decision_market_context` is keyed `(time, decision_id)`, so a decision
        // could in principle have several context rows. A LEFT JOIN LATERAL
        // picking the most recent keeps exactly one row per decision (no fan-out
        // that would inflate the result past the cap); `snapshot` is `NULL` when
        // no context exists.
        let rows = sqlx::query!(
            r#"
            SELECT
                d.id           AS "id!: uuid::Uuid",
                d.session_id   AS "session_id!: uuid::Uuid",
                d.created_at   AS "created_at!: chrono::DateTime<chrono::Utc>",
                d.reason       AS "reason!",
                d.orders_count AS "orders_count!",
                c.snapshot     AS "snapshot?: serde_json::Value"
            FROM strategy_decisions d
            LEFT JOIN LATERAL (
                SELECT snapshot
                FROM decision_market_context c
                WHERE c.decision_id = d.id
                ORDER BY c.time DESC
                LIMIT 1
            ) c ON true
            WHERE d.strategy_id = $1
              AND d.created_at >= $2
              AND d.created_at < $3
            ORDER BY d.created_at ASC
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

        Ok(rows
            .into_iter()
            .map(|row| LiveDecision {
                decision_id: row.id,
                session_id: row.session_id,
                created_at: row.created_at,
                reason: row.reason,
                orders_count: row.orders_count,
                market_context: row.snapshot,
            })
            .collect())
    }
}

/// Parses a DB-sourced side string into the domain whitelist.
fn parse_side(raw: &str) -> Result<OrderSide, RepositoryError> {
    OrderSide::try_from(raw).map_err(|InvalidOrderSide { input }| {
        RepositoryError::Internal(format!(
            "unexpected `fills.side` value `{input}` (schema drift?)"
        ))
    })
}

/// Maps an `sqlx::Error` to a [`RepositoryError`] (same policy as the other
/// adapters): transport failures become `Unavailable` (`503`), everything else
/// becomes `Internal` (`500`).
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
    fn parse_side_round_trips() {
        assert_eq!(parse_side("buy").unwrap(), OrderSide::Buy);
        assert_eq!(parse_side("sell").unwrap(), OrderSide::Sell);
    }

    #[test]
    fn parse_side_reports_drift_as_internal() {
        let err = parse_side("hodl").unwrap_err();
        assert!(matches!(err, RepositoryError::Internal(_)));
        assert!(err.to_string().contains("schema drift"));
    }
}
