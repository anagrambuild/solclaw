---
name: recipe-scale-into-position
version: 1.0.0
description: "Add to an existing position in calculated increments."
metadata:
  openclaw:
    category: "recipe"
    domain: "trading"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-trade-execution", "vulcan-lot-size-calculator", "vulcan-risk-management"]
---

# Scale Into Position

> **PREREQUISITE:** Load `vulcan-trade-execution`, `vulcan-lot-size-calculator`, and `vulcan-risk-management` skills.

Add to an existing position in calculated increments.

> **CAUTION:** Each increment is a real transaction. Confirm with user before each step.

## Steps

1. Check existing position:
   ```
   vulcan_position_show → { symbol: "SOL" }
   ```
   Note current size, entry price, and margin usage.

2. Check margin availability:
   ```
   vulcan_margin_status → {}
   ```
   Ensure sufficient collateral for the additional size.

3. Get market info for lot calculation:
   ```
   vulcan_market_info → { symbol: "SOL" }
   vulcan_market_ticker → { symbol: "SOL" }
   ```

4. Calculate increment size:
   ```
   increment_lots = desired_increment_tokens * 10^base_lots_decimals
   ```

5. Check leverage tier — ensure total position (existing + increment) doesn't exceed max leverage.

6. Confirm with user: current position, proposed addition, new total, margin impact.

7. Execute:
   ```
   vulcan_trade_market_buy → { symbol: "SOL", size: <increment_lots>, acknowledged: true }
   ```

8. Verify updated position:
   ```
   vulcan_position_show → { symbol: "SOL" }
   ```
   Confirm new size = old size + increment.

9. Report transaction signature.

Repeat steps 2-9 for each additional increment.
