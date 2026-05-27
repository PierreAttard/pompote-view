//! HTTP inbound adapter (axum 0.8).

pub mod api_key;
pub mod candles;
pub mod handlers;
pub mod router;
pub mod state;

pub use router::build_router;
pub use state::AppState;
