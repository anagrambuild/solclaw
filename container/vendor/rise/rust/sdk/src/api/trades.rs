use phoenix_types::{
    PhoenixHttpError, TradeHistoryQueryParams, TradeHistoryResponse, TraderKey,
};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;

pub struct TradesClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl TradesClient<'_> {
    pub async fn get_trader_trade_history(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        self.get_trade_history_internal(authority, params).await
    }

    pub async fn get_trader_trade_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_trade_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_trade_history_internal(
        &self,
        authority: &Pubkey,
        params: TradeHistoryQueryParams,
    ) -> Result<TradeHistoryResponse, PhoenixHttpError> {
        let url = format!(
            "{}/trader/{}/trades-history",
            self.http.api_url, authority
        );

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url))
            .query(&[("pdaIndex", params.pda_index)]);

        if let Some(market_symbol) = &params.market_symbol {
            request = request.query(&[("market_symbol", market_symbol)]);
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

        response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse TradeHistoryResponse: {}", e))
        })
    }
}
