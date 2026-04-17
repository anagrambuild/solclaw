use phoenix_types::{ExchangeKeysView, ExchangeResponse, PhoenixHttpError};

use crate::http_client::HttpClientInner;

pub struct ExchangeClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl ExchangeClient<'_> {
    pub async fn get_exchange(&self) -> Result<ExchangeResponse, PhoenixHttpError> {
        let url = format!("{}/exchange", self.http.api_url);

        let response = self
            .http
            .send_with_rate_limit_retry(self.http.maybe_add_api_key(self.http.client.get(&url)))
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        let body = response.text().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to read response body: {}", e))
        })?;

        serde_json::from_str(&body).map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse ExchangeResponse: {}", e))
        })
    }

    pub async fn get_keys(&self) -> Result<ExchangeKeysView, PhoenixHttpError> {
        let url = format!("{}/exchange/keys", self.http.api_url);

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
            PhoenixHttpError::ParseFailed(format!("Failed to parse ExchangeKeysView: {}", e))
        })
    }
}
