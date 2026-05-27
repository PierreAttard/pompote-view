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
use adapters::outbound::persistence::{SqlxCandleRepository, SqlxHealthChecker};
use application::use_cases::{GetCandles, ReadinessProbe};
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
    let pool = PgPoolOptions::new()
        .max_connections(POOL_MAX_CONNECTIONS)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .connect_lazy(&cfg.database_url)?;
    info!("postgres pool initialised (lazy connect)");

    let health_checker = Arc::new(SqlxHealthChecker::new(pool.clone()));
    let readiness = Arc::new(ReadinessProbe::new(health_checker));

    let candle_repo = Arc::new(SqlxCandleRepository::new(pool));
    let clock = Arc::new(SystemClock);
    let get_candles = Arc::new(GetCandles::new(candle_repo, clock));

    let state = AppState {
        readiness,
        api_key: Arc::new(cfg.api_key.into_bytes()),
        get_candles,
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
