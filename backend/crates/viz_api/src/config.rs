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

/// Parsed configuration for the viz API.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Postgres connection URL for the read-only `pompote_viz_reader` role.
    pub database_url: String,
    /// Expected value of the `X-API-Key` HTTP header.
    pub api_key: String,
    /// Socket address the axum server will bind to.
    pub bind_addr: String,
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
}

impl AppConfig {
    /// Reads the configuration from `std::env`.
    ///
    /// - `DATABASE_URL` (required, non-empty)
    /// - `VIZ_API_KEY`  (required, non-empty, >= [`MIN_API_KEY_LEN`] bytes)
    /// - `VIZ_API_BIND_ADDR` (optional, default `0.0.0.0:3100`)
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

        Ok(Self {
            database_url,
            api_key,
            bind_addr,
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
