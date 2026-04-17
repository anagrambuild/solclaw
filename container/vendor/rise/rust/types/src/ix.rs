use serde::{Deserialize, Serialize};

/// Account metadata returned from instruction-building endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// API representation of a Solana instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstructionResponse {
    pub data: Vec<u8>,
    pub keys: Vec<ApiAccountMeta>,
    pub program_id: String,
}

/// TP/SL configuration shared across isolated order endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TpSlOrderConfig {
    #[serde(default)]
    pub take_profit_trigger_price: Option<f64>,
    #[serde(default)]
    pub take_profit_trigger_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub take_profit_execution_price: Option<f64>,
    #[serde(default)]
    pub take_profit_execution_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub stop_loss_trigger_price: Option<f64>,
    #[serde(default)]
    pub stop_loss_trigger_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub stop_loss_execution_price: Option<f64>,
    #[serde(default)]
    pub stop_loss_execution_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub order_kind: Option<String>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
}

/// Request payload for /ix/place-isolated-limit-order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedLimitOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub price_in_ticks: Option<u64>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_reduce_only: Option<bool>,
    #[serde(default)]
    pub is_post_only: Option<bool>,
    #[serde(default)]
    pub slide: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub tp_sl: Option<TpSlOrderConfig>,
}

/// Request payload for /ix/place-isolated-market-order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaceIsolatedMarketOrderRequest {
    pub authority: String,
    #[serde(default)]
    pub position_authority: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(default)]
    pub num_base_lots: Option<u64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub transfer_amount: u64,
    #[serde(default)]
    pub max_price_in_ticks: Option<u64>,
    #[serde(default)]
    pub pda_index: Option<u8>,
    #[serde(default)]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
    #[serde(default)]
    pub fee_payer: Option<String>,
    #[serde(default)]
    pub is_reduce_only: Option<bool>,
    #[serde(default)]
    pub skip_transfer_to_parent: Option<bool>,
    #[serde(default)]
    pub tp_sl: Option<TpSlOrderConfig>,
}
