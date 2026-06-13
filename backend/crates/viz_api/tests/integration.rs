//! DB-backed integration tests for the viz backend (issue #12).
//!
//! Compiled only with `--features integration` (off in the default offline CI).
//! The harness:
//!
//! 1. spins a disposable TimescaleDB container (testcontainers),
//! 2. applies the real `robot_rust` migrations from `ROBOT_RUST_MIGRATIONS_DIR`
//!    (a checkout of `crates/robot_rust/migrations`),
//! 3. seeds representative fixtures,
//! 4. boots the real axum app and exercises every endpoint over HTTP.
//!
//! Because the viz backend is **read-only**, every check only issues `SELECT`s.
//! The whole suite therefore runs as a single `#[tokio::test]` (`endpoint_suite`)
//! that sets up the container + server once and exercises every endpoint
//! sequentially — a single test owns one runtime, which keeps the spawned
//! server alive for the whole run (a server spawned in one `#[tokio::test]`
//! dies when that test's runtime is dropped, hence not one test per endpoint).
//!
//! ## Schema-drift detection
//!
//! This harness is also the drift detector between `robot_rust`'s migrations
//! and the viz backend's SQL: the queries are checked at compile time against
//! the committed `.sqlx` cache, but here they run against the schema produced
//! by the *current* migrations. A renamed/removed column makes a query fail at
//! runtime and turns the test red.
//!
//! Run locally:
//! ```bash
//! ROBOT_RUST_MIGRATIONS_DIR=/path/to/robot_rust/crates/robot_rust/migrations \
//!   cargo test -p viz_api --features integration
//! ```
#![cfg(feature = "integration")]

use std::sync::Arc;
use std::time::Duration;

use adapters::inbound::http::{AppState, build_router, openapi_router};
use adapters::outbound::clock::SystemClock;
use adapters::outbound::persistence::{
    SqlxBacktestRepository, SqlxCandleRepository, SqlxHealthChecker, SqlxOrderRepository,
};
use application::use_cases::{
    GetBacktestRun, GetBacktestRunCandles, GetBacktestSeries, GetCandles, GetOrders,
    ListBacktestRuns, ReadinessProbe,
};
use axum::Router;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers::core::WaitFor;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio::net::TcpListener;

const API_KEY: &str = "integration-test-key-0123456789";
const RUN_ID: &str = "11111111-1111-1111-1111-111111111111";

/// Owned test context: the container (kept alive for the whole test) and a
/// client pointed at the running server.
struct TestCtx {
    _container: ContainerAsync<GenericImage>,
    base_url: String,
    client: reqwest::Client,
}

async fn init_ctx() -> TestCtx {
    let container = GenericImage::new("timescale/timescaledb", "latest-pg16")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "viz")
        .start()
        .await
        .expect("start timescaledb container");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("mapped pg port");
    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/viz");

    let pool = connect_with_retry(&db_url).await;
    apply_migrations(&pool).await;
    seed(&pool).await;

    let app = build_app(pool);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    TestCtx {
        _container: container,
        base_url: format!("http://{addr}"),
        client: reqwest::Client::new(),
    }
}

/// Connects to the freshly-started container, retrying while it finishes its
/// startup (the postgres image logs "ready" once during init before a restart).
async fn connect_with_retry(db_url: &str) -> PgPool {
    for _ in 0..60 {
        if let Ok(pool) = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(2))
            .connect(db_url)
            .await
        {
            if sqlx::query("SELECT 1").execute(&pool).await.is_ok() {
                return pool;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("postgres container never became reachable at {db_url}");
}

/// Applies every `*.up.sql` migration (lexical order) from the directory named
/// by `ROBOT_RUST_MIGRATIONS_DIR`.
async fn apply_migrations(pool: &PgPool) {
    let dir = std::env::var("ROBOT_RUST_MIGRATIONS_DIR").expect(
        "ROBOT_RUST_MIGRATIONS_DIR must point at a checkout of \
         robot_rust/crates/robot_rust/migrations",
    );
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read migrations dir {dir}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".up.sql"))
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no *.up.sql migrations found in {dir}");
    for path in files {
        let sql = std::fs::read_to_string(&path).expect("read migration");
        sqlx::raw_sql(&sql)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("apply migration {}: {e}", path.display()));
    }
}

/// Inserts representative fixtures used by the assertions below.
async fn seed(pool: &PgPool) {
    // Candle background for binance/BTCUSDT across the run window (3 hourly
    // buckets once aggregated by `time_bucket('1 hour', open_time)`). Literal
    // SQL avoids pulling chrono/rust_decimal into the test crate.
    for (open, hour) in [(100, 0), (101, 1), (102, 2)] {
        sqlx::query(&format!(
            "INSERT INTO candles_5s (open_time, close_time, exchange, symbol, open, high, low, \
             close, volume, buy_volume, sell_volume, trade_count) VALUES \
             (TIMESTAMPTZ '2026-06-01T0{hour}:00:00Z', \
              TIMESTAMPTZ '2026-06-01T0{hour}:00:05Z', 'binance', 'BTCUSDT', \
              {open}, {open}, {open}, {open}, 1, 1, 0, 1)"
        ))
        .execute(pool)
        .await
        .expect("seed candle");
    }

    sqlx::query(
        "INSERT INTO backtest_runs (id, strategy_kind, exchange, symbol, data_range_start, \
         data_range_end, status, slippage_bps, latency_ms, fee_kraken_bps, fee_binance_bps, \
         config_snapshot) VALUES ($1::uuid, 'directional', 'binance', 'BTCUSDT', \
         '2026-06-01T00:00:00Z', '2026-06-01T03:00:00Z', 'completed', 5, 200, 16, 10, \
         '{\"scenario_name\": \"it\"}'::jsonb)",
    )
    .bind(RUN_ID)
    .execute(pool)
    .await
    .expect("seed run");

    sqlx::query(
        "INSERT INTO strategy_decisions_backtest (id, run_id, session_id, strategy_id, \
         strategy_kind, orders_count, reason, market_ts) VALUES \
         ('22222222-2222-2222-2222-222222222222'::uuid, $1::uuid, \
         '33333333-3333-3333-3333-333333333333'::uuid, \
         '44444444-4444-4444-4444-444444444444'::uuid, 'directional', 1, 'rsi_oversold', \
         '2026-06-01T01:30:00Z')",
    )
    .bind(RUN_ID)
    .execute(pool)
    .await
    .expect("seed decision");

    sqlx::query(
        "INSERT INTO orders_backtest (id, run_id, decision_id, session_id, strategy_id, \
         strategy_kind, market_ts, exchange, symbol, side, quantity, kind, status) VALUES \
         ('55555555-5555-5555-5555-555555555555'::uuid, $1::uuid, \
         '22222222-2222-2222-2222-222222222222'::uuid, \
         '33333333-3333-3333-3333-333333333333'::uuid, \
         '44444444-4444-4444-4444-444444444444'::uuid, 'directional', \
         '2026-06-01T01:30:00Z', 'binance', 'BTCUSDT', 'buy', 0.5, 'market', 'filled')",
    )
    .bind(RUN_ID)
    .execute(pool)
    .await
    .expect("seed order");
}

/// Wires the real composition root over the test pool (mirrors `main.rs`).
fn build_app(pool: PgPool) -> Router {
    let clock = Arc::new(SystemClock);
    let readiness = Arc::new(ReadinessProbe::new(Arc::new(SqlxHealthChecker::new(
        pool.clone(),
    ))));
    let candle_repo = Arc::new(SqlxCandleRepository::new(pool.clone()));
    let order_repo = Arc::new(SqlxOrderRepository::new(pool.clone()));
    let backtest_repo = Arc::new(SqlxBacktestRepository::new(pool));
    let get_candles = Arc::new(GetCandles::new(candle_repo, clock.clone()));
    let state = AppState {
        readiness,
        api_key: Arc::new(API_KEY.as_bytes().to_vec()),
        get_candles: get_candles.clone(),
        get_orders: Arc::new(GetOrders::new(order_repo, clock.clone())),
        list_backtest_runs: Arc::new(ListBacktestRuns::new(backtest_repo.clone())),
        get_backtest_run: Arc::new(GetBacktestRun::new(backtest_repo.clone())),
        get_backtest_series: Arc::new(GetBacktestSeries::new(backtest_repo.clone(), clock)),
        get_backtest_candles: Arc::new(GetBacktestRunCandles::new(backtest_repo, get_candles)),
    };
    build_router(state).merge(openapi_router(false))
}

// --- HTTP helpers ----------------------------------------------------------

async fn get(ctx: &TestCtx, path: &str) -> reqwest::Response {
    ctx.client
        .get(format!("{}{path}", ctx.base_url))
        .header("X-API-Key", API_KEY)
        .send()
        .await
        .expect("request")
}

async fn get_no_key(ctx: &TestCtx, path: &str) -> reqwest::Response {
    ctx.client
        .get(format!("{}{path}", ctx.base_url))
        .send()
        .await
        .expect("request")
}

/// Single end-to-end test: the disposable DB + running server are created once
/// (a `#[tokio::test]` owns one runtime, so a single test keeps the spawned
/// server alive throughout), then every endpoint is exercised sequentially.
/// All assertions are read-only, so sharing one seeded DB is safe.
#[tokio::test]
async fn endpoint_suite() {
    let ctx = init_ctx().await;

    health_and_auth(&ctx).await;
    openapi_spec(&ctx).await;
    candles_endpoint(&ctx).await;
    backtest_endpoints(&ctx).await;
}

async fn health_and_auth(ctx: &TestCtx) {
    assert_eq!(get_no_key(ctx, "/healthz").await.status(), 200, "healthz");
    assert_eq!(get(ctx, "/readyz").await.status(), 200, "readyz");

    // Auth middleware on /api/v1/monitoring.
    assert_eq!(
        get_no_key(ctx, "/api/v1/monitoring/backtests")
            .await
            .status(),
        401,
        "missing key"
    );
    let wrong = ctx
        .client
        .get(format!("{}/api/v1/monitoring/backtests", ctx.base_url))
        .header("X-API-Key", "wrong")
        .send()
        .await
        .expect("request");
    assert_eq!(wrong.status(), 401, "wrong key");
    assert_eq!(
        get(ctx, "/api/v1/monitoring/backtests").await.status(),
        200,
        "valid key"
    );
}

async fn openapi_spec(ctx: &TestCtx) {
    let resp = get_no_key(ctx, "/api/openapi.json").await;
    assert_eq!(resp.status(), 200, "openapi.json");
    let spec: serde_json::Value = resp.json().await.unwrap();
    assert!(spec["paths"]["/api/v1/monitoring/backtests"].is_object());
}

async fn candles_endpoint(ctx: &TestCtx) {
    // Golden path: 3 hourly buckets from the seeded 5s candles.
    let resp = get(
        ctx,
        "/api/v1/monitoring/candles?exchange=binance&symbol=BTCUSDT&timeframe=1h\
         &from=2026-06-01T00:00:00Z&to=2026-06-01T03:00:00Z",
    )
    .await;
    assert_eq!(resp.status(), 200, "candles golden");
    let body: serde_json::Value = resp.json().await.unwrap();
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 3, "three hourly buckets, got {arr:?}");
    assert!(arr[0]["o"].as_f64().is_some());

    // Invalid timeframe -> 400.
    let resp = get(
        ctx,
        "/api/v1/monitoring/candles?exchange=binance&symbol=BTCUSDT&timeframe=bogus\
         &from=2026-06-01T00:00:00Z&to=2026-06-01T03:00:00Z",
    )
    .await;
    assert_eq!(resp.status(), 400, "invalid timeframe");
    assert_eq!(
        resp.json::<serde_json::Value>().await.unwrap()["error"],
        "invalid_timeframe"
    );

    // Depth cap: 5s buckets over a 1-day window = 17280 > 5000 -> 400.
    let resp = get(
        ctx,
        "/api/v1/monitoring/candles?exchange=binance&symbol=BTCUSDT&timeframe=5s\
         &from=2026-06-01T00:00:00Z&to=2026-06-02T00:00:00Z",
    )
    .await;
    assert_eq!(resp.status(), 400, "depth cap");
    assert_eq!(
        resp.json::<serde_json::Value>().await.unwrap()["error"],
        "too_many_points"
    );

    // Empty window -> empty array.
    let resp = get(
        ctx,
        "/api/v1/monitoring/candles?exchange=binance&symbol=BTCUSDT&timeframe=1h\
         &from=2030-01-01T00:00:00Z&to=2030-01-01T03:00:00Z",
    )
    .await;
    assert_eq!(resp.status(), 200, "empty window");
    assert!(
        resp.json::<serde_json::Value>()
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
}

async fn backtest_endpoints(ctx: &TestCtx) {
    // List contains the seeded run.
    let body: serde_json::Value = get(ctx, "/api/v1/monitoring/backtests")
        .await
        .json()
        .await
        .unwrap();
    let run = body
        .as_array()
        .expect("array")
        .iter()
        .find(|r| r["id"] == RUN_ID)
        .expect("seeded run present");
    assert_eq!(run["status"], "completed");
    assert_eq!(run["symbol"], "BTCUSDT");

    // Detail + config_snapshot.
    let resp = get(ctx, &format!("/api/v1/monitoring/backtests/{RUN_ID}")).await;
    assert_eq!(resp.status(), 200, "detail");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["run"]["status"], "completed");
    assert_eq!(body["config_snapshot"]["scenario_name"], "it");

    // Unknown run -> 404.
    assert_eq!(
        get(
            ctx,
            "/api/v1/monitoring/backtests/99999999-9999-9999-9999-999999999999"
        )
        .await
        .status(),
        404,
        "unknown run"
    );

    // Orders series (market_ts axis).
    let resp = get(
        ctx,
        &format!(
            "/api/v1/monitoring/backtests/{RUN_ID}/orders\
             ?from=2026-06-01T00:00:00Z&to=2026-06-01T03:00:00Z"
        ),
    )
    .await;
    assert_eq!(resp.status(), 200, "orders");
    let body: serde_json::Value = resp.json().await.unwrap();
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["side"], "buy");
    assert_eq!(arr[0]["status"], "filled");
    assert_eq!(arr[0]["market_ts"], "2026-06-01T01:30:00Z");

    // Composed candles + source hint.
    let resp = get(
        ctx,
        &format!("/api/v1/monitoring/backtests/{RUN_ID}/candles?timeframe=1h"),
    )
    .await;
    assert_eq!(resp.status(), 200, "backtest candles");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["source"], "candles_5s");
    assert_eq!(body["candles"].as_array().unwrap().len(), 3);
}
