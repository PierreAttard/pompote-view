//! `BacktestRepository` adapter — read-only `SELECT`s on `backtest_runs` and
//! its run-scoped mirror tables.
//!
//! All series queries are filtered by `run_id` and bounded by a `[from, to)`
//! window on **`market_ts`** (the replayed market clock, never `created_at`),
//! ordered ascending so the front-end plots markers chronologically without a
//! second JS sort. The row `LIMIT` is bound from the already-validated
//! `query.limit` (the use case rejects oversized requests upstream).
//!
//! Defence-in-depth at the boundary:
//! - `backtest_runs.status` has a 4-value `CHECK`, so it is re-parsed against
//!   [`BacktestStatus`]; an unexpected value is a schema-drift signal mapped to
//!   [`RepositoryError::Internal`] (`500`).
//! - `orders_backtest.side` / `fills_backtest.side` keep their `CHECK
//!   (buy|sell)` and are re-parsed against [`OrderSide`].
//! - `orders_backtest.status` has **no `CHECK`**, so it is forwarded verbatim
//!   as a free `String` (never re-parsed against the live whitelist).
//!
//! `config_snapshot` (`backtest_runs`) and the decision `snapshot`
//! (`decision_market_context_backtest`, `LEFT JOIN`) are opaque `JSONB` blobs
//! carried as `serde_json::Value` and forwarded unchanged.

use application::ports::{
    BacktestRepository, BacktestRunListQuery, BacktestSeriesQuery, RepositoryError,
};
use async_trait::async_trait;
use domain::backtest::{
    BacktestOrder, BacktestRun, BacktestRunDetail, BacktestStatus, FillAggregate,
};
use domain::decision::Decision;
use domain::fill::Fill;
use domain::order::{InvalidOrderSide, OrderSide};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// `BacktestRepository` implementation backed by a Postgres connection pool.
#[derive(Clone)]
pub struct SqlxBacktestRepository {
    pool: PgPool,
}

impl SqlxBacktestRepository {
    /// Wraps a Postgres pool in a `BacktestRepository`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BacktestRepository for SqlxBacktestRepository {
    async fn list_runs(
        &self,
        query: &BacktestRunListQuery,
    ) -> Result<Vec<BacktestRun>, RepositoryError> {
        let limit = to_i64_limit(query.limit)?;
        // Optional filters use the `($n::type IS NULL OR col = $n)` idiom so a
        // single prepared statement serves every filter combination.
        let status = query.status.map(|s| s.as_str());
        let rows = sqlx::query!(
            r#"
            SELECT
                id               AS "id!: Uuid",
                strategy_kind    AS "strategy_kind!",
                strategy_id      AS "strategy_id?: Uuid",
                exchange         AS "exchange!",
                symbol           AS "symbol!",
                data_range_start AS "data_range_start!: chrono::DateTime<chrono::Utc>",
                data_range_end   AS "data_range_end!: chrono::DateTime<chrono::Utc>",
                started_at       AS "started_at!: chrono::DateTime<chrono::Utc>",
                ended_at         AS "ended_at?: chrono::DateTime<chrono::Utc>",
                status           AS "status!",
                slippage_bps     AS "slippage_bps!",
                latency_ms       AS "latency_ms!",
                fee_kraken_bps   AS "fee_kraken_bps!",
                fee_binance_bps  AS "fee_binance_bps!",
                error            AS "error?"
            FROM backtest_runs
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR strategy_kind = $2)
              AND ($3::text IS NULL OR exchange = $3)
              AND ($4::text IS NULL OR symbol = $4)
            ORDER BY started_at DESC
            LIMIT $5
            "#,
            status,
            query.strategy_kind.as_deref(),
            query.exchange.as_deref(),
            query.symbol.as_deref(),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(BacktestRun {
                    id: row.id,
                    strategy_kind: row.strategy_kind,
                    strategy_id: row.strategy_id,
                    exchange: row.exchange,
                    symbol: row.symbol,
                    data_range_start: row.data_range_start,
                    data_range_end: row.data_range_end,
                    started_at: row.started_at,
                    ended_at: row.ended_at,
                    status: parse_status(&row.status)?,
                    slippage_bps: row.slippage_bps,
                    latency_ms: row.latency_ms,
                    fee_kraken_bps: row.fee_kraken_bps,
                    fee_binance_bps: row.fee_binance_bps,
                    error: row.error,
                })
            })
            .collect()
    }

    async fn get_run(&self, run_id: Uuid) -> Result<Option<BacktestRunDetail>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id               AS "id!: Uuid",
                strategy_kind    AS "strategy_kind!",
                strategy_id      AS "strategy_id?: Uuid",
                exchange         AS "exchange!",
                symbol           AS "symbol!",
                data_range_start AS "data_range_start!: chrono::DateTime<chrono::Utc>",
                data_range_end   AS "data_range_end!: chrono::DateTime<chrono::Utc>",
                started_at       AS "started_at!: chrono::DateTime<chrono::Utc>",
                ended_at         AS "ended_at?: chrono::DateTime<chrono::Utc>",
                status           AS "status!",
                slippage_bps     AS "slippage_bps!",
                latency_ms       AS "latency_ms!",
                fee_kraken_bps   AS "fee_kraken_bps!",
                fee_binance_bps  AS "fee_binance_bps!",
                error            AS "error?",
                config_snapshot  AS "config_snapshot!: serde_json::Value"
            FROM backtest_runs
            WHERE id = $1
            "#,
            run_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(|row| {
            Ok(BacktestRunDetail {
                run: BacktestRun {
                    id: row.id,
                    strategy_kind: row.strategy_kind,
                    strategy_id: row.strategy_id,
                    exchange: row.exchange,
                    symbol: row.symbol,
                    data_range_start: row.data_range_start,
                    data_range_end: row.data_range_end,
                    started_at: row.started_at,
                    ended_at: row.ended_at,
                    status: parse_status(&row.status)?,
                    slippage_bps: row.slippage_bps,
                    latency_ms: row.latency_ms,
                    fee_kraken_bps: row.fee_kraken_bps,
                    fee_binance_bps: row.fee_binance_bps,
                    error: row.error,
                },
                config_snapshot: row.config_snapshot,
            })
        })
        .transpose()
    }

    async fn fetch_orders(
        &self,
        query: &BacktestSeriesQuery,
    ) -> Result<Vec<BacktestOrder>, RepositoryError> {
        let limit = to_i64_limit(query.limit)?;
        let rows = sqlx::query!(
            r#"
            SELECT
                id          AS "id!: Uuid",
                decision_id AS "decision_id!: Uuid",
                side        AS "side!",
                COALESCE(limit_price, expected_price) AS "price?: Decimal",
                quantity    AS "quantity!: Decimal",
                status      AS "status!",
                market_ts   AS "market_ts!: chrono::DateTime<chrono::Utc>"
            FROM orders_backtest
            WHERE run_id = $1
              AND market_ts >= $2
              AND market_ts < $3
            ORDER BY market_ts ASC
            LIMIT $4
            "#,
            query.run_id,
            query.from,
            query.to,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(BacktestOrder {
                    order_id: row.id,
                    decision_id: row.decision_id,
                    side: parse_side(&row.side)?,
                    price: row.price,
                    quantity: row.quantity,
                    status: row.status,
                    market_ts: row.market_ts,
                })
            })
            .collect()
    }

    async fn fetch_fills(&self, query: &BacktestSeriesQuery) -> Result<Vec<Fill>, RepositoryError> {
        let limit = to_i64_limit(query.limit)?;
        let rows = sqlx::query!(
            r#"
            SELECT
                id        AS "id!: Uuid",
                order_id  AS "order_id!: Uuid",
                side      AS "side!",
                price     AS "price!: Decimal",
                quantity  AS "quantity!: Decimal",
                fee       AS "fee!: Decimal",
                fee_asset AS "fee_asset!",
                market_ts AS "market_ts!: chrono::DateTime<chrono::Utc>"
            FROM fills_backtest
            WHERE run_id = $1
              AND market_ts >= $2
              AND market_ts < $3
            ORDER BY market_ts ASC
            LIMIT $4
            "#,
            query.run_id,
            query.from,
            query.to,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(Fill {
                    fill_id: row.id,
                    order_id: row.order_id,
                    side: parse_side(&row.side)?,
                    price: row.price,
                    quantity: row.quantity,
                    fee: row.fee,
                    fee_asset: row.fee_asset,
                    market_ts: row.market_ts,
                })
            })
            .collect()
    }

    async fn fetch_decisions(
        &self,
        query: &BacktestSeriesQuery,
    ) -> Result<Vec<Decision>, RepositoryError> {
        let limit = to_i64_limit(query.limit)?;
        // LEFT JOIN the market context (PK `(run_id, decision_id)`, so at most
        // one row per decision); `snapshot` is `NULL` when no context exists.
        let rows = sqlx::query!(
            r#"
            SELECT
                d.id           AS "id!: Uuid",
                d.market_ts    AS "market_ts!: chrono::DateTime<chrono::Utc>",
                d.reason       AS "reason!",
                d.orders_count AS "orders_count!",
                c.snapshot     AS "snapshot?: serde_json::Value"
            FROM strategy_decisions_backtest d
            LEFT JOIN decision_market_context_backtest c
                   ON c.run_id = d.run_id AND c.decision_id = d.id
            WHERE d.run_id = $1
              AND d.market_ts >= $2
              AND d.market_ts < $3
            ORDER BY d.market_ts ASC
            LIMIT $4
            "#,
            query.run_id,
            query.from,
            query.to,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|row| Decision {
                decision_id: row.id,
                market_ts: row.market_ts,
                reason: row.reason,
                orders_count: row.orders_count,
                snapshot: row.snapshot,
            })
            .collect())
    }

    async fn fetch_fill_aggregates(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<FillAggregate>, RepositoryError> {
        // `GROUP BY side` recipe (robot_rust/docs/backtester.md §5). At most two
        // rows (buy/sell); each group is non-empty so the SUMs are non-null.
        let rows = sqlx::query!(
            r#"
            SELECT
                side                   AS "side!",
                count(*)               AS "fills!",
                sum(quantity)          AS "quantity!: Decimal",
                sum(price * quantity)  AS "notional!: Decimal",
                sum(fee)               AS "fees!: Decimal"
            FROM fills_backtest
            WHERE run_id = $1
            GROUP BY side
            "#,
            run_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(FillAggregate {
                    side: parse_side(&row.side)?,
                    fills: u64::try_from(row.fills).map_err(|_| {
                        RepositoryError::Internal(format!(
                            "negative fill count `{}` from aggregate (unexpected)",
                            row.fills
                        ))
                    })?,
                    quantity: row.quantity,
                    notional: row.notional,
                    fees: row.fees,
                })
            })
            .collect()
    }
}

/// Converts a validated `usize` limit to the `i64` sqlx expects.
///
/// The use case already guarantees `limit <= MAX_*`, so this never fails in
/// practice; the explicit conversion keeps the cast lossless and auditable.
fn to_i64_limit(limit: usize) -> Result<i64, RepositoryError> {
    i64::try_from(limit).map_err(|_| {
        RepositoryError::Internal(format!(
            "limit `{limit}` does not fit in i64 (should have been capped by the use case)"
        ))
    })
}

/// Parses a DB-sourced side string into the domain whitelist.
fn parse_side(raw: &str) -> Result<OrderSide, RepositoryError> {
    OrderSide::try_from(raw).map_err(|InvalidOrderSide { input }| {
        RepositoryError::Internal(format!(
            "unexpected backtest `side` value `{input}` (schema drift?)"
        ))
    })
}

/// Parses a DB-sourced `backtest_runs.status` string into the domain whitelist.
fn parse_status(raw: &str) -> Result<BacktestStatus, RepositoryError> {
    BacktestStatus::try_from(raw).map_err(|err| {
        RepositoryError::Internal(format!(
            "unexpected `backtest_runs.status` value `{}` (schema drift?)",
            err.input
        ))
    })
}

/// Maps an `sqlx::Error` to a [`RepositoryError`] (same policy as the candles
/// and orders adapters): transport failures become `Unavailable` (`503`),
/// everything else (incl. schema drift decoded as `Decode`/`ColumnNotFound`)
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
    fn parse_side_round_trips_for_allowed_values() {
        assert_eq!(parse_side("buy").unwrap(), OrderSide::Buy);
        assert_eq!(parse_side("sell").unwrap(), OrderSide::Sell);
    }

    #[test]
    fn parse_side_reports_schema_drift_as_internal() {
        let err = parse_side("hodl").unwrap_err();
        assert!(matches!(err, RepositoryError::Internal(_)));
        assert!(err.to_string().contains("hodl"));
        assert!(err.to_string().contains("schema drift"));
    }

    #[test]
    fn parse_status_round_trips_for_all_allowed_values() {
        for raw in BacktestStatus::ALLOWED {
            assert_eq!(parse_status(raw).unwrap().as_str(), *raw);
        }
    }

    #[test]
    fn parse_status_reports_schema_drift_as_internal() {
        let err = parse_status("paused").unwrap_err();
        assert!(matches!(err, RepositoryError::Internal(_)));
        assert!(err.to_string().contains("paused"));
        assert!(err.to_string().contains("backtest_runs.status"));
    }

    #[test]
    fn to_i64_limit_converts_valid_value() {
        assert_eq!(to_i64_limit(5000).unwrap(), 5000_i64);
    }
}
