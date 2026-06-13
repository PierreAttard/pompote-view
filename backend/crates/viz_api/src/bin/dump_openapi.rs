//! `dump_openapi` — prints the OpenAPI document as pretty JSON to stdout.
//!
//! Used by the frontend TypeScript codegen (`npm run codegen`) and the CI
//! freshness check to obtain the spec without starting the server or touching
//! a database. The document is built purely from the `utoipa` annotations.
//!
//! ```bash
//! cargo run -p viz_api --bin dump_openapi > openapi.json
//! ```

fn main() -> anyhow::Result<()> {
    println!("{}", adapters::inbound::http::openapi_pretty_json()?);
    Ok(())
}
