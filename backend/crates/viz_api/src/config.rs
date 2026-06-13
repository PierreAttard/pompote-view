//! Runtime configuration read from environment variables.
//!
//! Centralising env parsing here keeps `main.rs` linear and makes the failure
//! modes explicit (`ConfigError`). The viz API has no built-in defaults for
//! `DATABASE_URL` or `VIZ_API_KEY` on purpose: a missing value must fail fast
//! at startup rather than silently fall back to a dev value in production.

use std::env::{self, VarError};

use thiserror::Error;

/// Default bind address.
///
/// Port 3100 is used because the more common port 3000 is often already
/// occupied by other services on dev machines.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3100";

/// Minimum length enforced on `VIZ_API_KEY`.
///
/// 16 bytes is a soft floor designed to reject trivially short keys
/// (`""`, `"test"`, `"dev"`). It is not a cryptographic guarantee; rotating
/// to a longer random value in production is still recommended.
pub const MIN_API_KEY_LEN: usize = 16;

/// Default per-connection `statement_timeout`, in milliseconds.
///
/// Acts as an anti-contention guardrail on the shared Timescale primary: a
/// runaway viz `SELECT` (cap-busting scan, missing index on a cold backtest
/// window) is killed by Postgres after this delay instead of competing with
/// the `robot_rust` producer pipeline. 5 s is comfortably above a capped
/// (<= 5000 rows/points) query yet well below a human's patience threshold.
pub const DEFAULT_DB_STATEMENT_TIMEOUT_MS: u64 = 5000;

/// Default for `VIZ_ENABLE_SWAGGER_UI`.
///
/// Swagger UI is enabled by default (convenient in dev). Operators disable it
/// in production by setting `VIZ_ENABLE_SWAGGER_UI=false`. The machine-readable
/// `/api/openapi.json` spec is served regardless of this flag.
pub const DEFAULT_ENABLE_SWAGGER_UI: bool = true;

/// Parsed configuration for the viz API.
///
/// `Debug` is implemented manually to redact sensitive fields (`database_url`
/// contains the Postgres password, `api_key` is the shared HTTP secret). This
/// guards against accidental leaks via `tracing::debug!(?cfg)` or
/// `error!("config: {:?}", cfg)` in future code.
#[derive(Clone)]
pub struct AppConfig {
    /// Postgres connection URL for the read-only `pompote_viz_reader` role.
    pub database_url: String,
    /// Expected value of the `X-API-Key` HTTP header.
    pub api_key: String,
    /// Socket address the axum server will bind to.
    pub bind_addr: String,
    /// Per-connection `statement_timeout` in milliseconds, applied via the
    /// pool's `after_connect` hook (see `main.rs`).
    ///
    /// A value of `0` follows Postgres semantics and **disables** the timeout
    /// entirely. This is a deliberate operator escape hatch (e.g. local
    /// debugging of a slow query); it is distinct from a malformed value,
    /// which is rejected at boot (`ConfigError::InvalidInt`).
    pub db_statement_timeout_ms: u64,
    /// Whether to mount Swagger UI at `/swagger-ui` (disable in production).
    pub enable_swagger_ui: bool,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"<redacted>")
            .field("api_key", &"<redacted>")
            .field("bind_addr", &self.bind_addr)
            .field("db_statement_timeout_ms", &self.db_statement_timeout_ms)
            .field("enable_swagger_ui", &self.enable_swagger_ui)
            .finish()
    }
}

/// Errors returned when reading the configuration from the environment.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A required env variable is missing or empty.
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),

    /// `VIZ_API_KEY` was provided but is shorter than `MIN_API_KEY_LEN`.
    #[error(
        "environment variable `VIZ_API_KEY` is too short ({actual} bytes, \
         minimum {minimum}); pick a longer random value"
    )]
    ApiKeyTooShort { actual: usize, minimum: usize },

    /// `std::env::var` returned a non-UTF-8 / non-Unicode error.
    #[error("environment variable `{var}` is not valid UTF-8")]
    NotUnicode { var: &'static str },

    /// An integer-typed env variable was provided but could not be parsed.
    #[error("environment variable `{var}` is not a valid integer: `{value}`")]
    InvalidInt { var: &'static str, value: String },

    /// A boolean-typed env variable was provided but could not be parsed.
    #[error("environment variable `{var}` is not a valid boolean: `{value}`")]
    InvalidBool { var: &'static str, value: String },
}

impl AppConfig {
    /// Reads the configuration from `std::env`.
    ///
    /// - `DATABASE_URL` (required, non-empty)
    /// - `VIZ_API_KEY`  (required, non-empty, >= [`MIN_API_KEY_LEN`] bytes)
    /// - `VIZ_API_BIND_ADDR` (optional, default `0.0.0.0:3100`)
    /// - `VIZ_DB_STATEMENT_TIMEOUT_MS` (optional, default
    ///   [`DEFAULT_DB_STATEMENT_TIMEOUT_MS`])
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = required_env("DATABASE_URL")?;
        let api_key = required_env("VIZ_API_KEY")?;
        if api_key.len() < MIN_API_KEY_LEN {
            return Err(ConfigError::ApiKeyTooShort {
                actual: api_key.len(),
                minimum: MIN_API_KEY_LEN,
            });
        }

        let bind_addr = optional_env("VIZ_API_BIND_ADDR")?
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string());

        let db_statement_timeout_ms = optional_u64_env(
            "VIZ_DB_STATEMENT_TIMEOUT_MS",
            DEFAULT_DB_STATEMENT_TIMEOUT_MS,
        )?;

        let enable_swagger_ui =
            optional_bool_env("VIZ_ENABLE_SWAGGER_UI", DEFAULT_ENABLE_SWAGGER_UI)?;

        Ok(Self {
            database_url,
            api_key,
            bind_addr,
            db_statement_timeout_ms,
            enable_swagger_ui,
        })
    }
}

fn required_env(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        Ok(_) => Err(ConfigError::Missing(name)),
        Err(VarError::NotPresent) => Err(ConfigError::Missing(name)),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::NotUnicode { var: name }),
    }
}

fn optional_env(name: &'static str) -> Result<Option<String>, ConfigError> {
    match env::var(name) {
        Ok(v) => Ok(Some(v)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::NotUnicode { var: name }),
    }
}

/// Reads an optional boolean env variable (`true`/`false`/`1`/`0`,
/// case-insensitive), falling back to `default` when absent or empty. A
/// present-but-unrecognised value is a hard error (`InvalidBool`), so a typo
/// never silently flips the flag.
fn optional_bool_env(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    match optional_env(name)?.filter(|v| !v.is_empty()) {
        None => Ok(default),
        Some(v) => match v.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::InvalidBool {
                var: name,
                value: v,
            }),
        },
    }
}

/// Reads an optional `u64` env variable, falling back to `default` when the
/// variable is absent or empty. A present-but-unparseable value is a hard
/// error (`InvalidInt`) rather than a silent fallback: a typo in
/// `VIZ_DB_STATEMENT_TIMEOUT_MS` must fail fast, not disable the guardrail.
fn optional_u64_env(name: &'static str, default: u64) -> Result<u64, ConfigError> {
    match optional_env(name)?.filter(|v| !v.is_empty()) {
        None => Ok(default),
        Some(v) => v.parse::<u64>().map_err(|_| ConfigError::InvalidInt {
            var: name,
            value: v,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_sensitive_fields() {
        let cfg = AppConfig {
            database_url: "postgres://user:supersecretpassword@localhost/db".to_string(),
            api_key: "verysecretapikey1234567890".to_string(),
            bind_addr: "127.0.0.1:3100".to_string(),
            db_statement_timeout_ms: DEFAULT_DB_STATEMENT_TIMEOUT_MS,
            enable_swagger_ui: DEFAULT_ENABLE_SWAGGER_UI,
        };

        let dbg = format!("{:?}", cfg);

        assert!(
            dbg.contains("<redacted>"),
            "Debug output should contain `<redacted>` marker: {dbg}"
        );
        assert!(
            !dbg.contains("supersecretpassword"),
            "Debug output must not leak the DB password: {dbg}"
        );
        assert!(
            !dbg.contains("verysecretapikey1234567890"),
            "Debug output must not leak the API key: {dbg}"
        );
        assert!(
            dbg.contains("127.0.0.1:3100"),
            "Debug output should still expose non-sensitive bind_addr: {dbg}"
        );
    }

    #[test]
    fn optional_u64_env_falls_back_when_absent() {
        // Use a variable name that is extremely unlikely to be set in the
        // test environment so the absent branch is exercised deterministically.
        let got = optional_u64_env("VIZ_TEST_ABSENT_TIMEOUT_VAR_XYZ", 5000).unwrap();
        assert_eq!(got, 5000);
    }

    #[test]
    fn optional_bool_env_parses_and_defaults() {
        // Absent and empty both fall back to the default.
        assert!(optional_bool_env("VIZ_TEST_ABSENT_BOOL_XYZ", true).unwrap());
        // SAFETY: unique variable name no other test reads; set/read/remove
        // happen synchronously here, so the parallel runner cannot interleave.
        unsafe { env::set_var("VIZ_TEST_BOOL_VAR", "") };
        let empty_defaults = optional_bool_env("VIZ_TEST_BOOL_VAR", true).unwrap();
        unsafe { env::set_var("VIZ_TEST_BOOL_VAR", "FALSE") };
        let falsey = optional_bool_env("VIZ_TEST_BOOL_VAR", true).unwrap();
        unsafe { env::set_var("VIZ_TEST_BOOL_VAR", "on") };
        let truthy = optional_bool_env("VIZ_TEST_BOOL_VAR", false).unwrap();
        unsafe { env::set_var("VIZ_TEST_BOOL_VAR", "nope") };
        let err = optional_bool_env("VIZ_TEST_BOOL_VAR", true).unwrap_err();
        unsafe { env::remove_var("VIZ_TEST_BOOL_VAR") };
        assert!(empty_defaults, "empty value must use the default");
        assert!(!falsey, "`FALSE` must parse to false");
        assert!(truthy, "`on` must parse to true");
        assert!(matches!(err, ConfigError::InvalidBool { .. }));
    }

    #[test]
    fn optional_u64_env_rejects_non_integer() {
        // SAFETY: `set_var`/`remove_var` mutate process-global state, but this
        // test uses a unique variable name no other test reads, and calls
        // `optional_u64_env` synchronously between set and remove — so the
        // parallel test runner cannot observe an interleaved value.
        unsafe { env::set_var("VIZ_TEST_BAD_TIMEOUT_VAR", "not-a-number") };
        let err = optional_u64_env("VIZ_TEST_BAD_TIMEOUT_VAR", 5000).unwrap_err();
        unsafe { env::remove_var("VIZ_TEST_BAD_TIMEOUT_VAR") };
        assert!(
            matches!(err, ConfigError::InvalidInt { .. }),
            "expected InvalidInt, got {err:?}"
        );
    }
}
