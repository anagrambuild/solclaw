# Vulcan Runtime Context for AI Agents

**This is experimental software. Commands interact with the live Phoenix DEX on Solana and can result in real financial transactions. The user who deploys this tool is responsible for all outcomes.**

This file is optimized for runtime agent use. It defines how to call `vulcan` safely and reliably.

## Core Invocation Contract

### MCP (preferred)

Tools are named `vulcan_<group>_<action>`. Dangerous tools require `acknowledged: true`.

### CLI (fallback)

```bash
vulcan <command> [args...] -o json
```

- `stdout` is the only machine data channel (JSON).
- `stderr` is diagnostics only.
- Exit code `0` = success, non-zero = failure with JSON error envelope in stdout.

## Authentication

MCP server unlocks the wallet once at startup — no per-call prompts:

```json
{
  "mcpServers": {
    "vulcan": {
      "command": "vulcan",
      "args": ["mcp", "--allow-dangerous"],
      "env": { "VULCAN_WALLET_PASSWORD": "your_password" }
    }
  }
}
```

For CLI: `export VULCAN_WALLET_PASSWORD=your_password`

Without `--allow-dangerous`, only read-only market data tools are available.

## Symbol Format

Uppercase ticker: `SOL`, `BTC`, `ETH`, `DOGE`, `SUI`, `XRP`, `BNB`, `AAVE`, `ZEC`, `HYPE`, `SKR`. No `-PERP` suffix. Use `vulcan_market_list` to discover active markets.

## Size Units — Base Lots

The `size` parameter is in **base lots**, not tokens or USD. Call `vulcan_market_info` first to get `base_lots_decimals`.

**Conversion**: `base_lots = desired_tokens * 10^base_lots_decimals`

Examples: 0.5 SOL at decimals=2 = 50 base lots. 0.001 BTC at decimals=4 = 10 base lots.

## Safety Rules

1. All dangerous operations require `acknowledged: true` (MCP) or `--yes` (CLI).
2. Always call `vulcan_market_info` before trading — never guess lot sizes.
3. Always call `vulcan_margin_status` before opening positions — ensure sufficient collateral.
4. Always call `vulcan_position_list` before trading — know existing exposure.
5. Report all transaction signatures to the user.

## Error Handling Contract

On failure, parse the error envelope:

```json
{
  "ok": false,
  "error": {
    "category": "validation",
    "code": "UNKNOWN_MARKET",
    "message": "Market not found",
    "retryable": false
  }
}
```

Route on `.error.category`:
- `validation` — Fix inputs, do not retry unchanged request.
- `auth` — Check wallet and password.
- `config` — Run `vulcan setup`.
- `api` — Phoenix API issue, inspect message.
- `network` — Transient, retry with exponential backoff.
- `rate_limit` — Wait and retry.
- `tx_failed` — **Verify position/account state before retrying.** Never blind-retry on-chain transactions.
- `dangerous_gate` — Set `acknowledged: true`.
- `io` — Check filesystem permissions.
- `internal` — Report a bug.

## High-Value Patterns

### Price check

```
vulcan_market_ticker → { symbol: "SOL" }
```

### Safe order flow (5 steps)

```
1. vulcan_market_info    → { symbol }      # lot sizes, fees, leverage tiers
2. vulcan_market_ticker  → { symbol }      # current price, funding rate
3. vulcan_margin_status  → {}              # available collateral, risk state
4. vulcan_position_list  → {}              # existing positions
5. vulcan_trade_market_buy → { symbol, size, acknowledged: true }
```

### Portfolio snapshot

```
vulcan_margin_status  → {}     # collateral, PnL, risk state
vulcan_position_list  → {}     # all open positions
vulcan_trade_orders   → {}     # all resting orders
```

### Close a position

```
vulcan_position_close → { symbol: "SOL", acknowledged: true }
```

## Tool Groups

37 tools across 8 groups:

| Group | Tools | Auth | Dangerous |
|-------|-------|------|-----------|
| market (5) | list, ticker, info, orderbook, candles | No | No |
| trade (9) | market_buy, market_sell, limit_buy, limit_sell, orders, cancel, cancel_all, set_tpsl, cancel_tpsl | Yes | Yes (except orders) |
| position (5) | list, show, close, reduce, tp_sl | Yes | Yes (except list, show) |
| margin (8) | status, deposit, withdraw, transfer, transfer_child_to_parent, sync_parent_to_child, leverage_tiers, add_collateral | Yes | Yes (except status, leverage_tiers) |
| history (5) | trades, orders, collateral, funding, pnl | Yes | No |
| status (1) | status | No | No |
| wallet (2) | list, balance | No | No |
| account (2) | info, register | Yes | register only |

## Tool Discovery

- Full machine-readable contract: `agents/tool-catalog.json`
- Error codes and recovery hints: `agents/error-catalog.json`
- Workflow skills: `skills/INDEX.md`
- Skills are also available as MCP resources: `vulcan://skills/<name>`
