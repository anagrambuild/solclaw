---
name: vulcan-tpsl-management
version: 1.0.0
description: "Take-profit and stop-loss: direction rules, constraints, and set/cancel flows."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
    skills: ["vulcan-shared"]
---

# vulcan-tpsl-management

Use this skill for:
- Setting TP/SL when opening a position
- Attaching TP/SL to an existing position
- Modifying or cancelling TP/SL
- Understanding TP/SL constraints and gotchas

## Direction Rules

### Long positions (buy)

- Take-profit price MUST be **above** entry price.
- Stop-loss price MUST be **below** entry price.

### Short positions (sell)

- Take-profit price MUST be **below** entry price.
- Stop-loss price MUST be **above** entry price.

## Setting TP/SL at Order Time

Attach to market orders:

```
vulcan_trade_market_buy → {
  symbol: "SOL", size: 50,
  tp: 160.0, sl: 140.0,
  acknowledged: true
}
```

**Critical constraint**: TP/SL at order time only works when **opening or extending** a position. If the market order **reduces** an existing position, the entire transaction rolls back (market order does not execute either).

## Setting TP/SL on Existing Position

### Method 1: Trade tool

```
vulcan_trade_set_tpsl → { symbol: "SOL", tp: 160.0, sl: 140.0, acknowledged: true }
```

### Method 2: Position tool

```
vulcan_position_tp_sl → { symbol: "SOL", tp: 160.0, sl: 140.0, acknowledged: true }
```

Both auto-detect position side. You can set just TP, just SL, or both.

## Modifying TP/SL

To change existing TP/SL, call set again with new values. The new values replace the old ones:

```
vulcan_trade_set_tpsl → { symbol: "SOL", tp: 165.0, acknowledged: true }
```

## Cancelling TP/SL

```
vulcan_trade_cancel_tpsl → { symbol: "SOL", tp: true, sl: true, acknowledged: true }
```

Set `tp: true` to cancel take-profit, `sl: true` to cancel stop-loss, or both.

## Viewing TP/SL

TP/SL are **trigger orders** — they appear in `vulcan_position_show`, NOT in `vulcan_trade_orders`.

```
vulcan_position_show → { symbol: "SOL" }
# Look for: take_profit_price, stop_loss_price
```

## Common Mistakes

1. **Wrong direction**: TP must be on the profitable side. For longs, TP > entry. For shorts, TP < entry.
2. **Setting on a reduce order**: TP/SL fails if the market order reduces a position. Use `vulcan_trade_set_tpsl` on the existing position instead.
3. **Looking for TP/SL in orders**: They won't appear in `vulcan_trade_orders`. Check `vulcan_position_show`.
