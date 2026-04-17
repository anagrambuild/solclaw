---
name: vulcan-twap-execution
version: 1.0.0
description: "Execute large orders as time-weighted slices to reduce market impact on Phoenix DEX."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-trade-execution", "vulcan-lot-size-calculator"]
---

# vulcan-twap-execution

Use this skill for:
- Breaking a large order into smaller time-spaced slices
- Reducing market impact and slippage on size
- Executing over minutes or hours
- Tracking average fill price across slices

## Core Concept

Time-Weighted Average Price (TWAP) splits a large order into N equal slices executed at regular intervals. The goal is an average fill price close to the time-weighted market average, reducing the impact a single large order would have on the book.

Vulcan does not have a built-in scheduler — the agent manages the loop externally.

## Parameters

Agree on these with the user before starting:
- **Symbol**: e.g., SOL
- **Side**: buy or sell
- **Total size**: in tokens (agent converts to base lots)
- **Slices**: number of child orders (e.g., 5-10)
- **Interval**: time between slices (e.g., 60s, 300s)

## Pre-TWAP Checks

```
1. vulcan_market_info      → { symbol: "SOL" }    # base_lots_decimals, fees
2. vulcan_market_ticker    → { symbol: "SOL" }    # current price, volume
3. vulcan_market_orderbook → { symbol: "SOL" }    # depth — is there enough liquidity per slice?
4. vulcan_margin_status    → {}                    # enough collateral for total position?
5. vulcan_position_list    → {}                    # existing exposure
```

## Calculate Slice Size

```
total_base_lots = total_tokens * 10^base_lots_decimals
slice_lots = total_base_lots / slices    # round to integer
```

Example: 5 SOL over 5 slices, decimals=2:
```
total_base_lots = 5 * 100 = 500
slice_lots = 500 / 5 = 100 base lots per slice
```

Verify: `slice_lots * slices` should equal `total_base_lots`. Adjust last slice for remainder.

## Confirm with User

Present before starting:
- Total: 5 SOL (500 base lots) across 5 slices
- Per slice: 1 SOL (100 base lots)
- Interval: 60 seconds
- Estimated total fees: `total_base_lots * price * taker_fee * 2` (if round-trip)
- Get explicit approval to begin the TWAP.

## Market Order TWAP Loop

For each slice:

### 1. Check price hasn't moved beyond tolerance

```
vulcan_market_ticker → { symbol: "SOL" }
```

If price has moved >X% from the start price, pause and alert the user.

### 2. Execute slice

```
vulcan_trade_market_buy → { symbol: "SOL", size: 100, acknowledged: true }
```

### 3. Record fill details

Save: slice number, timestamp, fill price, tx signature.

### 4. Wait for interval

Wait the agreed interval before the next slice.

### 5. Repeat until all slices complete

## Limit-Order TWAP Variant

Use limit orders at the current best bid/ask for potentially better fills (maker fees):

### 1. Read current price

```
vulcan_market_ticker → { symbol: "SOL" }
```

### 2. Place limit order at or near the ask (for buys)

```
vulcan_trade_limit_buy → { symbol: "SOL", size: 100, price: <current_ask>, acknowledged: true }
```

### 3. Wait, then check fill status

```
vulcan_trade_orders → { symbol: "SOL" }
```

### 4. If unfilled after interval, cancel and place next slice

```
vulcan_trade_cancel → { symbol: "SOL", order_ids: ["<id>"], acknowledged: true }
```

Then adjust price and place next slice. Track unfilled volume to add to remaining slices.

## Tracking Average Fill

After all slices, compute the volume-weighted average price:

```
vulcan_position_show → { symbol: "SOL" }
```

The `entry_price` field reflects the average across all fills for the position.

For detailed per-slice tracking, the agent should maintain its own log of each slice's fill price and size.

## Handling Errors Mid-Loop

- **On `network` error**: Pause, retry the current slice after backoff.
- **On `tx_failed`**: Check position state before retrying. See `vulcan-error-recovery` skill.
- **On `rate_limit`**: Wait and retry. A 60s interval between slices should avoid rate limits.
- **Never skip a slice on error** — pause the loop, diagnose, then resume.

## Hard Rules

1. Each TWAP session requires human approval before the first slice.
2. In confirm-each mode, confirm each individual slice. In auto-execute mode, log every slice.
3. Track cumulative fill volume — stop if total exceeds target (handle partial fills from limit orders).
4. On any error, pause the loop rather than skipping the slice.
5. If price moves beyond user-defined tolerance, pause and alert.
6. Report all transaction signatures.
7. Present final summary: total filled, average price, total fees, time elapsed.
