//! Environment configuration for Phoenix SDK.
//!
//! This module provides a centralized way to load configuration from
//! environment variables with sane defaults.

use phoenix_types::PhoenixWsError;
use url::Url;

pub(crate) const PHOENIX_WS_URL_ENV: &str = "PHOENIX_WS_URL";
pub(crate) const PHOENIX_API_URL_ENV: &str = "PHOENIX_API_URL";
pub(crate) const PHOENIX_API_KEY_ENV: &str = "PHOENIX_API_KEY";

pub(crate) const DEFAULT_PHOENIX_API_URL: &str = "https://public-api.phoenix.trade";
pub(crate) const DEFAULT_WS_URL: &str = "wss://public-api.phoenix.trade/ws";

/// Environment configuration for Phoenix SDK.
///
/// Holds the API URL, WebSocket URL, and optional API key needed to connect
/// to the Phoenix API.
///
/// # Example
///
/// ```no_run
/// use phoenix_sdk::PhoenixEnv;
///
/// // Load configuration from environment variables
/// let env = PhoenixEnv::load();
///
/// println!("API URL: {}", env.api_url);
/// println!("WS URL: {}", env.ws_url);
/// println!("API Key: {:?}", env.api_key.as_ref().map(|_| "***"));
/// ```
#[derive(Debug, Clone)]
pub struct PhoenixEnv {
    /// Base URL for the Phoenix HTTP API.
    pub api_url: String,
    /// WebSocket URL for real-time subscriptions.
    pub ws_url: String,
    /// Optional API key for authenticated endpoints.
    pub api_key: Option<String>,
}

impl PhoenixEnv {
    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// * `PHOENIX_API_URL` - Base URL for the Phoenix API. Defaults to `https://public-api.phoenix.trade`.
    /// * `PHOENIX_WS_URL` - WebSocket URL. If not set, derived from the API URL
    ///   by converting the scheme (https→wss, http→ws) and appending `/ws`.
    /// * `PHOENIX_API_KEY` - Optional API key for authenticated endpoints.
    pub fn load() -> Self {
        let api_url = std::env::var(PHOENIX_API_URL_ENV)
            .unwrap_or_else(|_| DEFAULT_PHOENIX_API_URL.to_string());

        let ws_url = std::env::var(PHOENIX_WS_URL_ENV).unwrap_or_else(|_| {
            ws_url_from_api_url(&api_url).unwrap_or_else(|_| DEFAULT_WS_URL.to_string())
        });

        let api_key = std::env::var(PHOENIX_API_KEY_ENV).ok();

        Self {
            api_url,
            ws_url,
            api_key,
        }
    }
}

impl Default for PhoenixEnv {
    /// Returns the default environment configuration.
    ///
    /// Uses `https://public-api.phoenix.trade` as the API URL and
    /// `wss://public-api.phoenix.trade/ws` as the WebSocket URL. No API key is
    /// set.
    fn default() -> Self {
        Self {
            api_url: DEFAULT_PHOENIX_API_URL.to_string(),
            ws_url: DEFAULT_WS_URL.to_string(),
            api_key: None,
        }
    }
}

// ============================================================================
// URL utilities
// ============================================================================

pub(crate) fn ws_url_from_api_url(api_url: &str) -> Result<String, PhoenixWsError> {
    let mut url = Url::parse(api_url)?;

    // Convert http(s) to ws(s) by replacing "http" prefix with "ws"
    let scheme = url.scheme();
    if let Some(ws_scheme) = scheme.strip_prefix("http") {
        let new_scheme = format!("ws{}", ws_scheme);
        let _ = url.set_scheme(&new_scheme);
    } else if scheme != "ws" && scheme != "wss" {
        return Err(PhoenixWsError::UnsupportedUrlScheme(scheme.to_string()));
    }

    // Append /ws path if not already present
    let mut segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();
    if segments.last().copied() != Some("ws") {
        segments.push("ws");
    }
    url.set_path(&format!("/{}", segments.join("/")));
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        // load() always succeeds with defaults when env vars are not set
        let env = PhoenixEnv::load();
        // Should have valid URLs (either from env or defaults)
        assert!(!env.api_url.is_empty());
        assert!(!env.ws_url.is_empty());
    }

    #[test]
    fn test_default_env() {
        let env = PhoenixEnv::default();
        assert_eq!(env.api_url, "https://public-api.phoenix.trade");
        assert_eq!(env.ws_url, "wss://public-api.phoenix.trade/ws");
        assert!(env.api_key.is_none());
    }

    #[test]
    fn test_ws_url_from_https_api_url_appends_ws() {
        let ws_url = ws_url_from_api_url("https://public-api.phoenix.trade").unwrap();
        assert_eq!(ws_url, "wss://public-api.phoenix.trade/ws");
    }

    #[test]
    fn test_ws_url_from_http_api_url_appends_ws() {
        let ws_url = ws_url_from_api_url("http://localhost:8080").unwrap();
        assert_eq!(ws_url, "ws://localhost:8080/ws");
    }

    #[test]
    fn test_ws_url_preserves_path() {
        let ws_url = ws_url_from_api_url("https://api.phoenix.trade/v1").unwrap();
        assert_eq!(ws_url, "wss://api.phoenix.trade/v1/ws");
    }

    #[test]
    fn test_ws_url_handles_trailing_slash() {
        let ws_url = ws_url_from_api_url("https://public-api.phoenix.trade/").unwrap();
        assert_eq!(ws_url, "wss://public-api.phoenix.trade/ws");
    }

    #[test]
    fn test_ws_url_does_not_double_append_ws() {
        let ws_url = ws_url_from_api_url("https://public-api.phoenix.trade/ws").unwrap();
        assert_eq!(ws_url, "wss://public-api.phoenix.trade/ws");
    }
}
