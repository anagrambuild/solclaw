---
name: vulcan-grid-trading
version: 1.0.0
description: "Grid trading with layered limit orders across a price range on Phoenix DEX perpetuals."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-trade-execution", "vulcan-lot-size-calculator", "vulcan-risk-management"]
---

# vulcan-grid-trading

Use this skill for:
- Placing a grid of limit orders across a price range
- Profiting from sideways/ranging markets on perpetual futures
- Managing grid state (filled orders, replacements)
- Running a market-making-like strategy

## Core Concept

Grid trading places buy limit orders below the current price and sell limit orders above it at fixed intervals. When a buy fills, a corresponding sell is placed one grid level higher. When a sell fills, a corresponding buy is placed one grid level lower. Profit comes from capturing the spread at each level.

On perpetual futures (Phoenix DEX), this means opening and closing positions at grid levels. Funding rate costs/income also factor into profitability.

## Grid Parameters

Define with the user before starting:
- **Symbol**: e.g., SOL
- **Price range**: lower bound to upper bound (e.g., 140–160)
- **Grid levels**: number of orders per side (e.g., 5 buy + 5 sell = 10 total)
- **Size per level**: in tokens (agent converts to base lots)

Grid spacing = (upper - lower) / total_levels

## Pre-Grid Checks

```
1. vulcan_market_info      → { symbol: "SOL" }    # base_lots_decimals, tick_size, fees
2. vulcan_market_ticker    → { symbol: "SOL" }    # current price (center the grid)
3. vulcan_market_orderbook → { symbol: "SOL" }    # spread, depth
4. vulcan_margin_status    → {}                    # enough collateral for worst case?
5. vulcan_position_list    → {}                    # existing positions in this market
```

## Calculate Grid Levels

```
spacing = (upper_bound - lower_bound) / total_levels
```

Example: Range 140–160, 10 levels, current price 150:
```
spacing = (160 - 140) / 10 = 2.0
Buy levels:  148, 146, 144, 142, 140
Sell levels: 152, 154, 156, 158, 160
```

Ensure all prices are valid multiples of `tick_size` from `vulcan_market_info`.

## Calculate Size Per Level

```
size_per_level_lots = desired_tokens_per_level * 10^base_lots_decimals
```

## Margin Estimation

Worst case: all buy orders fill (max long position) or all sell orders fill (max short position). Calculate margin required:

```
max_position_lots = size_per_level_lots * levels_per_side
```

Check this against leverage tiers and available collateral.

## Confirm with User

Present the full grid before placing:
- Price range, grid levels, spacing
- Size per level (base lots + token equivalent)
- Total margin required (worst case)
- Estimated fees per round-trip
- Funding rate exposure
- Get explicit approval for the entire grid.

## Place the Grid

Use `vulcan_trade_multi_limit` to place all grid orders in a single transaction. This is much faster than placing orders individually.

```
vulcan_trade_multi_limit → {
  symbol: "SOL",
  bids: [
    { price: 148.00, size: 50 },
    { price: 146.00, size: 50 },
    { price: 144.00, size: 50 },
    { price: 142.00, size: 50 },
    { price: 140.00, size: 50 }
  ],
  asks: [
    { price: 152.00, size: 50 },
    { price: 154.00, size: 50 },
    { price: 156.00, size: 50 },
    { price: 158.00, size: 50 },
    { price: 160.00, size: 50 }
  ],
  slide: false,
  acknowledged: true
}
```

### Verify all orders placed

```
vulcan_trade_orders → { symbol: "SOL" }
```

## Grid Maintenance Loop

Periodically check for fills and replace completed orders:

### 1. Check open orders

```
vulcan_trade_orders → { symbol: "SOL" }
```

Compare against the expected grid. Missing orders = filled.

### 2. Check position

```
vulcan_position_show → { symbol: "SOL" }
```

### 3. Replace filled orders

- For each filled **buy** at price P: queue a **sell** at P + spacing.
- For each filled **sell** at price P: queue a **buy** at P - spacing.

Batch all replacement orders into a single `vulcan_trade_multi_limit` call:

```
vulcan_trade_multi_limit → {
  symbol: "SOL",
  bids: [{ price: <P - spacing>, size: 50 }, ...],
  asks: [{ price: <P + spacing>, size: 50 }, ...],
  slide: false,
  acknowledged: true
}
```

### 4. Check margin health

```
vulcan_margin_status → {}
```

If risk_state is not Healthy, pause grid maintenance and alert user.

### 5. Repeat at regular intervals

Suggested check interval: 30-60 seconds.

## Grid Shutdown

Cancel all grid orders:

```
vulcan_trade_cancel_all → { symbol: "SOL", acknowledged: true }
```

Then optionally close any remaining position:

```
vulcan_position_close → { symbol: "SOL", acknowledged: true }
```

## Risk Considerations

- **Trending markets**: Grid trading profits in ranging markets but loses in strong trends. If price drops below the entire grid, you accumulate a large long position at a loss. If price rises above, you're fully short.
- **Funding rates**: On perpetuals, holding a position incurs funding payments. Check `vulcan_market_ticker` for the funding rate — a high funding rate can erode grid profits.
- **Margin**: All resting limit orders consume margin. A wide grid with many levels can lock up significant collateral.
- **Slippage on replacement**: Replacement orders may not fill at exactly the grid level if the market moves fast.

## Hard Rules

1. Never place a live grid without explicit user approval for the full grid plan.
2. Always dry-run the grid math and present to user before placing.
3. Check margin status before placing and during maintenance.
4. Cancel the entire grid before adjusting parameters — never leave orphaned orders.
5. Track total grid P&L (sum of all fill spreads minus fees and funding).
6. Set price boundaries — if price moves outside the grid range, pause and alert.
7. Report all transaction signatures.
