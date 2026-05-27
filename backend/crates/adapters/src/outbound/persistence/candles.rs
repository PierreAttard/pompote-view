//! `CandleRepository` adapter — `time_bucket()` aggregation over `candles_5s`.
//!
//! Issues a single read-only `SELECT` against the TimescaleDB hypertable. The
//! aggregation contract:
//!
//! - `open`  = `first(open, open_time)` (earliest 5s row in the bucket)
//! - `high`  = `max(high)`
//! - `low`   = `min(low)`
//! - `close` = `last(close, open_time)` (latest 5s row in the bucket)
//! - `volume` = `sum(volume)`
//!
//! All NUMERIC columns are mapped to [`rust_decimal::Decimal`] — no `f64`
//! conversion at the I/O boundary. The HTTP DTO layer is where we down-cast
//! to `f64` (with the caveat documented at the call site).
//!
//! The SQL `LIMIT` is set to `MAX_CANDLE_POINTS + 1` as a defence-in-depth
//! safety net: the use case already rejects oversized queries before reaching
//! this adapter, so under normal operation we expect at most
//! `MAX_CANDLE_POINTS` rows.

use application::ports::{CandleQuery, CandleRepository, RepositoryError};
use async_trait::async_trait;
use domain::candle::{Candle, MAX_CANDLE_POINTS};
use rust_decimal::Decimal;
use sqlx::PgPool;

/// `LIMIT` value applied to the SQL query.
///
/// `MAX_CANDLE_POINTS + 1` so we could, in future work, observe that the cap
/// saturated even though the use-case validation rejects the request first.
const SQL_ROW_LIMIT: i64 = MAX_CANDLE_POINTS as i64 + 1;

/// `CandleRepository` implementation backed by a Postgres connection pool.
#[derive(Clone)]
pub struct SqlxCandleRepository {
    pool: PgPool,
}

impl SqlxCandleRepository {
    /// Wraps a Postgres pool in a `CandleRepository`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CandleRepository for SqlxCandleRepository {
    async fn fetch_aggregated(&self, query: &CandleQuery) -> Result<Vec<Candle>, RepositoryError> {
        // The bucket width is bound as text and parsed by Postgres into
        // `INTERVAL` server-side (`$1::interval`). This keeps the query
        // prepare-once-and-reuse friendly: only six parameters vary across
        // calls — `interval`, `exchange`, `symbol`, `from`, `to`, `limit`.
        //
        // `first()` / `last()` are Timescale extension aggregates; sqlx
        // cannot statically infer their nullability, so we annotate each
        // projection column with `name!: Decimal` to force a non-NULL
        // mapping (Timescale only emits a row when at least one input row
        // exists in the bucket, so the aggregates cannot be NULL here).
        let interval = query.timeframe.to_pg_interval();
        // We pass the interval literal as TEXT and parse it server-side with
        // `$1::text::interval`. Two reasons:
        //   1. sqlx's `query!` macro otherwise infers `PgInterval` for
        //      `$1::interval`, forcing us to allocate a complex Postgres
        //      type for every call.
        //   2. The interval value comes from our own `Timeframe` whitelist —
        //      not from user input — so there is no SQL injection surface.
        //
        // We also repeat the `time_bucket(...)` expression in `GROUP BY` /
        // `ORDER BY` rather than referencing the `bucket` alias because
        // sqlx's `DESCRIBE` codepath does not resolve aliases there (it
        // rejects the query with "column does not exist" even though
        // Postgres accepts it at runtime).
        let rows = sqlx::query!(
            r#"
            SELECT
                time_bucket($1::text::interval, open_time) AS "bucket!: chrono::DateTime<chrono::Utc>",
                first(open, open_time)                     AS "open!: Decimal",
                max(high)                                  AS "high!: Decimal",
                min(low)                                   AS "low!: Decimal",
                last(close, open_time)                     AS "close!: Decimal",
                sum(volume)                                AS "volume!: Decimal"
            FROM candles_5s
            WHERE exchange = $2
              AND symbol = $3
              AND open_time >= $4
              AND open_time < $5
            GROUP BY time_bucket($1::text::interval, open_time)
            ORDER BY time_bucket($1::text::interval, open_time) ASC
            LIMIT $6
            "#,
            interval,
            query.exchange,
            query.symbol,
            query.from,
            query.to,
            SQL_ROW_LIMIT,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|row| Candle {
                open_time: row.bucket,
                open: row.open,
                high: row.high,
                low: row.low,
                close: row.close,
                volume: row.volume,
            })
            .collect())
    }
}

/// Maps an `sqlx::Error` to a [`RepositoryError`].
///
/// We deliberately collapse transport-layer failures (PoolTimedOut, Io,
/// Tls…) into `Unavailable` so the HTTP layer can return `503`. Anything
/// else (Database errors, schema mismatch decoded as Decode/ColumnNotFound)
/// becomes `Internal` and surfaces as `500` — these indicate a bug or
/// schema drift in `robot_rust`, not a transient outage.
fn map_sqlx_error(err: sqlx::Error) -> RepositoryError {
    use sqlx::Error::*;
    match err {
        PoolClosed | PoolTimedOut | Io(_) | Tls(_) | WorkerCrashed => {
            RepositoryError::Unavailable(err.to_string())
        }
        _ => RepositoryError::Internal(err.to_string()),
    }
}
