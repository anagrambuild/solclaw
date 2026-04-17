---
name: recipe-emergency-flatten
version: 1.0.0
description: "Cancel all orders and close all positions across all markets."
metadata:
  openclaw:
    category: "recipe"
    domain: "risk"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-risk-management", "vulcan-position-management"]
---

# Emergency Flatten

> **PREREQUISITE:** Load `vulcan-risk-management` and `vulcan-position-management` skills.

Cancel all resting orders and close all open positions. Use when margin health is critical or the user wants to exit everything immediately.

> **CAUTION:** This executes multiple real transactions. Each step is irreversible.

## Steps

1. Check current state:
   ```
   vulcan_margin_status → {}
   vulcan_position_list → {}
   vulcan_trade_orders  → {}
   ```

2. Cancel all orders for each market with open orders:
   ```
   vulcan_trade_cancel_all → { symbol: "SOL", acknowledged: true }
   vulcan_trade_cancel_all → { symbol: "BTC", acknowledged: true }
   # ... repeat for each market
   ```

3. Close each open position:
   ```
   vulcan_position_close → { symbol: "SOL", acknowledged: true }
   vulcan_position_close → { symbol: "BTC", acknowledged: true }
   # ... repeat for each position
   ```

4. Verify everything is flat:
   ```
   vulcan_position_list → {}    # should be empty
   vulcan_trade_orders  → {}    # should be empty
   vulcan_margin_status → {}    # all collateral should be available
   ```

5. Report all transaction signatures to the user.
