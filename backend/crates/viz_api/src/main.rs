//! `viz_api` — composition root for the read-only visualization backend.
//!
//! This binary owns the wiring between adapters and use cases. It contains
//! no domain logic: it reads the config, builds the sqlx pool, instantiates
//! the `SqlxHealthChecker` adapter, hands it to the `ReadinessProbe` use
//! case, and serves the axum router built by `adapters::inbound::http`.
//!
//! See `backend/README.md` for the environment variables and run commands.

mod config;

use std::sync::Arc;

use adapters::inbound::http::{AppState, build_router};
use adapters::outbound::clock::SystemClock;
use adapters::outbound::persistence::{
    SqlxBacktestRepository, SqlxCandleRepository, SqlxHealthChecker, SqlxOrderRepository,
};
use application::use_cases::{
    GetBacktestRun, GetBacktestSeries, GetCandles, GetOrders, ListBacktestRuns, ReadinessProbe,
};
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;

/// Max connections in the read-only pool.
///
/// The viz backend has a single use case at boot (`SELECT 1` on `/readyz`)
/// and a few `SELECT`s per UI poll cycle. 5 is comfortably above the
/// expected steady-state load while leaving room on the Timescale instance
/// for the producer pipeline owned by `robot_rust`.
const POOL_MAX_CONNECTIONS: u32 = 5;

/// Bound applied to `pool.acquire()` so `/readyz` reports `503` quickly when
/// Postgres is unreachable instead of hanging on the sqlx default (30s).
const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cfg = AppConfig::from_env()?;

    // `connect_lazy` parses the URL (so a malformed DSN still fails fast at
    // boot) but defers the actual TCP/TLS handshake until the first query.
    // This is exactly what we want for `/readyz`: the server can still bind
    // and answer `/healthz` when Postgres is temporarily unreachable, and
    // `/readyz` will surface `503` for the orchestrator.
    // Apply a per-connection `statement_timeout` as an anti-contention
    // guardrail on the shared Timescale primary (see `config.rs`). We use
    // `set_config(..)` rather than `SET statement_timeout = $1` because the
    // `SET` statement does not accept bind parameters. `is_local = false`
    // makes it a session-level setting that survives across the queries run
    // on this pooled connection. Captured by copy (`u64: Copy`) so the
    // `after_connect` closure stays `Fn`.
    let stmt_timeout_ms = cfg.db_statement_timeout_ms;
    let pool = PgPoolOptions::new()
        .max_connections(POOL_MAX_CONNECTIONS)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                sqlx::query("SELECT set_config('statement_timeout', $1, false)")
                    .bind(stmt_timeout_ms.to_string())
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect_lazy(&cfg.database_url)?;
    info!(
        statement_timeout_ms = stmt_timeout_ms,
        "postgres pool initialised (lazy connect)"
    );

    let health_checker = Arc::new(SqlxHealthChecker::new(pool.clone()));
    let readiness = Arc::new(ReadinessProbe::new(health_checker));

    let candle_repo = Arc::new(SqlxCandleRepository::new(pool.clone()));
    let order_repo = Arc::new(SqlxOrderRepository::new(pool.clone()));
    let backtest_repo = Arc::new(SqlxBacktestRepository::new(pool));
    let clock = Arc::new(SystemClock);
    let get_candles = Arc::new(GetCandles::new(candle_repo, clock.clone()));
    let get_orders = Arc::new(GetOrders::new(order_repo, clock.clone()));
    let list_backtest_runs = Arc::new(ListBacktestRuns::new(backtest_repo.clone()));
    let get_backtest_run = Arc::new(GetBacktestRun::new(backtest_repo.clone()));
    let get_backtest_series = Arc::new(GetBacktestSeries::new(backtest_repo, clock));

    let state = AppState {
        readiness,
        api_key: Arc::new(cfg.api_key.into_bytes()),
        get_candles,
        get_orders,
        list_backtest_runs,
        get_backtest_run,
        get_backtest_series,
    };

    let app = build_router(state);

    let listener = TcpListener::bind(&cfg.bind_addr).await?;
    info!(addr = %cfg.bind_addr, "viz_api listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
