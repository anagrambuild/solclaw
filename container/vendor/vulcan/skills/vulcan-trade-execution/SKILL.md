---
name: vulcan-trade-execution
version: 1.0.0
description: "Execute perpetual futures orders with pre-trade checks and post-trade verification."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-shared", "vulcan-lot-size-calculator"]
---

# vulcan-trade-execution

Use this skill for:
- Placing market or limit orders on Phoenix DEX
- Attaching TP/SL to new orders
- Cancelling orders
- The complete safe order flow

## Safe Market Order Flow

### 1. Gather market context

```
vulcan_market_info   → { symbol: "SOL" }     # lot sizes, fees, leverage tiers
vulcan_market_ticker → { symbol: "SOL" }     # current price, funding rate
vulcan_margin_status → {}                     # available collateral, risk state
vulcan_position_list → {}                     # existing positions
vulcan_trade_orders  → { symbol: "SOL" }     # existing resting orders
```

### 2. Calculate size

From `vulcan_market_info`, extract `base_lots_decimals`:
```
base_lots = desired_tokens * 10^base_lots_decimals
```
Example: Want 0.5 SOL, decimals=2 → 0.5 * 100 = 50 base lots.

### 3. Validate against constraints

- Ensure `vulcan_margin_status` shows risk_state = Healthy.
- Check leverage tiers — larger positions have lower max leverage.
- Factor in existing positions (same-side increases exposure, opposite-side reduces).

### 4. Confirm with user

Present: symbol, direction, size (base lots + token equivalent), order type, estimated fees, mark price, existing positions.

### 5. Execute

```
vulcan_trade_market_buy → { symbol: "SOL", size: 50, acknowledged: true }
```

### 6. Verify

```
vulcan_position_list → {}    # confirm position opened
```

Report the transaction signature to the user.

## Market Order with TP/SL

Attach take-profit and/or stop-loss at order time:

```
vulcan_trade_market_buy → {
  symbol: "SOL",
  size: 50,
  tp: 160.0,
  sl: 140.0,
  acknowledged: true
}
```

**Direction rules:**
- Long (buy): TP must be above entry, SL must be below entry.
- Short (sell): TP must be below entry, SL must be above entry.

**Constraints:**
- TP/SL only works when opening or extending a position. Fails if the order reduces a position (entire tx rolls back).
- TP/SL shows in `vulcan_position_show`, NOT in `vulcan_trade_orders`.

## Limit Orders

```
vulcan_trade_limit_buy → {
  symbol: "SOL",
  size: 50,
  price: 145.00,
  acknowledged: true
}
```

Limit orders rest on the book. They pay maker fees (typically lower). After placing, verify with:

```
vulcan_trade_orders → { symbol: "SOL" }    # confirm order on book
```

## Isolated Margin Orders

For markets requiring isolated margin, or when you want dedicated collateral:

```
vulcan_trade_market_buy → {
  symbol: "SOL",
  size: 50,
  isolated: true,
  collateral: 100.0,
  acknowledged: true
}
```

## Reduce-Only Orders

To ensure an order only reduces (never increases) a position:

```
vulcan_trade_market_sell → {
  symbol: "SOL",
  size: 25,
  reduce_only: true,
  acknowledged: true
}
```

## Cancel Orders

```
vulcan_trade_orders     → { symbol: "SOL" }                              # get order IDs
vulcan_trade_cancel     → { symbol: "SOL", order_ids: ["id1"], acknowledged: true }
vulcan_trade_cancel_all → { symbol: "SOL", acknowledged: true }          # cancel all
```

## Hard Rules

- Never execute orders without explicit user approval (unless in auto-execute mode).
- Route failures by `.error.category`.
- On `tx_failed`, check position state before retrying.
