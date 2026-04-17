# Risk Management Rules

> **Note:** This workflow is superseded by the richer skill at `skills/vulcan-risk-management/SKILL.md`. This file is kept for backward compatibility with existing MCP resource URIs.

Guardrails for AI agents trading on Phoenix DEX via Vulcan.

## Hard Rules

1. **Never trade without user confirmation.** Present the full trade details and wait for explicit approval.
2. **Never deposit or withdraw without user confirmation.**
3. **Always check margin before opening new positions.** If margin status shows HighRisk or worse, warn the user before any new trades.
4. **Never exceed available margin.** Calculate required margin from leverage tiers before proposing a trade size.
5. **Always report transaction signatures.** Every on-chain action returns a tx signature — share it with the user.

## Pre-Trade Risk Checks

Before every trade, verify:

1. **Margin sufficiency**: `vulcan_margin_status` → ensure risk_state is Healthy
2. **Position awareness**: `vulcan_position_list` → know what's already open
3. **Order awareness**: `vulcan_trade_orders` → know what's resting on the book
4. **Slippage check**: For market orders, check `vulcan_market_orderbook` — if the order size is large relative to available liquidity at the best levels, warn the user about potential slippage

## Leverage Tiers

Markets have tiered leverage limits — larger positions get lower max leverage. Always check `vulcan_market_info` to find the applicable tier for the proposed position size.

The first tier in `leverage_tiers` gives you the max leverage for normal-sized positions. Subsequent tiers apply to progressively larger positions. Tiers are configured by the exchange and subject to change — always fetch fresh values rather than caching them.

## Funding Rate Awareness

Perpetual futures charge/pay funding periodically. Check `vulcan_market_ticker` for the current funding rate:
- **Positive rate**: Longs pay shorts
- **Negative rate**: Shorts pay longs

For longer-duration positions, factor funding costs into the trade thesis.

## Position Sizing Guidelines

When the user doesn't specify an exact size:
1. Ask them how much risk they want to take (in USD terms or as % of collateral)
2. Fetch market info for lot size conversion
3. Calculate position size based on their risk tolerance
4. Present the calculation before executing

## When to Warn

Alert the user when:
- Risk state is anything other than Healthy
- A trade would use >50% of available margin
- Liquidation price is within 10% of current mark price
- Funding rate is elevated (>0.01% per interval)
- Orderbook spread is wide (>10bps)
- They're about to increase an already-large position
