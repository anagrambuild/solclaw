//! HTTP error types for the Phoenix SDK.

use thiserror::Error;

/// Errors that can occur when using the Phoenix HTTP client.
#[derive(Debug, Error)]
pub enum PhoenixHttpError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// Failed to parse response.
    #[error("Failed to parse response: {0}")]
    ParseFailed(String),

    /// API returned an error.
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    /// API rate limit was hit and automatic retries were exhausted or disabled.
    #[error(
        "Rate limited after {attempts} attempt(s), retry_after_seconds={retry_after_seconds:?}: {message}"
    )]
    RateLimited {
        retry_after_seconds: Option<u64>,
        message: String,
        attempts: u32,
    },

    /// Missing environment variable.
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
}
