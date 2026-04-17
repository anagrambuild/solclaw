# Agent Integration Guide: vulcan

> **This is experimental software. Commands execute real financial transactions on Solana mainnet. The user who deploys this tool is responsible for all outcomes.**

Self-contained guide for integrating `vulcan` into AI agents, MCP clients, and automated pipelines.

Fast entry points:
- Runtime agent context: `CONTEXT.md`
- Full command contract: `agents/tool-catalog.json`
- Error routing contract: `agents/error-catalog.json`
- Workflow skills: `skills/INDEX.md`

## Installation

From the repo:

```bash
cargo install --path vulcan
```

Verify: `vulcan --version`

## Authentication

### Wallet password (required for dangerous operations)

```bash
export VULCAN_WALLET_PASSWORD="your_password"
```

The MCP server reads this at startup and unlocks the wallet for the session. No per-call prompts.

### Configuration

```bash
vulcan setup    # interactive setup wizard
```

Creates `~/.vulcan/config.toml`:

```toml
[network]
rpc_url = "https://api.mainnet-beta.solana.com"
api_url = "https://perp-api.phoenix.trade"

[wallet]
default = "my-wallet"
```

### Credential resolution order

1. CLI flags (`--rpc-url`, `--api-url`, `--api-key`)
2. Config file (`~/.vulcan/config.toml`)

## Invocation Pattern

### MCP (preferred for agents)

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

Tools are named `vulcan_<group>_<action>`. Dangerous tools require `acknowledged: true`.

Without `--allow-dangerous`, only read-only tools are exposed.

Group filtering: `vulcan mcp --groups market,position` exposes only those groups.

### CLI (fallback)

```bash
vulcan <command> [args...] -o json
```

- `stdout`: JSON data (envelope format).
- `stderr`: Diagnostics/logging only.
- Exit code `0` = success, non-zero = failure.

### Agent context command

For agents using CLI without filesystem access:

```bash
vulcan agent-context    # prints full runtime context to stdout
```

## All Tool Groups

### Market Data (5 tools, read-only, no auth)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_market_list` | — | All markets with fees and leverage |
| `vulcan_market_ticker` | `symbol` | Price, funding rate, 24h volume |
| `vulcan_market_info` | `symbol` | Tick size, lot sizes, fees, leverage tiers |
| `vulcan_market_orderbook` | `symbol`, `depth?` | L2 orderbook snapshot |
| `vulcan_market_candles` | `symbol`, `interval?`, `limit?` | OHLCV history |

### Trading (9 tools, auth required)

| Tool | Dangerous | Args | Description |
|------|-----------|------|-------------|
| `vulcan_trade_market_buy` | Yes | `symbol`, `size`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Market buy |
| `vulcan_trade_market_sell` | Yes | `symbol`, `size`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Market sell |
| `vulcan_trade_limit_buy` | Yes | `symbol`, `size`, `price`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Limit buy |
| `vulcan_trade_limit_sell` | Yes | `symbol`, `size`, `price`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Limit sell |
| `vulcan_trade_orders` | No | `symbol?` | List open orders |
| `vulcan_trade_cancel` | Yes | `symbol`, `order_ids[]`, `acknowledged` | Cancel specific orders |
| `vulcan_trade_cancel_all` | Yes | `symbol`, `acknowledged` | Cancel all orders for market |
| `vulcan_trade_set_tpsl` | Yes | `symbol`, `tp?`, `sl?`, `acknowledged` | Set TP/SL on position |
| `vulcan_trade_cancel_tpsl` | Yes | `symbol`, `tp?`, `sl?`, `acknowledged` | Cancel TP/SL |

### Position (5 tools, auth required)

| Tool | Dangerous | Args | Description |
|------|-----------|------|-------------|
| `vulcan_position_list` | No | — | All open positions |
| `vulcan_position_show` | No | `symbol` | Detailed position info |
| `vulcan_position_close` | Yes | `symbol`, `acknowledged` | Close entire position |
| `vulcan_position_reduce` | Yes | `symbol`, `size`, `acknowledged` | Reduce by size |
| `vulcan_position_tp_sl` | Yes | `symbol`, `tp?`, `sl?`, `acknowledged` | Attach TP/SL bracket |

### Margin (8 tools, auth required)

| Tool | Dangerous | Args | Description |
|------|-----------|------|-------------|
| `vulcan_margin_status` | No | — | Collateral, PnL, risk state |
| `vulcan_margin_deposit` | Yes | `amount`, `acknowledged` | Deposit USDC |
| `vulcan_margin_withdraw` | Yes | `amount`, `acknowledged` | Withdraw USDC |
| `vulcan_margin_transfer` | Yes | `from_subaccount`, `to_subaccount`, `amount`, `acknowledged` | Transfer between subaccounts |
| `vulcan_margin_transfer_child_to_parent` | Yes | `child_subaccount`, `acknowledged` | Sweep child to cross-margin |
| `vulcan_margin_sync_parent_to_child` | Yes | `child_subaccount`, `acknowledged` | Sync parent to child |
| `vulcan_margin_leverage_tiers` | No | `symbol` | Leverage tier schedule |
| `vulcan_margin_add_collateral` | Yes | `symbol`, `amount`, `acknowledged` | Add collateral to isolated |

### History (5 tools, read-only, NOT YET IMPLEMENTED)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_history_trades` | `symbol?`, `limit?` | Trade/fill history |
| `vulcan_history_orders` | `symbol?`, `limit?` | Order history |
| `vulcan_history_collateral` | `limit?` | Deposit/withdrawal history |
| `vulcan_history_funding` | `symbol?`, `limit?` | Funding payment history |
| `vulcan_history_pnl` | `resolution?`, `limit?` | PnL over time |

### Status, Wallet, Account (5 tools)

| Tool | Dangerous | Auth | Args | Description |
|------|-----------|------|------|-------------|
| `vulcan_status` | No | No | — | Health check |
| `vulcan_wallet_list` | No | No | — | List wallets |
| `vulcan_wallet_balance` | No | No | `name?` | SOL/USDC balance |
| `vulcan_account_info` | No | Yes | — | Account info |
| `vulcan_account_register` | Yes | Yes | `invite_code`, `acknowledged` | Register account |

## Output Parsing

### Success envelope

```json
{
  "ok": true,
  "data": { ... },
  "meta": { "timestamp": "...", "duration_ms": 123 }
}
```

### Error envelope

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

## Error Handling

| Category | Exit | Retryable | Action |
|----------|------|-----------|--------|
| `validation` | 1 | No | Fix input |
| `auth` | 2 | No | Check wallet/password |
| `config` | 3 | No | Run `vulcan setup` |
| `api` | 4 | No | Check API connectivity |
| `network` | 5 | Yes | Retry with backoff |
| `rate_limit` | 6 | Yes | Wait and retry |
| `tx_failed` | 7 | No | Verify state, then retry |
| `io` | 8 | Yes | Check file permissions |
| `dangerous_gate` | 9 | No | Set `acknowledged: true` |
| `internal` | 10 | No | Report bug |

Full error code reference: `agents/error-catalog.json`

## Symbol Format

Uppercase ticker: `SOL`, `BTC`, `ETH`, `DOGE`, `SUI`, `XRP`, `BNB`, `AAVE`, `ZEC`, `HYPE`, `SKR`. No `-PERP` suffix.

## Size Units

Size is in **base lots**. Call `vulcan_market_info` first to get `base_lots_decimals`.

`base_lots = desired_tokens * 10^base_lots_decimals`

## Dangerous Commands (20 total)

All dangerous tools require `acknowledged: true` via MCP or `--yes` via CLI:

- Trade: market_buy, market_sell, limit_buy, limit_sell, cancel, cancel_all, set_tpsl, cancel_tpsl
- Position: close, reduce, tp_sl
- Margin: deposit, withdraw, transfer, transfer_child_to_parent, sync_parent_to_child, add_collateral
- Account: register

## Workflow Skills

Goal-oriented workflow guides in `skills/`. Read `skills/INDEX.md` for the full list.

Skills are also available as MCP resources: `vulcan://skills/<skill-name>`

**Core**: vulcan-shared, vulcan-risk-management, vulcan-error-recovery
**Trading**: vulcan-trade-execution, vulcan-lot-size-calculator, vulcan-tpsl-management
**Market**: vulcan-market-intel
**Portfolio**: vulcan-portfolio-intel, vulcan-margin-operations, vulcan-onboarding
**Position**: vulcan-position-management
**Recipes**: recipe-emergency-flatten, recipe-open-hedged-position, recipe-morning-portfolio-check, recipe-scale-into-position, recipe-funding-rate-harvest, recipe-close-and-withdraw

## Machine-Readable Resources

| Resource | Path | Description |
|----------|------|-------------|
| Tool catalog | `agents/tool-catalog.json` | All 37 tools with parameters and schemas |
| Error catalog | `agents/error-catalog.json` | Error codes, categories, recovery hints |
| Skills index | `skills/INDEX.md` | All workflow skills |
| Runtime context | `CONTEXT.md` | Compact runtime contract |
