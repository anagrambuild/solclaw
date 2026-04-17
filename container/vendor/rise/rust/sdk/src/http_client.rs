//! HTTP client for Phoenix API.
//!
//! This module provides a client for making HTTP requests to the Phoenix API
//! to fetch exchange configuration and market data.

use std::time::Duration;

use phoenix_ix::{IsolatedCollateralFlow, Side};
use phoenix_types::{
    ApiCandle, CandlesQueryParams, CollateralHistoryQueryParams, CollateralHistoryResponse,
    ExchangeKeysView, ExchangeMarketConfig, ExchangeResponse, FundingHistoryQueryParams,
    FundingHistoryResponse, OrderHistoryQueryParams, OrderHistoryResponse, PhoenixHttpError,
    PlaceIsolatedLimitOrderRequest, PlaceIsolatedMarketOrderRequest, PnlPoint, PnlQueryParams,
    TradeHistoryQueryParams, TradeHistoryResponse, TraderKey, TraderView,
};
use reqwest::header::RETRY_AFTER;
use reqwest::{Client, RequestBuilder, Response};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use tracing::debug;

use crate::api::{
    CandlesClient, CollateralClient, ExchangeClient, FundingClient, InviteClient, MarketsClient,
    OrdersClient, TradersClient, TradesClient,
};
use crate::env::PhoenixEnv;
use crate::tx_builder::BracketLegOrders;

const API_KEY_HEADER: &str = "x-api-key";
const RATE_LIMIT_STATUS: u16 = 429;

/// Automatic retry behavior for HTTP 429 (rate-limited) responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRetryConfig {
    /// Enable automatic retry on HTTP 429.
    pub enabled: bool,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
    /// Maximum total time spent sleeping between retries.
    pub max_total_wait: Duration,
    /// Fallback delay if `Retry-After` is missing or invalid.
    pub fallback_delay: Duration,
    /// Maximum delay per retry attempt.
    pub max_delay: Duration,
}

impl Default for RateLimitRetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 2,
            max_total_wait: Duration::from_secs(15),
            fallback_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        }
    }
}

/// Shared HTTP transport used by all resource sub-clients.
#[derive(Debug, Clone)]
pub(crate) struct HttpClientInner {
    pub api_url: String,
    pub api_key: Option<String>,
    pub client: Client,
    pub rate_limit_retry: RateLimitRetryConfig,
}

impl HttpClientInner {
    pub fn maybe_add_api_key(&self, request: RequestBuilder) -> RequestBuilder {
        match &self.api_key {
            Some(key) => request.header(API_KEY_HEADER, key),
            None => request,
        }
    }

    pub async fn send_with_rate_limit_retry(
        &self,
        mut request: RequestBuilder,
    ) -> Result<Response, PhoenixHttpError> {
        let mut retries: u32 = 0;
        let mut total_wait = Duration::ZERO;

        loop {
            let retry_request = request.try_clone();
            let response = request.send().await?;

            if response.status().as_u16() != RATE_LIMIT_STATUS {
                return Ok(response);
            }

            let retry_after_seconds = parse_retry_after_seconds(response.headers());
            let message = response.text().await.unwrap_or_default();
            let attempts = retries.saturating_add(1);

            let can_retry =
                self.rate_limit_retry.enabled && retries < self.rate_limit_retry.max_retries;
            if !can_retry {
                return Err(PhoenixHttpError::RateLimited {
                    retry_after_seconds,
                    message,
                    attempts,
                });
            }

            let Some(next_request) = retry_request else {
                return Err(PhoenixHttpError::RateLimited {
                    retry_after_seconds,
                    message: if message.is_empty() {
                        "rate_limited (request could not be cloned for retry)".to_string()
                    } else {
                        message
                    },
                    attempts,
                });
            };

            let wait = retry_after_seconds
                .map(Duration::from_secs)
                .unwrap_or(self.rate_limit_retry.fallback_delay)
                .min(self.rate_limit_retry.max_delay);
            let next_total_wait = total_wait.saturating_add(wait);

            if next_total_wait > self.rate_limit_retry.max_total_wait {
                return Err(PhoenixHttpError::RateLimited {
                    retry_after_seconds,
                    message: if message.is_empty() {
                        "rate_limited (max_total_wait exceeded)".to_string()
                    } else {
                        message
                    },
                    attempts,
                });
            }

            debug!(
                "HTTP rate limited, retrying attempt {} in {:?} (retry_after={:?})",
                attempts + 1,
                wait,
                retry_after_seconds
            );

            tokio::time::sleep(wait).await;
            total_wait = next_total_wait;
            retries = retries.saturating_add(1);
            request = next_request;
        }
    }
}

/// HTTP client for Phoenix API.
///
/// Provides resource sub-client accessors (e.g. `client.markets()`,
/// `client.traders()`) that mirror the TypeScript SDK's `V1ApiClients`
/// shape. Existing flat methods remain for backwards compatibility and
/// delegate to the sub-clients.
///
/// # Example
///
/// ```no_run
/// use phoenix_sdk::PhoenixHttpClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PhoenixHttpClient::new_from_env();
///
///     // Resource-based access (new)
///     let markets = client.markets().get_markets().await?;
///     let sol = client.markets().get_market("SOL").await?;
///
///     // Flat access (backwards-compatible)
///     let keys = client.get_exchange_keys().await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PhoenixHttpClient {
    inner: HttpClientInner,
}

impl PhoenixHttpClient {
    /// Creates a new HTTP client using environment variables.
    pub fn new_from_env() -> Self {
        Self::from_env(PhoenixEnv::load())
    }

    /// Creates a new HTTP client from a `PhoenixEnv`.
    pub fn from_env(env: PhoenixEnv) -> Self {
        Self {
            inner: HttpClientInner {
                api_url: env.api_url,
                api_key: env.api_key,
                client: Client::new(),
                rate_limit_retry: RateLimitRetryConfig::default(),
            },
        }
    }

    /// Creates a new HTTP client with the given API URL and API key.
    pub fn new(api_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            inner: HttpClientInner {
                api_url: api_url.into(),
                api_key: Some(api_key.into()),
                client: Client::new(),
                rate_limit_retry: RateLimitRetryConfig::default(),
            },
        }
    }

    /// Creates a new HTTP client without an API key.
    pub fn new_public(api_url: impl Into<String>) -> Self {
        Self {
            inner: HttpClientInner {
                api_url: api_url.into(),
                api_key: None,
                client: Client::new(),
                rate_limit_retry: RateLimitRetryConfig::default(),
            },
        }
    }

    /// Sets automatic rate-limit retry behavior for this client.
    pub fn set_rate_limit_retry_config(&mut self, config: RateLimitRetryConfig) {
        self.inner.rate_limit_retry = config;
    }

    /// Builder-style variant of [`Self::set_rate_limit_retry_config`].
    pub fn with_rate_limit_retry_config(mut self, config: RateLimitRetryConfig) -> Self {
        self.inner.rate_limit_retry = config;
        self
    }

    /// Returns the current automatic rate-limit retry configuration.
    pub fn rate_limit_retry_config(&self) -> &RateLimitRetryConfig {
        &self.inner.rate_limit_retry
    }

    // --- Resource sub-client accessors ---

    pub fn markets(&self) -> MarketsClient<'_> {
        MarketsClient { http: &self.inner }
    }

    pub fn exchange(&self) -> ExchangeClient<'_> {
        ExchangeClient { http: &self.inner }
    }

    pub fn traders(&self) -> TradersClient<'_> {
        TradersClient { http: &self.inner }
    }

    pub fn collateral(&self) -> CollateralClient<'_> {
        CollateralClient { http: &self.inner }
    }

    pub fn funding(&self) -> FundingClient<'_> {
        FundingClient { http: &self.inner }
    }

    pub fn orders(&self) -> OrdersClient<'_> {
        OrdersClient { http: &self.inner }
    }

    pub fn trades(&self) -> TradesClient<'_> {
        TradesClient { http: &self.inner }
    }

    pub fn candles(&self) -> CandlesClient<'_> {
        CandlesClient { http: &self.inner }
    }

    pub fn invite(&self) -> InviteClient<'_> {
        InviteClient { http: &self.inner }
    }

    // --- Backwards-compatible flat methods (delegate to sub-clients) ---

    pub async fn get_exchange_keys(&self) -> Result<ExchangeKeysView, PhoenixHttpError> {
        self.exchange().get_keys().await
    }

    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        self.markets().get_markets().await
    }

    pub async fn get_market(
        &self,
        symbol: &str,
    ) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        self.markets().get_market(symbol).await
    }

    pub async fn get_exchange(&self) -> Result<ExchangeResponse, PhoenixHttpError> {
        self.exchange().get_exchange().await
    }

    pub async fn get_traders(
        &self,
        authority: &Pubkey,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        self.traders().get_trader(authority).await
    }

    pub async fn get_collateral_history(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.collateral()
            .get_user_collateral_history(authority, params)
            .await
    }

    pub async fn get_collateral_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.collateral()
            .get_trader_collateral_history(trader_key, params)
            .await
    }

    pub async fn get_funding_history(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_user_funding_history(authority, params)
            .await
    }

    pub async fn get_funding_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.funding()
            .get_trader_funding_history(trader_key, params)
            .await
    }

    pub async fn get_order_history(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.orders()
            .get_trader_order_history(authority, params)
            .await
    }

    pub async fn get_order_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.orders()
            .get_trader_order_history_with_trader_key(trader_key, params)
            .await
    }

    pub async fn get_candles(
        &self,
        params: CandlesQueryParams,
    ) -> Result<Vec<ApiCandle>, PhoenixHttpError> {
        self.candles().get_candles(params).await
    }

    pub async fn get_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_trader_trade_history(authority, params)
            .await
    }

    pub async fn get_trade_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.trades()
            .get_trader_trade_history_with_trader_key(trader_key, params)
            .await
    }

    pub async fn get_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        self.traders().get_trader_pnl(authority, params).await
    }

    pub async fn build_isolated_limit_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        price: f64,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx(
                authority,
                symbol,
                side,
                price,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
            )
            .await
    }

    pub async fn build_isolated_limit_order_tx_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_limit_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx(
        &self,
        authority: &Pubkey,
        symbol: &str,
        side: Side,
        num_base_lots: u64,
        collateral: Option<IsolatedCollateralFlow>,
        allow_cross_and_isolated: bool,
        bracket: Option<&BracketLegOrders>,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx(
                authority,
                symbol,
                side,
                num_base_lots,
                collateral,
                allow_cross_and_isolated,
                bracket,
            )
            .await
    }

    pub async fn build_isolated_market_order_tx_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        self.orders()
            .build_isolated_market_order_tx_with_request(request)
            .await
    }

    pub async fn register_trader(
        &self,
        authority: &Pubkey,
        code: &str,
    ) -> Result<String, PhoenixHttpError> {
        self.invite().activate_invite(authority, code).await
    }
}

fn parse_retry_after_seconds(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PhoenixHttpClient::new("https://api.phoenix.trade/v1", "test-key");
        assert_eq!(client.inner.api_url, "https://api.phoenix.trade/v1");
        assert_eq!(client.inner.api_key.as_deref(), Some("test-key"));
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn test_client_with_string() {
        let url = String::from("https://api.example.com");
        let key = String::from("my-api-key");
        let client = PhoenixHttpClient::new(url, key);
        assert_eq!(client.inner.api_url, "https://api.example.com");
        assert_eq!(client.inner.api_key.as_deref(), Some("my-api-key"));
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn test_client_public() {
        let client = PhoenixHttpClient::new_public("https://api.example.com");
        assert_eq!(client.inner.api_url, "https://api.example.com");
        assert!(client.inner.api_key.is_none());
        assert_eq!(
            client.inner.rate_limit_retry,
            RateLimitRetryConfig::default()
        );
    }

    #[test]
    fn test_parse_retry_after_seconds() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(RETRY_AFTER, reqwest::header::HeaderValue::from_static("3"));
        assert_eq!(parse_retry_after_seconds(&headers), Some(3));

        headers.insert(RETRY_AFTER, reqwest::header::HeaderValue::from_static("0"));
        assert_eq!(parse_retry_after_seconds(&headers), Some(1));

        headers.insert(
            RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("not-a-number"),
        );
        assert_eq!(parse_retry_after_seconds(&headers), None);
    }
}
