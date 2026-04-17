---
name: vulcan-shared
version: 1.0.0
description: "Shared runtime contract for vulcan: auth, invocation, symbol format, size units, and safety."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
---

# vulcan-shared

**This tool is experimental. Commands execute real financial transactions on Solana mainnet. Test with `--dry-run` before using real funds.**

## Invocation Contract

### MCP (preferred)

Tools are named `vulcan_<group>_<action>`. Call them directly via MCP tool calls. Dangerous tools require `acknowledged: true`.

### CLI (fallback)

```bash
vulcan <command> [args...] -o json
```

- Parse `stdout` only (JSON).
- Treat `stderr` as diagnostics.
- Exit code `0` = success.
- Non-zero = failure with JSON error envelope in stdout.

## Authentication

MCP server unlocks wallet at startup via `VULCAN_WALLET_PASSWORD` env var. No per-call prompts.

For CLI: `export VULCAN_WALLET_PASSWORD=your_password`

## Symbol Format

Uppercase ticker only: `SOL`, `BTC`, `ETH`, `DOGE`, `SUI`, `XRP`, `BNB`, `AAVE`, `ZEC`, `HYPE`, `SKR`.

No `-PERP` suffix. Run `vulcan_market_list` to discover active markets.

## Size Units — Base Lots

The `size` parameter is in **base lots**, not tokens or USD. Always call `vulcan_market_info` first.

**Conversion**: `base_lots = desired_tokens * 10^base_lots_decimals`

## Error Routing

Route on `.error.category`:
- `validation` — Fix inputs, do not retry.
- `auth` — Check wallet/password.
- `network` — Retry with exponential backoff.
- `tx_failed` — **Verify state before retrying.** Never blind-retry on-chain tx.
- `dangerous_gate` — Set `acknowledged: true`.

## Safety

Require explicit human approval before:
- Buy or sell orders (market and limit)
- Order cancellations
- Position close or reduce
- Deposits, withdrawals, and transfers
- TP/SL changes
- Account registration

Hard rules:
1. Always call `vulcan_market_info` before trading.
2. Always call `vulcan_margin_status` before opening positions.
3. Always call `vulcan_position_list` before trading.
4. Never guess lot sizes.
5. Report all transaction signatures.
