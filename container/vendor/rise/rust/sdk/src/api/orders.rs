use phoenix_ix::{IsolatedCollateralFlow, Side};
use phoenix_types::{
    ApiInstructionResponse, OrderHistoryQueryParams, OrderHistoryResponse, PhoenixHttpError,
    PlaceIsolatedLimitOrderRequest, PlaceIsolatedMarketOrderRequest, TraderKey,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::http_client::HttpClientInner;
use crate::tx_builder::BracketLegOrders;

pub struct OrdersClient<'a> {
    pub(crate) http: &'a HttpClientInner,
}

impl OrdersClient<'_> {
    pub async fn get_trader_order_history(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        self.get_order_history_internal(authority, params).await
    }

    pub async fn get_trader_order_history_with_trader_key(
        &self,
        trader_key: &TraderKey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        let params = params.with_pda_index(trader_key.pda_index);
        self.get_order_history_internal(&trader_key.authority(), params)
            .await
    }

    async fn get_order_history_internal(
        &self,
        authority: &Pubkey,
        params: OrderHistoryQueryParams,
    ) -> Result<OrderHistoryResponse, PhoenixHttpError> {
        let url = format!("{}/trader/{}/order-history", self.http.api_url, authority);

        let mut request = self
            .http
            .maybe_add_api_key(self.http.client.get(&url))
            .query(&[("limit", params.limit)]);

        if let Some(pda_index) = params.trader_pda_index {
            request = request.query(&[("traderPdaIndex", pda_index)]);
        }
        if let Some(market_symbol) = &params.market_symbol {
            request = request.query(&[("marketSymbol", market_symbol)]);
        }
        if let Some(cursor) = &params.cursor {
            request = request.query(&[("cursor", cursor)]);
        }
        if let Some(privy_id) = &params.privy_id {
            request = request.query(&[("privyId", privy_id)]);
        }

        let response = self.http.send_with_rate_limit_retry(request).await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse OrderHistoryResponse: {}", e))
        })
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
        let transfer_amount = collateral_transfer_amount(&collateral)?;

        let request = PlaceIsolatedLimitOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            price: Some(price),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            ..Default::default()
        };

        self.build_isolated_limit_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_limit_order_tx_with_request(
        &self,
        request: PlaceIsolatedLimitOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let url = format!("{}/ix/place-isolated-limit-order", self.http.api_url);

        let response = self
            .http
            .send_with_rate_limit_retry(
                self.http
                    .maybe_add_api_key(self.http.client.post(&url))
                    .json(&request),
            )
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        let api_ixs: Vec<ApiInstructionResponse> = response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse instruction response: {}", e))
        })?;

        api_ixs.into_iter().map(try_into_instruction).collect()
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
        let transfer_amount = collateral_transfer_amount(&collateral)?;
        let tp_sl = bracket.map(BracketLegOrders::to_tp_sl_config);

        let request = PlaceIsolatedMarketOrderRequest {
            authority: authority.to_string(),
            symbol: symbol.to_string(),
            side: side.to_api_string().to_string(),
            num_base_lots: Some(num_base_lots),
            transfer_amount,
            allow_cross_and_isolated_for_asset: Some(allow_cross_and_isolated),
            tp_sl,
            ..Default::default()
        };

        self.build_isolated_market_order_tx_with_request(request)
            .await
    }

    pub async fn build_isolated_market_order_tx_with_request(
        &self,
        request: PlaceIsolatedMarketOrderRequest,
    ) -> Result<Vec<Instruction>, PhoenixHttpError> {
        let url = format!("{}/ix/place-isolated-market-order", self.http.api_url);

        let response = self
            .http
            .send_with_rate_limit_retry(
                self.http
                    .maybe_add_api_key(self.http.client.post(&url))
                    .json(&request),
            )
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(PhoenixHttpError::ApiError { status, message });
        }

        let api_ixs: Vec<ApiInstructionResponse> = response.json().await.map_err(|e| {
            PhoenixHttpError::ParseFailed(format!("Failed to parse instruction response: {}", e))
        })?;

        api_ixs.into_iter().map(try_into_instruction).collect()
    }
}

fn collateral_transfer_amount(
    collateral: &Option<IsolatedCollateralFlow>,
) -> Result<u64, PhoenixHttpError> {
    match collateral {
        Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }) => Ok(*collateral),
        Some(IsolatedCollateralFlow::Deposit { .. }) => Err(PhoenixHttpError::ApiError {
            status: 0,
            message: "IsolatedCollateralFlow::Deposit is not supported by the server-side \
                      endpoint; use TransferFromCrossMargin instead"
                .to_string(),
        }),
        None => Ok(0),
    }
}

fn try_into_instruction(api_ix: ApiInstructionResponse) -> Result<Instruction, PhoenixHttpError> {
    let program_id: Pubkey = api_ix
        .program_id
        .parse()
        .map_err(|e| PhoenixHttpError::ParseFailed(format!("Invalid program_id pubkey: {}", e)))?;

    let accounts = api_ix
        .keys
        .into_iter()
        .map(|meta| {
            let pubkey: Pubkey = meta.pubkey.parse().map_err(|e| {
                PhoenixHttpError::ParseFailed(format!("Invalid account pubkey: {}", e))
            })?;
            Ok(if meta.is_writable {
                AccountMeta::new(pubkey, meta.is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, meta.is_signer)
            })
        })
        .collect::<Result<Vec<_>, PhoenixHttpError>>()?;

    Ok(Instruction::new_with_bytes(
        program_id,
        &api_ix.data,
        accounts,
    ))
}
