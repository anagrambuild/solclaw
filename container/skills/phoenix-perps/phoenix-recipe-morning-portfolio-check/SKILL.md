---
name: recipe-morning-portfolio-check
version: 1.0.0
description: "Daily portfolio review with margin, positions, orders, and funding rates."
metadata:
  openclaw:
    category: "recipe"
    domain: "portfolio"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-portfolio-intel", "vulcan-market-intel"]
---

# Morning Portfolio Check

> **PREREQUISITE:** Load `vulcan-portfolio-intel` and `vulcan-market-intel` skills.

Daily portfolio review — read-only, no trades.

## Steps

1. Get account health:
   ```
   vulcan_margin_status → {}
   ```

2. Get all positions:
   ```
   vulcan_position_list → {}
   ```

3. Get all resting orders:
   ```
   vulcan_trade_orders → {}
   ```

4. For each position, check funding rate:
   ```
   vulcan_market_ticker → { symbol: "SOL" }
   vulcan_market_ticker → { symbol: "BTC" }
   # ... for each held position
   ```

5. For each position, check TP/SL status:
   ```
   vulcan_position_show → { symbol: "SOL" }
   # Check take_profit_price and stop_loss_price
   ```

6. Present summary to user:
   - Account: risk state, total collateral, total PnL, available to withdraw.
   - Each position: symbol, side, size, entry, mark, PnL, liquidation price, TP/SL.
   - Funding exposure: which positions are paying/receiving funding.
   - Resting orders: any limit orders on the book.
   - Warnings: positions near liquidation, elevated funding rates, wide spreads.
