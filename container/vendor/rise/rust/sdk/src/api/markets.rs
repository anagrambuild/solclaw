use phoenix_types::{ExchangeMarketConfig, PhoenixHttpError};

use crate::http_client::HttpClientInner;

pub struct MarketsClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl MarketsClient<'_> {
    pub async fn get_markets(&self) -> Result<Vec<ExchangeMarketConfig>, PhoenixHttpError> {
        let url = format!("{}/exchange/markets", self.http.api_url);

        let response = self
            .http
            .send_with_rate_limit_retry(self.http.maybe_add_api_key(self.http.client.get(&url)))
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| PhoenixHttpError::ParseFailed(format!("Failed to parse markets: {}", e)))
    }

    pub async fn get_market(
        &self,
        symbol: &str,
    ) -> Result<ExchangeMarketConfig, PhoenixHttpError> {
        let symbol_upper = symbol.to_ascii_uppercase();
        let url = format!("{}/exchange/market/{}", self.http.api_url, symbol_upper);

        let response = self
            .http
            .send_with_rate_limit_retry(self.http.maybe_add_api_key(self.http.client.get(&url)))
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse ExchangeMarketConfig: {}", e))
        })
    }
}
