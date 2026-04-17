---
name: vulcan-risk-management
version: 1.0.0
description: "Pre-trade risk checks, leverage tiers, margin health thresholds, and when-to-warn rules."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-shared"]
---

# vulcan-risk-management

Use this skill for:
- Pre-trade risk assessment
- Monitoring margin health
- Understanding leverage tiers
- Deciding when to warn the user

## Pre-Trade Risk Checklist

Before every trade, call these tools:

```
1. vulcan_margin_status     → {}              # risk_state, collateral, PnL
2. vulcan_position_list     → {}              # existing positions
3. vulcan_trade_orders      → { symbol }      # resting orders consuming margin
4. vulcan_market_orderbook  → { symbol }      # slippage check for market orders
```

## Margin Health States

| State | Meaning | Action |
|-------|---------|--------|
| `Healthy` | Sufficient collateral | Safe to trade |
| `HighRisk` | Margin getting thin | Warn user before any new trades |
| `Liquidatable` | At risk of liquidation | Do NOT open new positions. Suggest reducing exposure or adding collateral |

## Leverage Tiers

Markets have tiered leverage limits. Larger positions get lower max leverage.

```
vulcan_margin_leverage_tiers → { symbol: "SOL" }
```

The first tier gives max leverage for typical sizes. Always check before proposing a trade.

## Funding Rate Awareness

```
vulcan_market_ticker → { symbol: "SOL" }    # check funding_rate field
```

- Positive rate: Longs pay shorts.
- Negative rate: Shorts pay longs.
- For longer-duration positions, factor funding costs into the trade thesis.

## Position Sizing

When the user doesn't specify exact size:
1. Ask their risk tolerance (USD or % of collateral).
2. Fetch `vulcan_market_info` for lot size conversion.
3. Calculate position size.
4. Present the calculation before executing.

## When to Warn

Alert the user when:
- Risk state is anything other than Healthy.
- A trade would use >50% of available margin.
- Liquidation price is within 10% of mark price.
- Funding rate is elevated (>0.01% per interval).
- Orderbook spread is wide (>10bps).
- They're about to increase an already-large position.

## Slippage Check

For market orders, check the orderbook:

```
vulcan_market_orderbook → { symbol: "SOL", depth: 10 }
```

If order size is large relative to available liquidity at the best levels, warn about potential slippage.

## Hard Rules

1. Never trade without user confirmation.
2. Never deposit or withdraw without user confirmation.
3. Always check margin before opening new positions.
4. Never exceed available margin.
5. Always report transaction signatures.
