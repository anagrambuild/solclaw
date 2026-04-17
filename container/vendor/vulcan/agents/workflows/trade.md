# Trade Workflow

> **Note:** This workflow is superseded by the richer skill at `skills/vulcan-trade-execution/SKILL.md`. This file is kept for backward compatibility with existing MCP resource URIs.

Follow this checklist before placing any order on Phoenix DEX.

## Pre-Trade Checklist

### 1. Gather Market Context

```
vulcan_market_info     → { symbol }         # lot sizes, fees, leverage tiers
vulcan_market_ticker   → { symbol }         # current price, funding rate, 24h volume
vulcan_margin_status   → {}                 # available collateral
vulcan_position_list   → {}                 # existing positions
vulcan_trade_orders    → { symbol }         # existing open orders
```

### 2. Calculate Size

From `vulcan_market_info`, extract:
- `base_lots_decimals` (e.g., 2 means 1 base lot = 0.01 tokens)
- `taker_fee` for market orders, `maker_fee` for limit orders
- `tick_size` — minimum price increment for limit orders (price must be a multiple of this in the on-chain representation)

To convert a desired token amount to base lots:
```
base_lots = desired_tokens × 10^base_lots_decimals
```

Examples:
- Want 0.5 SOL, base_lots_decimals = 2 → 0.5 × 100 = 50 base lots
- Want 0.01 ETH, base_lots_decimals = 3 → 0.01 × 1000 = 10 base lots
- Want 0.001 BTC, base_lots_decimals = 4 → 0.001 × 10000 = 10 base lots

### 3. Validate Against Constraints

- **Margin**: Ensure collateral covers initial margin for the position size at your leverage tier.
- **Leverage tier**: Check `leverage_tiers` — larger positions have lower max leverage.
- **Existing exposure**: Account for open positions. Same-side orders increase exposure; opposite-side orders reduce it.
- **Limit order margin**: Resting limit orders consume margin even before they fill. Factor this into available margin calculations when placing multiple orders.

### 4. Confirm With User

Before executing, present:
- Symbol and direction (buy/sell)
- Size in both base lots AND approximate token amount
- Order type (market/limit) and price if limit
- Estimated fees
- Current mark price for reference
- Any existing positions in this market

**In confirm-each mode** (default): Wait for explicit user approval before executing.

**In auto-execute mode**: Log the trade details, then execute immediately without waiting. The user has already granted session-wide permission. Still report results and signatures after execution.

## Market Orders

```
vulcan_trade_market_buy  → { symbol, size, acknowledged: true }
vulcan_trade_market_sell → { symbol, size, acknowledged: true }
```

Market orders fill immediately at best available price. Check the orderbook first to estimate slippage for larger sizes.

Note: Taker fees are deducted from collateral on each fill. A round-trip (open + close) costs 2× taker fee on the notional value.

### Take-Profit / Stop-Loss (TP/SL)

Attach TP and/or SL to market orders using optional `tp` and `sl` parameters:

```
vulcan_trade_market_buy  → { symbol, size, tp: 100.0, sl: 90.0, acknowledged: true }
vulcan_trade_market_sell → { symbol, size, tp: 650.0, sl: 690.0, acknowledged: true }
```

**Rules:**
- **Long positions** (market buy): TP must be above entry, SL must be below entry.
- **Short positions** (market sell): TP must be below entry, SL must be above entry.
- You can set just TP, just SL, or both.
- TP/SL are **only available on market orders**, not limit orders.
- TP/SL can only be set when **opening or extending** a position. They will fail if the market order *reduces* an existing position (the entire transaction rolls back — the market order won't execute either).
- TP/SL are trigger orders — they show in `vulcan_position_show` as `take_profit_price` / `stop_loss_price`, **not** in `vulcan_trade_orders`.

## Limit Orders

```
vulcan_trade_limit_buy  → { symbol, size, price, acknowledged: true }
vulcan_trade_limit_sell → { symbol, size, price, acknowledged: true }
```

Limit orders rest on the book until filled or cancelled. They pay maker fees (often lower or negative/rebate). TP/SL is not available on limit orders.

## After Placing an Order

1. Report the transaction signature to the user.
2. For limit orders, check `vulcan_trade_orders` to confirm the order is on the book.
3. For market orders, check `vulcan_position_list` to confirm the position was opened/modified.

## Cancelling Orders

```
vulcan_trade_orders     → { symbol }                              # get order IDs
vulcan_trade_cancel     → { symbol, order_ids: [...], acknowledged: true }  # cancel specific
vulcan_trade_cancel_all → { symbol, acknowledged: true }          # cancel all for market
```
