---
name: vulcan-portfolio-intel
version: 1.0.0
description: "Full portfolio snapshot: margin status, positions, orders, and funding rate awareness."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-shared"]
---

# vulcan-portfolio-intel

Use this skill for:
- Daily portfolio reviews
- Presenting account status to the user
- Monitoring margin health and PnL
- Understanding funding exposure

## Portfolio Snapshot

Run these calls (they can be called in parallel):

```
vulcan_margin_status  → {}     # collateral, total PnL, risk state, available to withdraw
vulcan_position_list  → {}     # all open positions with unrealized PnL
vulcan_trade_orders   → {}     # all resting limit orders
```

## Interpreting Margin Status

Key fields:
- `collateral` — Total USDC deposited.
- `total_unrealized_pnl` — Combined PnL across all positions.
- `risk_state` — Healthy, HighRisk, or Liquidatable.
- `available_to_withdraw` — USDC that can be withdrawn without affecting positions.
- `initial_margin_used` — Margin locked by open positions and orders.

## Interpreting Positions

Key fields per position:
- `symbol`, `side` (Long/Short), `size` — What you hold.
- `entry_price`, `mark_price` — Where you entered vs current price.
- `unrealized_pnl` — Current profit/loss.
- `liquidation_price` — Price at which position gets liquidated.

## Interpreting Orders

Key fields per order:
- `symbol`, `side`, `order_type` — What's resting.
- `size`, `price` — Order parameters.
- `filled` — How much has filled so far.

Note: Resting limit orders consume margin even before filling.

## Funding Rate Check

For each open position, check funding exposure:

```
vulcan_market_ticker → { symbol }    # funding_rate field
```

- Positive rate: Longs pay shorts (costs you money if long).
- Negative rate: Shorts pay longs (costs you money if short).

## Presenting to User

Summarize:
1. Account health (risk state, collateral, total PnL).
2. Each position: symbol, side, size, entry, mark, PnL, liquidation price.
3. Resting orders: symbol, side, type, size, price.
4. Funding rate exposure for held positions.
5. Available to withdraw.
