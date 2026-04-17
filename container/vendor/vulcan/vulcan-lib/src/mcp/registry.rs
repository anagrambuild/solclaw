//! Tool registry — static tool definitions with JSON schemas for MCP.

use serde_json::{json, Value};

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub group: &'static str,
    pub dangerous: bool,
    pub schema: fn() -> Value,
}

/// All tools exposed by the Vulcan MCP server.
pub static TOOLS: &[ToolDef] = &[
    // ── Market (read-only) ──────────────────────────────────────────────
    ToolDef {
        name: "vulcan_market_list",
        description: "List all available perpetual markets on Phoenix DEX with fees and leverage info.",
        group: "market",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_market_ticker",
        description: "Get real-time ticker data for a market: mark price, funding rate, 24h volume and change.",
        group: "market",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_market_info",
        description: "Get detailed market configuration: tick size, fees, funding params, leverage tiers.",
        group: "market",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_market_orderbook",
        description: "Get L2 orderbook snapshot with bids, asks, mid price, and spread.",
        group: "market",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "depth": { "type": "integer", "description": "Number of price levels per side", "default": 10 }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_market_candles",
        description: "Get historical candlestick (OHLCV) data for a market.",
        group: "market",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "interval": { "type": "string", "description": "Candle interval: 1m, 5m, 15m, 1h, 4h, 1d", "default": "1h" },
                "limit": { "type": "integer", "description": "Max candles to return", "default": 50 }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },

    // ── Trade (dangerous) ───────────────────────────────────────────────
    ToolDef {
        name: "vulcan_trade_market_buy",
        description: "Place a market buy order. Executes immediately at best available price.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "size": { "type": "number", "description": "Order size in base lots" },
                "tp": { "type": "number", "description": "Optional take-profit price" },
                "sl": { "type": "number", "description": "Optional stop-loss price" },
                "isolated": { "type": "boolean", "description": "Use isolated margin (dedicated collateral per position)" },
                "collateral": { "type": "number", "description": "USDC collateral for isolated subaccount (requires isolated=true)" },
                "reduce_only": { "type": "boolean", "description": "Order can only reduce existing position" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "size", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_market_sell",
        description: "Place a market sell order. Executes immediately at best available price.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "size": { "type": "number", "description": "Order size in base lots" },
                "tp": { "type": "number", "description": "Optional take-profit price" },
                "sl": { "type": "number", "description": "Optional stop-loss price" },
                "isolated": { "type": "boolean", "description": "Use isolated margin (dedicated collateral per position)" },
                "collateral": { "type": "number", "description": "USDC collateral for isolated subaccount (requires isolated=true)" },
                "reduce_only": { "type": "boolean", "description": "Order can only reduce existing position" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "size", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_limit_buy",
        description: "Place a limit buy order at a specific price. Rests on the book until filled or cancelled.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "size": { "type": "number", "description": "Order size in base lots" },
                "price": { "type": "number", "description": "Limit price in USD" },
                "tp": { "type": "number", "description": "Optional take-profit price" },
                "sl": { "type": "number", "description": "Optional stop-loss price" },
                "isolated": { "type": "boolean", "description": "Use isolated margin (dedicated collateral per position)" },
                "collateral": { "type": "number", "description": "USDC collateral for isolated subaccount (requires isolated=true)" },
                "reduce_only": { "type": "boolean", "description": "Order can only reduce existing position" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "size", "price", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_limit_sell",
        description: "Place a limit sell order at a specific price. Rests on the book until filled or cancelled.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "size": { "type": "number", "description": "Order size in base lots" },
                "price": { "type": "number", "description": "Limit price in USD" },
                "tp": { "type": "number", "description": "Optional take-profit price" },
                "sl": { "type": "number", "description": "Optional stop-loss price" },
                "isolated": { "type": "boolean", "description": "Use isolated margin (dedicated collateral per position)" },
                "collateral": { "type": "number", "description": "USDC collateral for isolated subaccount (requires isolated=true)" },
                "reduce_only": { "type": "boolean", "description": "Order can only reduce existing position" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "size", "price", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_multi_limit",
        description: "Place multiple limit orders (bids and asks) in a single transaction. Much faster than placing orders individually. Orders are post-only.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "bids": {
                    "type": "array",
                    "description": "Array of bid orders, each with price (USD) and size (base lots)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "price": { "type": "number", "description": "Limit price in USD" },
                            "size": { "type": "integer", "description": "Order size in base lots" }
                        },
                        "required": ["price", "size"]
                    }
                },
                "asks": {
                    "type": "array",
                    "description": "Array of ask orders, each with price (USD) and size (base lots)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "price": { "type": "number", "description": "Limit price in USD" },
                            "size": { "type": "integer", "description": "Order size in base lots" }
                        },
                        "required": ["price", "size"]
                    }
                },
                "slide": { "type": "boolean", "description": "Whether orders should slide to top of book if they would cross. Default false.", "default": false },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "bids", "asks", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_orders",
        description: "List open orders. Omit symbol to list across all markets.",
        group: "trade",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL. Omit to list all markets." }
            },
            "required": [],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_cancel",
        description: "Cancel specific orders by their IDs.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "order_ids": { "type": "array", "items": { "type": "string" }, "description": "Order IDs to cancel" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "order_ids", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_cancel_all",
        description: "Cancel all open orders for a market.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "acknowledged"],
            "additionalProperties": false
        }),
    },

    ToolDef {
        name: "vulcan_trade_set_tpsl",
        description: "Set take-profit and/or stop-loss on an existing position. Auto-detects position side.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "tp": { "type": "number", "description": "Take-profit price (optional)" },
                "sl": { "type": "number", "description": "Stop-loss price (optional)" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_trade_cancel_tpsl",
        description: "Cancel take-profit and/or stop-loss on an existing position.",
        group: "trade",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "tp": { "type": "boolean", "description": "Cancel take-profit (default false)" },
                "sl": { "type": "boolean", "description": "Cancel stop-loss (default false)" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "acknowledged"],
            "additionalProperties": false
        }),
    },

    // ── Position ────────────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_position_list",
        description: "List all open positions across all markets.",
        group: "position",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_position_show",
        description: "Show detailed info for a specific position: PnL, margin, liquidation price, TP/SL.",
        group: "position",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_position_close",
        description: "Close an entire position via market order on the opposite side.",
        group: "position",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "acknowledged"],
            "additionalProperties": false
        }),
    },

    // ── Margin ──────────────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_margin_status",
        description: "Show current margin status: collateral, PnL, risk state, available to withdraw.",
        group: "margin",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_deposit",
        description: "Deposit USDC collateral into the trading account.",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "amount": { "type": "number", "description": "USDC amount to deposit" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["amount", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_withdraw",
        description: "Withdraw USDC collateral from the trading account.",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "amount": { "type": "number", "description": "USDC amount to withdraw" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["amount", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_transfer",
        description: "Transfer collateral between subaccounts (e.g., cross-margin to isolated).",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "from_subaccount": { "type": "integer", "description": "Source subaccount index (0 = cross-margin)" },
                "to_subaccount": { "type": "integer", "description": "Destination subaccount index" },
                "amount": { "type": "number", "description": "USDC amount to transfer" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["from_subaccount", "to_subaccount", "amount", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_transfer_child_to_parent",
        description: "Sweep all collateral from a child (isolated) subaccount back to cross-margin.",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "child_subaccount": { "type": "integer", "description": "Child subaccount index to sweep" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["child_subaccount", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_sync_parent_to_child",
        description: "Sync parent (cross-margin) state to a child (isolated) subaccount.",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "child_subaccount": { "type": "integer", "description": "Child subaccount index to sync to" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["child_subaccount", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_margin_leverage_tiers",
        description: "Show leverage tier schedule for a market: max leverage and max size per tier.",
        group: "margin",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" }
            },
            "required": ["symbol"],
            "additionalProperties": false
        }),
    },

    ToolDef {
        name: "vulcan_margin_add_collateral",
        description: "Add USDC collateral to an isolated position by symbol. Transfers from cross-margin to the isolated subaccount.",
        group: "margin",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol of the isolated position, e.g. SOL" },
                "amount": { "type": "number", "description": "USDC amount to add" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "amount", "acknowledged"],
            "additionalProperties": false
        }),
    },

    // ── Position (new) ─────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_position_reduce",
        description: "Reduce a position by a specified size via market order on the opposite side.",
        group: "position",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "size": { "type": "number", "description": "Size to reduce by in base lots" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "size", "acknowledged"],
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_position_tp_sl",
        description: "Attach take-profit and/or stop-loss bracket orders to an existing position.",
        group: "position",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Market symbol, e.g. SOL" },
                "tp": { "type": "number", "description": "Take-profit price" },
                "sl": { "type": "number", "description": "Stop-loss price" },
                "acknowledged": { "type": "boolean", "description": "Must be true to confirm this dangerous operation" }
            },
            "required": ["symbol", "acknowledged"],
            "additionalProperties": false
        }),
    },

    // ── History (read-only) ────────────────────────────────────────────
    ToolDef {
        name: "vulcan_history_trades",
        description: "Get past trade/fill history.",
        group: "history",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Filter by market symbol" },
                "limit": { "type": "integer", "description": "Max results to return", "default": 20 }
            },
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_history_orders",
        description: "Get past order history (filled, cancelled, expired).",
        group: "history",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Filter by market symbol" },
                "limit": { "type": "integer", "description": "Max results to return", "default": 20 }
            },
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_history_collateral",
        description: "Get collateral deposit/withdrawal history.",
        group: "history",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "description": "Max results to return", "default": 20 }
            },
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_history_funding",
        description: "Get funding payment history.",
        group: "history",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "Filter by market symbol" },
                "limit": { "type": "integer", "description": "Max results to return", "default": 20 }
            },
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_history_pnl",
        description: "Get PnL history over time.",
        group: "history",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "resolution": { "type": "string", "description": "Resolution: hourly or daily", "default": "hourly" },
                "limit": { "type": "integer", "description": "Max results to return", "default": 24 }
            },
            "additionalProperties": false
        }),
    },

    // ── Status ────────────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_status",
        description: "Health check: verify config, wallet, RPC, API, and trader registration status.",
        group: "status",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },

    // ── Wallet ────────────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_wallet_list",
        description: "List all stored wallets with names, public keys, and default status.",
        group: "wallet",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_wallet_balance",
        description: "Check SOL and USDC balance for a wallet.",
        group: "wallet",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Wallet name (omit for default wallet)" }
            },
            "additionalProperties": false
        }),
    },

    // ── Account ───────────────────────────────────────────────────────
    ToolDef {
        name: "vulcan_account_info",
        description: "Get trader account info: collateral, positions, risk state.",
        group: "account",
        dangerous: false,
        schema: || json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    },
    ToolDef {
        name: "vulcan_account_register",
        description: "Register a new trader account with an invite code.",
        group: "account",
        dangerous: true,
        schema: || json!({
            "type": "object",
            "properties": {
                "invite_code": { "type": "string", "description": "Invite code for registration" },
                "acknowledged": { "type": "boolean", "description": "Must be true to execute" }
            },
            "required": ["invite_code", "acknowledged"],
            "additionalProperties": false
        }),
    },
];

/// Filter tools by group. If groups is None, return all tools.
pub fn tools_for_groups(groups: &Option<Vec<String>>) -> Vec<&'static ToolDef> {
    match groups {
        None => TOOLS.iter().collect(),
        Some(gs) => TOOLS
            .iter()
            .filter(|t| gs.iter().any(|g| g == t.group))
            .collect(),
    }
}
