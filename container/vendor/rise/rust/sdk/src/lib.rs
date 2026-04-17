//! Phoenix WebSocket SDK for Rust.
//!
//! This crate provides a client for subscribing to real-time trader state
//! updates from the Phoenix exchange via WebSocket.
//!
//! # Example
//!
//! ```no_run
//! use phoenix_sdk::{PhoenixWSClient, Trader, TraderKey};
//! use solana_pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to the WebSocket server
//!     let client = PhoenixWSClient::new("wss://api.phoenix.trade/v1/ws", None)?;
//!
//!     // Create a trader key from your authority pubkey
//!     let key = TraderKey::new(Pubkey::new_unique());
//!
//!     // Create a trader state container
//!     let mut trader = Trader::new(key.clone());
//!
//!     // Subscribe to trader state updates using the authority pubkey
//!     let (mut rx, _handle) = client.subscribe_to_trader_state(&key.authority())?;
//!
//!     // Process updates
//!     while let Some(msg) = rx.recv().await {
//!         trader.apply_update(&msg);
//!         println!(
//!             "Collateral: {}, Positions: {}",
//!             trader.total_collateral(),
//!             trader.all_positions().len()
//!         );
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod api;
mod client;
mod env;
mod http_client;
mod tx_builder;
mod ws_client;

// Re-export main types
pub use api::{
    CandlesClient, CollateralClient, ExchangeClient, FundingClient, InviteClient, MarketsClient,
    OrdersClient, TradersClient, TradesClient,
};
pub use client::PhoenixClient;
pub use env::PhoenixEnv;
pub use http_client::{PhoenixHttpClient, RateLimitRetryConfig};
// Re-export phoenix-ix types users will need for orders
pub use phoenix_ix::{
    CancelId, CondensedOrder, FifoOrderId, MultiLimitOrderParams, OrderFlags,
    RegisterTraderParams, SelfTradeBehavior, Side, TransferCollateralParams,
};
pub use phoenix_ix::{
    CancelStopLossParams, Direction, IsolatedCollateralFlow, IsolatedLimitOrderParams,
    IsolatedMarketOrderParams, StopLossOrderKind, StopLossParams,
};
/// Re-export the types crate for direct access if needed.
pub use phoenix_types as types;
pub use phoenix_types::conversions::*;
// Re-export useful types from the types crate
pub use phoenix_types::{
    AllMidsData, ApiCandle, CROSS_MARGIN_SUBACCOUNT_IDX, CandleData, CandlesQueryParams,
    CandlesSubscriptionRequest, ClientCommand, ClientSubscriptionId, CollateralEvent,
    CollateralHistoryQueryParams, CollateralHistoryResponse, ETERNAL_PROGRAM_ID,
    ExchangeMarketConfig, ExchangeView, FundingHistoryEvent, FundingHistoryQueryParams,
    FundingHistoryResponse, FundingRateMessage, L2Book, L2BookUpdate, LogicalSubscription,
    MarginTrigger, Market, MarketStats, MarketStatsUpdate, OrderHistoryItem,
    OrderHistoryQueryParams, OrderHistoryResponse, OrderStatus, PaginatedResponse,
    PhoenixClientError, PhoenixClientEvent, PhoenixClientSubscriptionHandle, PhoenixHttpError,
    PhoenixMetadata, PhoenixSubscription, PhoenixWsError, PlaceIsolatedLimitOrderRequest,
    PlaceIsolatedMarketOrderRequest, PnlPoint, PnlQueryParams, PnlResolution, Position, PriceLevel,
    RuntimeState, ServerMessage, Spline, SubaccountState, SubscriptionKey, Timeframe,
    TpSlOrderConfig, TradeEvent, TradeHistoryItem, TradeHistoryQueryParams, TradeHistoryResponse,
    Trader, TraderKey, TraderStateDelta, TraderStatePayload, TraderStateServerMessage,
    TraderStateSnapshot, TradesMessage, TradesSubscriptionRequest,
};
pub use rust_decimal::Decimal;
pub use tx_builder::{BracketLegOrders, PhoenixTxBuilder, PhoenixTxBuilderError};
pub use ws_client::{PhoenixWSClient, SubscriptionHandle, WsConnectionStatus};
