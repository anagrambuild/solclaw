# Portfolio Overview Workflow

> **Note:** This workflow is superseded by the richer skill at `skills/vulcan-portfolio-intel/SKILL.md`. This file is kept for backward compatibility with existing MCP resource URIs.

Use this workflow to get a complete picture of the trading account state.

## Full Portfolio Snapshot

Run these in parallel for a complete view:

```
vulcan_margin_status   → {}     # collateral, PnL, risk state
vulcan_position_list   → {}     # all open positions
vulcan_trade_orders    → {}     # all open orders across markets
```

## Interpreting Margin Status

Key fields:
- **collateral_balance**: Total USDC deposited
- **unrealized_pnl**: Sum of PnL across all open positions
- **portfolio_value**: effective collateral + unrealized PnL
- **effective_collateral**: Collateral adjusted for risk
- **available_to_withdraw**: How much can be withdrawn without liquidation risk
- **risk_state**: Current risk level (`Healthy`, `HighRisk`, `Liquidatable`)
- **risk_tier**: Additional classification (`Safe`, etc.)
- **initial_margin**: Total margin currently used by open positions
- **maintenance_margin**: Minimum margin to avoid liquidation
- **num_positions** / **num_open_orders**: Quick counts

## Interpreting Positions

Key fields per position:
- **side**: Long or Short
- **size**: Position size (negative = short)
- **entry_price**: Average entry price
- **mark_price**: Current market price
- **unrealized_pnl**: Current profit/loss
- **liquidation_price**: Price at which position gets liquidated ("N/A" if unreachable)
- **maintenance_margin**: Margin required to keep position open

## Position Detail

For deeper analysis on a specific position:

```
vulcan_position_show → { symbol }
```

Additional fields:
- **take_profit_price / stop_loss_price**: Attached trigger orders (set via `tp`/`sl` on market orders). These do NOT appear in `vulcan_trade_orders` — only visible here.
- **unsettled_funding / accumulated_funding**: Funding payments
- **initial_margin**: Margin required to open at current size
- **position_value**: Notional value of position
- **discounted_unrealized_pnl**: PnL after risk discount (used for margin calculations)

Note: `collateral_balance` decreases with each trade due to taker/maker fees, even if positions are profitable. A round-trip trade costs 2× taker fee on the notional value.

## Presenting to the User

When asked for portfolio status, summarize:

1. **Account health**: Risk state, margin utilization (initial_margin / collateral_balance)
2. **Position summary**: For each position — symbol, side, size, entry vs mark, PnL
3. **Open orders**: Any resting limit orders
4. **Available capital**: Collateral available for new positions or withdrawal

## Closing Positions

```
vulcan_position_close → { symbol, acknowledged: true }
```

This places a market order to fully close the position. Always confirm with the user first and check the orderbook for slippage on larger positions.
