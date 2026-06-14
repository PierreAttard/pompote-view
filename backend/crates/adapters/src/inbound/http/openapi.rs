//! OpenAPI document definition (utoipa) and serving routes.
//!
//! [`ApiDoc`] aggregates every annotated path and component schema.
//! [`openapi_router`] mounts the spec at `/api/openapi.json` (always) and,
//! optionally, Swagger UI at `/swagger-ui`. [`openapi_pretty_json`] serialises
//! the same document for the `dump_openapi` binary (frontend codegen + CI).
//!
//! When you add a new endpoint:
//!
//! 1. Annotate the handler with `#[utoipa::path(...)]`.
//! 2. Reference the function path in [`ApiDoc::paths`] below.
//! 3. Add any new DTO / error body to [`ApiDoc::components(schemas(...))`].
//!
//! The `x_api_key` security scheme name matches the [`security(...)`]
//! reference on every protected handler; the `X-API-Key` header is read by
//! the [`super::api_key::require_api_key`] middleware.

use axum::{Json, Router, routing::get};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

use super::{backtests, candles, orders, strategies};

/// Top-level OpenAPI document for the viz backend.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "pompote-view API",
        description = "Read-only HTTP surface for monitoring trading strategies. \
                       Every `/api/v1/*` route requires the `X-API-Key` header.",
        version = "0.1.0",
        license(name = "Apache-2.0"),
    ),
    paths(
        candles::get_candles,
        orders::get_orders,
        backtests::list_backtests,
        backtests::get_backtest,
        backtests::get_backtest_orders,
        backtests::get_backtest_fills,
        backtests::get_backtest_decisions,
        backtests::get_backtest_candles,
        backtests::get_backtest_metrics,
        strategies::get_timeframes,
        strategies::list_strategies,
        strategies::get_strategy_fills,
    ),
    components(
        schemas(
            candles::CandleDto,
            candles::CandleErrorBody,
            orders::OrderDto,
            orders::OrderErrorBody,
            strategies::StrategyDto,
            strategies::LiveFillDto,
            strategies::StrategyErrorBody,
            backtests::BacktestRunSummaryDto,
            backtests::BacktestRunDetailDto,
            backtests::BacktestOrderDto,
            backtests::FillDto,
            backtests::DecisionDto,
            backtests::BacktestCandlesDto,
            backtests::BacktestMetricsDto,
            backtests::BacktestErrorBody,
        )
    ),
    tags(
        (name = "monitoring", description = "Read-only monitoring endpoints (candles, orders, …).")
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Adds the `x_api_key` security scheme to the generated document.
///
/// Declared as a `Modify` impl rather than inline because utoipa does not
/// (yet) accept the `security_schemes(...)` shorthand in `#[openapi]`.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .as_mut()
            .expect("components registry should be initialised by utoipa");
        components.add_security_scheme(
            "x_api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
        );
    }
}

/// Builds the OpenAPI-serving routes, merged into the app by `bootstrap`.
///
/// `GET /api/openapi.json` is always served: the spec is the public shape of
/// the API (no secrets), and the frontend codegen + external tools consume it.
/// When `enable_swagger_ui` is `true`, the interactive Swagger UI is also
/// mounted at `/swagger-ui` (convenient in dev, disabled in production via
/// `VIZ_ENABLE_SWAGGER_UI=false`). Neither route sits behind the `X-API-Key`
/// middleware (that wraps only `/api/v1/monitoring`).
pub fn openapi_router(enable_swagger_ui: bool) -> Router {
    if enable_swagger_ui {
        // `SwaggerUi` serves both the UI and the spec at the given URL.
        Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()))
    } else {
        // UI disabled: still expose the machine-readable spec on its own route.
        Router::new().route("/api/openapi.json", get(serve_openapi))
    }
}

/// Handler returning the OpenAPI document as JSON.
async fn serve_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Serialises the OpenAPI document as pretty JSON.
///
/// Used by the `dump_openapi` binary to feed the frontend TypeScript codegen
/// without needing a running server.
pub fn openapi_pretty_json() -> Result<String, serde_json::Error> {
    ApiDoc::openapi().to_pretty_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: the document must serialise to JSON and expose the two
    /// monitoring paths. We don't snapshot the full doc here (utoipa internals
    /// shift between minor versions); presence of the routes is enough.
    #[test]
    fn document_serialises_with_known_paths_and_schemas() {
        let doc = ApiDoc::openapi();
        let json = serde_json::to_string(&doc).expect("OpenAPI doc must serialise");
        assert!(json.contains("/api/v1/monitoring/candles"));
        assert!(json.contains("/api/v1/monitoring/strategies/{id}/orders"));
        assert!(json.contains("/api/v1/monitoring/backtests"));
        assert!(json.contains("/api/v1/monitoring/backtests/{run_id}/decisions"));
        assert!(json.contains("CandleDto"));
        assert!(json.contains("OrderDto"));
        assert!(json.contains("BacktestRunDetailDto"));
        assert!(json.contains("x_api_key"));
    }

    #[test]
    fn pretty_json_is_valid_and_contains_paths() {
        let json = openapi_pretty_json().expect("spec must serialise");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("pretty JSON must parse");
        assert_eq!(
            parsed["openapi"].as_str().unwrap().chars().next(),
            Some('3')
        );
        assert!(parsed["paths"]["/api/v1/monitoring/backtests"].is_object());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn openapi_json_route_served_when_ui_disabled() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = openapi_router(false);
        let resp = app
            .oneshot(
                Request::get("/api/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(parsed["paths"]["/api/v1/monitoring/candles"].is_object());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn openapi_json_route_served_when_ui_enabled() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        // The Swagger-UI branch also serves the spec at the same URL; this
        // pins the absence of a route conflict at construction.
        let app = openapi_router(true);
        let resp = app
            .oneshot(
                Request::get("/api/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
