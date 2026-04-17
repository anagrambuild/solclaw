use phoenix_types::{PhoenixHttpError, PnlPoint, PnlQueryParams, TraderStateResponse, TraderView};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;

pub struct TradersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradersClient<'_> {
    pub async fn get_trader(
        &self,
        authority: &Pubkey,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        self.get_trader_internal(authority, 0).await
    }

    pub async fn get_trader_internal(
        &self,
        authority: &Pubkey,
        pda_index: u8,
    ) -> Result<Vec<TraderView>, PhoenixHttpError> {
        let url = format!("{}/trader/{}/state", self.http.api_url, authority);

        let response = self
            .http
            .send_with_rate_limit_retry(
                self.http
                    .maybe_add_api_key(self.http.client.get(&url))
                    .query(&[("pdaIndex", pda_index)]),
            )
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        let resp: TraderStateResponse = response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse TraderStateResponse: {}", e))
        })?;

        Ok(resp.traders)
    }

    pub async fn get_trader_pnl(
        &self,
        authority: &Pubkey,
        params: PnlQueryParams,
    ) -> Result<Vec<PnlPoint>, PhoenixHttpError> {
        let url = format!("{}/trader/{}/pnl", self.http.api_url, authority);

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url))
            .query(&[("resolution", params.resolution.to_string())]);

        if let Some(start_time) = params.start_time {
            request = request.query(&[("startTime", start_time)]);
        }
        if let Some(end_time) = params.end_time {
            request = request.query(&[("endTime", end_time)]);
        }
        if let Some(limit) = params.limit {
            request = request.query(&[("limit", limit)]);
        }

        let response = self.http.send_with_rate_limit_retry(request).await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse PnL response: {}", e))
        })
    }
}
