//! Shared application state injected into axum handlers via `State`.

use std::sync::Arc;

use application::use_cases::{GetCandles, GetOrders, ReadinessProbe};

/// Immutable runtime state shared by every HTTP handler.
///
/// Cloning is cheap because every field is an `Arc`. `AppState` is wired by
/// `bootstrap` at startup and passed to `Router::with_state`.
#[derive(Clone)]
pub struct AppState {
    /// Readiness probe use case, used by `GET /readyz`.
    pub readiness: Arc<ReadinessProbe>,
    /// Expected `X-API-Key` value for the `/api/v1/*` middleware.
    ///
    /// Stored as raw bytes to allow constant-time comparison via `subtle`.
    pub api_key: Arc<Vec<u8>>,
    /// `GET /api/v1/monitoring/candles` use case.
    pub get_candles: Arc<GetCandles>,
    /// `GET /api/v1/monitoring/strategies/:id/orders` use case.
    pub get_orders: Arc<GetOrders>,
}
