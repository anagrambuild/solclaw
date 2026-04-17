---
name: recipe-close-and-withdraw
version: 1.0.0
description: "Close all positions and withdraw collateral to wallet."
metadata:
  openclaw:
    category: "recipe"
    domain: "portfolio"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-position-management", "vulcan-margin-operations"]
---

# Close and Withdraw

> **PREREQUISITE:** Load `vulcan-position-management` and `vulcan-margin-operations` skills.

Close all positions, cancel all orders, and withdraw collateral to the wallet.

> **CAUTION:** This exits all positions at market price and withdraws funds. Irreversible.

## Steps

1. Check current state:
   ```
   vulcan_position_list → {}
   vulcan_trade_orders  → {}
   vulcan_margin_status → {}
   ```

2. Cancel all resting orders for each market:
   ```
   vulcan_trade_cancel_all → { symbol: "SOL", acknowledged: true }
   # ... repeat for each market with open orders
   ```

3. Close each position:
   ```
   vulcan_position_close → { symbol: "SOL", acknowledged: true }
   # ... repeat for each open position
   ```

4. Verify flat:
   ```
   vulcan_position_list → {}    # should be empty
   vulcan_trade_orders  → {}    # should be empty
   ```

5. Check available to withdraw:
   ```
   vulcan_margin_status → {}
   ```
   Note the `available_to_withdraw` amount.

6. Withdraw collateral:
   ```
   vulcan_margin_withdraw → { amount: <available_to_withdraw>, acknowledged: true }
   ```

7. Verify withdrawal:
   ```
   vulcan_wallet_balance → {}       # USDC should have increased
   vulcan_margin_status  → {}       # collateral should be ~0
   ```

8. Report all transaction signatures.
