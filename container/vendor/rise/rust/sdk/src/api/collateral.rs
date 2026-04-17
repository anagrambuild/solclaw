use phoenix_types::{
    CollateralHistoryQueryParams, CollateralHistoryResponse, PhoenixHttpError, TraderKey,
};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;

pub struct CollateralClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl CollateralClient<'_> {
    pub async fn get_user_collateral_history(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        self.get_collateral_history_internal(authority, params).await
    }

    pub async fn get_trader_collateral_history(
        &self,
        trader_key: &TraderKey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_collateral_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_collateral_history_internal(
        &self,
        authority: &Pubkey,
        params: CollateralHistoryQueryParams,
    ) -> Result<CollateralHistoryResponse, PhoenixHttpError> {
        let url = format!(
            "{}/trader/{}/collateral-history",
            self.http.api_url, authority
        );

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url))
            .query(&[
                ("pdaIndex", params.pda_index.to_string()),
                ("limit", params.request.limit.to_string()),
            ]);

        if let Some(next_cursor) = &params.request.next_cursor {
            request = request.query(&[("nextCursor", next_cursor)]);
        }
        if let Some(prev_cursor) = &params.request.prev_cursor {
            request = request.query(&[("prevCursor", prev_cursor)]);
        }
        if let Some(cursor) = &params.request.cursor {
            request = request.query(&[("cursor", cursor)]);
        }

        let response = self.http.send_with_rate_limit_retry(request).await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!(
                "Failed to parse CollateralHistoryResponse: {}",
                e
            ))
        })
    }
}
