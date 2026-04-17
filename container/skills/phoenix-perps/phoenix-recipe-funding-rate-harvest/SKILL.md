---
name: recipe-funding-rate-harvest
version: 1.0.0
description: "Scan markets for favorable funding rates and open positions to capture funding."
metadata:
  openclaw:
    category: "recipe"
    domain: "strategy"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-market-intel", "vulcan-trade-execution"]
---

# Funding Rate Harvest

> **PREREQUISITE:** Load `vulcan-market-intel` and `vulcan-trade-execution` skills.

Scan perpetual markets for attractive funding rates and open positions to earn funding payments.

> **CAUTION:** Funding rate strategies carry directional risk. The position PnL may exceed funding income.

## Steps

1. List all markets:
   ```
   vulcan_market_list → {}
   ```

2. Get ticker for each market to check funding rates:
   ```
   vulcan_market_ticker → { symbol: "SOL" }
   vulcan_market_ticker → { symbol: "BTC" }
   vulcan_market_ticker → { symbol: "ETH" }
   # ... for each active market
   ```

3. Identify favorable rates:
   - **Positive funding rate** → shorts receive payment → consider short.
   - **Negative funding rate** → longs receive payment → consider long.
   - Look for rates > 0.01% per interval for meaningful income.

4. For the best opportunity, check market conditions:
   ```
   vulcan_market_info      → { symbol }     # fees, leverage tiers
   vulcan_market_orderbook → { symbol }     # spread, depth
   ```

5. Check margin:
   ```
   vulcan_margin_status → {}
   ```

6. Calculate position size accounting for:
   - Expected funding income vs taker fees (round-trip cost).
   - Leverage tier limits.
   - Acceptable directional risk.

7. Present analysis to user: funding rate, estimated daily income, entry cost (fees + spread), break-even time.

8. Execute with user approval:
   ```
   vulcan_trade_market_sell → { symbol: "SOL", size: <lots>, acknowledged: true }
   ```
   (Short for positive funding rate, long for negative.)

9. Verify position:
   ```
   vulcan_position_show → { symbol }
   ```
