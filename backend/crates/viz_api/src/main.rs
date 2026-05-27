//! viz_api — read-only HTTP server for visualizing trading strategy decisions.
//!
//! This is the minimal scaffold introduced by Issue #4. Routing, middleware,
//! DB access and monitoring endpoints land in subsequent issues (#7+).

use axum::{Router, http::StatusCode, routing::get};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

const BIND_ADDR: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz));

    let listener = TcpListener::bind(BIND_ADDR).await?;
    info!(addr = %BIND_ADDR, "viz_api listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

async fn readyz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}
