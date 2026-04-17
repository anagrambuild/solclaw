use phoenix_types::{
    FundingHistoryQueryParams, FundingHistoryResponse, PhoenixHttpError, TraderKey,
};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;

pub struct FundingClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl FundingClient<'_> {
    pub async fn get_user_funding_history(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        self.get_funding_history_internal(authority, params).await
    }

    pub async fn get_trader_funding_history(
        &self,
        trader_key: &TraderKey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_funding_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_funding_history_internal(
        &self,
        authority: &Pubkey,
        params: FundingHistoryQueryParams,
    ) -> Result<FundingHistoryResponse, PhoenixHttpError> {
        let url = format!(
            "{}/trader/{}/funding-history",
            self.http.api_url, authority
        );

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url));

        if let Some(symbol) = &params.symbol {
            request = request.query(&[("symbol", symbol)]);
        }
        if let Some(start_time) = params.start_time {
            request = request.query(&[("startTime", start_time)]);
        }
        if let Some(end_time) = params.end_time {
            request = request.query(&[("endTime", end_time)]);
        }
        if let Some(limit) = params.limit {
            request = request.query(&[("limit", limit)]);
        }
        if let Some(cursor) = &params.cursor {
            request = request.query(&[("cursor", cursor)]);
        }

        let response = self.http.send_with_rate_limit_retry(request).await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        let body = response.text().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to read response body: {}", e))
        })?;

        serde_json::from_str(&body).map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse FundingHistoryResponse: {}", e))
        })
    }
}
