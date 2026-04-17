---
name: recipe-open-hedged-position
version: 1.0.0
description: "Open a position with TP/SL protection in one complete flow."
metadata:
  openclaw:
    category: "recipe"
    domain: "trading"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-trade-execution", "vulcan-lot-size-calculator", "vulcan-tpsl-management"]
---

# Open Hedged Position

> **PREREQUISITE:** Load `vulcan-trade-execution`, `vulcan-lot-size-calculator`, and `vulcan-tpsl-management` skills.

Open a position with take-profit and stop-loss protection in one complete flow.

> **CAUTION:** Live orders spend real money. Confirm with the user before executing.

## Steps

1. Get market configuration:
   ```
   vulcan_market_info → { symbol: "SOL" }
   ```
   Extract `base_lots_decimals` for size calculation.

2. Get current price:
   ```
   vulcan_market_ticker → { symbol: "SOL" }
   ```

3. Check margin:
   ```
   vulcan_margin_status → {}
   ```
   Ensure risk_state is Healthy and sufficient collateral is available.

4. Check orderbook for slippage:
   ```
   vulcan_market_orderbook → { symbol: "SOL", depth: 10 }
   ```

5. Calculate lot size:
   ```
   base_lots = desired_tokens * 10^base_lots_decimals
   ```

6. Calculate TP/SL levels based on user's risk/reward ratio.

7. Confirm with user: symbol, direction, size, TP, SL, estimated fees.

8. Execute with TP/SL attached:
   ```
   vulcan_trade_market_buy → {
     symbol: "SOL",
     size: 50,
     tp: 160.0,
     sl: 140.0,
     acknowledged: true
   }
   ```

9. Verify position:
   ```
   vulcan_position_show → { symbol: "SOL" }
   ```
   Confirm: position opened, TP/SL attached (check take_profit_price, stop_loss_price).

10. Report transaction signature.
