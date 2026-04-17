//! Trade types for Phoenix WebSocket protocol.
//!
//! These types represent real-time trade events streamed via WebSocket.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::Side;
use crate::js_safe_ints::JsSafeU64;

/// Trades message from the trades channel (wrapper with array of events).
///
/// The trades channel sends messages containing the symbol and an array of
/// trade events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesMessage {
    /// Market symbol (e.g., "SOL").
    pub symbol: String,
    /// Array of trade events.
    pub trades: Vec<TradeEvent>,
}

/// Individual trade event from the trades channel.
///
/// Represents a single trade with price, quantity, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeEvent {
    /// Slot when the trade occurred.
    pub slot: JsSafeU64,
    /// Index within the slot.
    pub slot_index: u32,
    /// Timestamp of the trade (RFC3339).
    pub timestamp: DateTime<Utc>,
    /// Market symbol (e.g., "SOL").
    pub symbol: String,
    /// Taker authority pubkey.
    pub taker: String,
    /// Monotonically increasing trade sequence number.
    pub trade_sequence_number: JsSafeU64,
    /// Side of the taker order.
    pub side: Side,
    /// Base lots filled.
    pub base_lots_filled: JsSafeU64,
    /// Quote lots filled.
    pub quote_lots_filled: JsSafeU64,
    /// Fee in quote lots.
    pub fee_in_quote_lots: JsSafeU64,
    /// Human-readable base amount.
    pub base_amount: f64,
    /// Human-readable quote amount.
    pub quote_amount: f64,
    /// Number of fills in this trade.
    pub num_fills: u32,
}

/// Subscription request for the trades channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TradesSubscriptionRequest {
    /// Market symbol to filter trades (e.g., "SOL").
    pub symbol: String,
}

// ============================================================================
// Trade History Types (HTTP API)
// ============================================================================

/// Individual trade record from the trade history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryItem {
    /// Market symbol associated with the fill (e.g., "SOL").
    pub market_symbol: String,
    /// Human-readable base quantity.
    pub base_qty: String,
    /// Human-readable quote quantity.
    pub quote_qty: String,
    /// Human-readable price derived from the fill quantities.
    pub price: String,
    /// Timestamp of the fill (ISO 8601).
    pub timestamp: String,
    /// Transaction signature containing the fill.
    pub transaction_signature: String,
    /// Instruction type that emitted this fill (e.g., PlaceMarketOrder,
    /// LiquidateViaMarketOrder).
    pub instruction_type: String,
}

/// Response from the trade history endpoint.
pub type TradeHistoryResponse = crate::core::PaginatedResponse<Vec<TradeHistoryItem>>;

/// Query parameters for fetching trader trade history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryQueryParams {
    /// PDA index for the trader account.
    #[serde(default, alias = "pdaIndex", alias = "pda_index")]
    pub pda_index: u8,
    /// Optional market symbol filter (e.g., "SOL").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_symbol: Option<String>,
    /// Number of items to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Cursor for pagination (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl TradeHistoryQueryParams {
    /// Creates new query params with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the PDA index.
    pub fn with_pda_index(mut self, pda_index: u8) -> Self {
        self.pda_index = pda_index;
        self
    }

    /// Sets the market symbol filter.
    pub fn with_market_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.market_symbol = Some(symbol.into());
        self
    }

    /// Sets the limit.
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the cursor for pagination.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_trade_event() {
        let json = r#"{
            "slot": "123456789",
            "slotIndex": 5,
            "timestamp": "2026-02-03T16:12:03Z",
            "symbol": "SOL",
            "taker": "ABC123pubkey",
            "tradeSequenceNumber": "100",
            "side": "bid",
            "baseLotsFilled": "1000",
            "quoteLotsFilled": "150000",
            "feeInQuoteLots": "30",
            "baseAmount": 10.0,
            "quoteAmount": 1500.0,
            "numFills": 2
        }"#;

        let trade: TradeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(trade.symbol, "SOL");
        assert_eq!(trade.base_amount, 10.0);
        assert_eq!(trade.quote_amount, 1500.0);
        assert_eq!(trade.num_fills, 2);
        assert_eq!(trade.side, Side::Bid);
    }

    #[test]
    fn test_serialize_trade_event() {
        let trade = TradeEvent {
            slot: 123456789u64.into(),
            slot_index: 5,
            timestamp: "2026-02-03T16:12:03Z".parse().unwrap(),
            symbol: "SOL".to_string(),
            taker: "ABC123pubkey".to_string(),
            trade_sequence_number: 100u64.into(),
            side: Side::Ask,
            base_lots_filled: 1000u64.into(),
            quote_lots_filled: 150000u64.into(),
            fee_in_quote_lots: 30u64.into(),
            base_amount: 10.0,
            quote_amount: 1500.0,
            num_fills: 2,
        };

        let json = serde_json::to_string(&trade).unwrap();
        assert!(json.contains("\"symbol\":\"SOL\""));
        assert!(json.contains("\"baseAmount\":10"));
        assert!(json.contains("\"side\":\"ask\""));
    }

    #[test]
    fn test_trades_subscription_request() {
        let req = TradesSubscriptionRequest {
            symbol: "SOL".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"symbol\":\"SOL\""));
    }
}
