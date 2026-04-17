use phoenix_types::{ApiCandle, CandlesQueryParams, PhoenixHttpError};

use crate::http_client::HttpClientInner;

pub struct CandlesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CandlesClient<'_> {
    pub async fn get_candles(
        &self,
        params: CandlesQueryParams,
    ) -> Result<Vec<ApiCandle>, PhoenixHttpError> {
        let url = format!("{}/candles", self.http.api_url);

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url))
            .query(&[("symbol", &params.symbol), ("timeframe", &params.timeframe)]);

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
            PhoenixHttpError::ParseFailed(format!("Failed to parse candles response: {}", e))
        })
    }
}
