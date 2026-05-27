//! OpenAPI document definition (utoipa).
//!
//! [`ApiDoc`] aggregates every annotated path and component schema so that
//! issue #13 can wire `utoipa-swagger-ui` and serve `/openapi.json` without
//! re-discovering them. The struct is intentionally not mounted on the
//! router here: this PR (issue #10) only lays the foundation.
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

use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};

use super::{candles, orders};

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
    ),
    components(
        schemas(
            candles::CandleDto,
            candles::CandleErrorBody,
            orders::OrderDto,
            orders::OrderErrorBody,
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
        assert!(json.contains("CandleDto"));
        assert!(json.contains("OrderDto"));
        assert!(json.contains("x_api_key"));
    }
}
