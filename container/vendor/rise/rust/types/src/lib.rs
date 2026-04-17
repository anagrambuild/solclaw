//! Minimal API types for Phoenix WebSocket protocol.
//!
//! These types mirror the wire format used by the Phoenix API without
//! requiring the full phoenix-api-types crate and its dependencies.

pub mod candles;
pub mod client;
pub mod conversions;
pub mod core;
pub mod exchange;
pub mod http_error;
pub mod ix;
pub mod js_safe_ints;
pub mod l2book;
pub mod market;
pub mod market_state;
pub mod market_stats;
pub mod metadata;
pub mod subscription_key;
pub mod trader;
pub mod trader_http;
pub mod trader_key;
pub mod trader_state;
pub mod trades;
pub mod ws;
pub mod ws_error;

// Re-export all types at crate root for backwards compatibility
pub use core::*;

pub use candles::*;
pub use client::*;
pub use conversions::*;
pub use exchange::*;
pub use http_error::*;
pub use ix::*;
pub use js_safe_ints::*;
pub use l2book::*;
pub use market::*;
pub use market_state::*;
pub use market_stats::*;
pub use metadata::*;
pub use subscription_key::*;
pub use trader::*;
pub use trader_http::*;
pub use trader_key::*;
pub use trader_state::{Position, Spline, SubaccountState, Trader};
pub use trades::*;
pub use ws::*;
pub use ws_error::*;

/// Deprecated module for backwards compatibility.
///
/// Use the specific modules instead:
/// - [`core`] for `Decimal`, `Price`
/// - [`market`] for market config and status types
/// - [`exchange`] for `ExchangeKeysView`, `AuthoritySetView`
#[deprecated(
    since = "0.2.0",
    note = "Use specific modules instead: core, market, exchange"
)]
pub mod http {
    pub use crate::core::{Decimal, Price};
    pub use crate::exchange::{AuthoritySetView, ExchangeKeysView};
    pub use crate::market::{
        L2Orderbook, LeverageTier, MarketFeeConfig, MarketInfo, MarketStatus, MarketSummary,
        MarketUnitConfig, MarketView, MarketsView, RiskFactors, RiskState, RiskTier,
    };
}
