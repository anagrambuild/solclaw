# CLAUDE.md

> **This is experimental software. Commands execute real financial transactions on Solana mainnet. The user who deploys this tool is responsible for all outcomes.**

Vulcan is an AI-native CLI for trading perpetual futures on Phoenix DEX (Solana). Every command returns structured JSON. Designed for AI agents and automated pipelines.

Fast entry points:
- Runtime context: `CONTEXT.md`
- Full command contract: `agents/tool-catalog.json`
- Error routing: `agents/error-catalog.json`
- Workflow skills: `skills/INDEX.md`

## Build & Run

```bash
cargo build                          # Build all crates
cargo run -- --help                  # Show help
cargo run -- market ticker SOL       # Get ticker
cargo test                           # Run all tests
```

## Architecture

- **`vulcan/`** — Binary crate. Entry point, clap parse, dispatch to commands.
- **`vulcan-lib/`** — Library crate. All logic lives here.
  - `cli/` — Clap derive structs only. No business logic.
  - `commands/` — Command execution. Receives parsed args, calls SDK, returns typed results.
  - `output/` — JSON envelope and table formatting.
  - `mcp/` — MCP server, tool registry, session wallet.
  - `wallet/` — Wallet struct + encrypted storage (AES-256-GCM + Argon2id).
  - `config/` — `~/.vulcan/config.toml` parsing.
  - `context.rs` — `AppContext` shared across commands.
  - `error.rs` — `VulcanError` with categories and exit codes.

## Agent Runtime Contract

**This section is the primary context for AI agents. Read it before using any Vulcan tool.**

### Invocation

Prefer MCP tools when available. Fallback to CLI:

```bash
vulcan <command> [args...] -o json
```

- MCP tools are named `vulcan_<group>_<action>` (e.g., `vulcan_market_ticker`).
- CLI: stdout is JSON, stderr is diagnostics. Exit 0 = success, non-zero = failure.
- For non-Claude-Code agents: run `vulcan agent-context` to load this runtime contract.

### Authentication

MCP server unlocks the wallet once at startup:

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

### Symbol Format

Uppercase ticker only: `SOL`, `BTC`, `ETH`, `DOGE`, `SUI`, `XRP`, `BNB`, `AAVE`, `ZEC`, `HYPE`, `SKR`. No `-PERP` suffix. Use `vulcan_market_list` to discover active markets.

### Size Units — Base Lots

The `size` parameter is in **base lots**, not tokens or USD. You MUST call `vulcan_market_info` first to get `base_lots_decimals`.

**Conversion**: `base_lots = desired_tokens * 10^base_lots_decimals`

| Want | Decimals | Calculation | Base lots |
|------|----------|-------------|-----------|
| 0.5 SOL | 2 | 0.5 * 100 | 50 |
| 0.01 ETH | 3 | 0.01 * 1000 | 10 |
| 0.001 BTC | 4 | 0.001 * 10000 | 10 |

### Safety Rules

1. All dangerous operations require `acknowledged: true` (MCP) or `--yes` (CLI).
2. **Always call `vulcan_market_info` before trading** — never guess lot sizes.
3. **Always call `vulcan_margin_status` before opening positions** — ensure sufficient collateral.
4. **Always call `vulcan_position_list` before trading** — know what's already open.
5. Report all transaction signatures to the user.

### Confirmation Modes

Ask the user at session start:
- **Confirm each** (default): Present trade details, wait for explicit approval before every dangerous op.
- **Auto-execute**: User grants blanket permission. Still log actions, report signatures, respect risk guardrails.

### Error Handling

Errors return `{ "category", "code", "message", "retryable" }`. Route on category:

| Category | Action |
|----------|--------|
| `validation` | Fix input, do not retry unchanged |
| `auth` | Check wallet, password, permissions |
| `config` | Run `vulcan setup` |
| `api` | Check API connectivity, inspect message |
| `network` | Retry with backoff |
| `rate_limit` | Wait and retry |
| `tx_failed` | **Check position state before retrying** — never blind-retry on-chain tx |
| `dangerous_gate` | Add `acknowledged: true` |

## All 37 MCP Tools

### Market Data (5 tools, read-only)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_market_list` | — | All markets with fees and leverage |
| `vulcan_market_ticker` | `symbol` | Price, funding rate, 24h volume |
| `vulcan_market_info` | `symbol` | Tick size, lot sizes, fees, leverage tiers |
| `vulcan_market_orderbook` | `symbol`, `depth?` | L2 orderbook snapshot |
| `vulcan_market_candles` | `symbol`, `interval?`, `limit?` | OHLCV history |

### Trading (9 tools, dangerous except orders)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_trade_market_buy` | `symbol`, `size`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Market buy |
| `vulcan_trade_market_sell` | `symbol`, `size`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Market sell |
| `vulcan_trade_limit_buy` | `symbol`, `size`, `price`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Limit buy |
| `vulcan_trade_limit_sell` | `symbol`, `size`, `price`, `tp?`, `sl?`, `isolated?`, `collateral?`, `reduce_only?`, `acknowledged` | Limit sell |
| `vulcan_trade_orders` | `symbol?` | List open orders (read-only) |
| `vulcan_trade_cancel` | `symbol`, `order_ids[]`, `acknowledged` | Cancel specific orders |
| `vulcan_trade_cancel_all` | `symbol`, `acknowledged` | Cancel all orders for market |
| `vulcan_trade_set_tpsl` | `symbol`, `tp?`, `sl?`, `acknowledged` | Set TP/SL on existing position |
| `vulcan_trade_cancel_tpsl` | `symbol`, `tp?`, `sl?`, `acknowledged` | Cancel TP/SL on position |

### Position (5 tools)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_position_list` | — | All open positions with PnL |
| `vulcan_position_show` | `symbol` | Detailed position: PnL, margin, liquidation, TP/SL |
| `vulcan_position_close` | `symbol`, `acknowledged` | Close entire position (dangerous) |
| `vulcan_position_reduce` | `symbol`, `size`, `acknowledged` | Reduce position by size (dangerous) |
| `vulcan_position_tp_sl` | `symbol`, `tp?`, `sl?`, `acknowledged` | Attach TP/SL bracket to position (dangerous) |

### Margin (8 tools)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_margin_status` | — | Collateral, PnL, risk state (read-only) |
| `vulcan_margin_deposit` | `amount`, `acknowledged` | Deposit USDC (dangerous) |
| `vulcan_margin_withdraw` | `amount`, `acknowledged` | Withdraw USDC (dangerous) |
| `vulcan_margin_transfer` | `from_subaccount`, `to_subaccount`, `amount`, `acknowledged` | Transfer between subaccounts (dangerous) |
| `vulcan_margin_transfer_child_to_parent` | `child_subaccount`, `acknowledged` | Sweep child to cross-margin (dangerous) |
| `vulcan_margin_sync_parent_to_child` | `child_subaccount`, `acknowledged` | Sync parent state to child (dangerous) |
| `vulcan_margin_leverage_tiers` | `symbol` | Leverage tier schedule (read-only) |
| `vulcan_margin_add_collateral` | `symbol`, `amount`, `acknowledged` | Add collateral to isolated position (dangerous) |

### History (5 tools, read-only, NOT YET IMPLEMENTED)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_history_trades` | `symbol?`, `limit?` | Past trade/fill history |
| `vulcan_history_orders` | `symbol?`, `limit?` | Past order history |
| `vulcan_history_collateral` | `limit?` | Deposit/withdrawal history |
| `vulcan_history_funding` | `symbol?`, `limit?` | Funding payment history |
| `vulcan_history_pnl` | `resolution?`, `limit?` | PnL over time |

### Status, Wallet, Account (5 tools)

| Tool | Args | Description |
|------|------|-------------|
| `vulcan_status` | — | Health check: config, wallet, RPC, API, registration |
| `vulcan_wallet_list` | — | All stored wallets |
| `vulcan_wallet_balance` | `name?` | SOL and USDC balance |
| `vulcan_account_info` | — | Trader account: collateral, positions, risk |
| `vulcan_account_register` | `invite_code`, `acknowledged` | Register with invite code (dangerous) |

## Common Patterns

### Check price

```
vulcan_market_ticker → { symbol: "SOL" }
```

### Safe order flow

```
1. vulcan_market_info    → { symbol: "SOL" }     # get lot sizes, fees
2. vulcan_market_ticker  → { symbol: "SOL" }     # current price
3. vulcan_margin_status  → {}                     # check collateral
4. vulcan_position_list  → {}                     # existing positions
5. vulcan_trade_market_buy → { symbol: "SOL", size: 50, acknowledged: true }
6. vulcan_position_list  → {}                     # verify position opened
```

### Portfolio snapshot

```
vulcan_margin_status   → {}          # collateral, PnL, risk state
vulcan_position_list   → {}          # all positions
vulcan_trade_orders    → {}          # all open orders
```

### Close position

```
vulcan_position_close → { symbol: "SOL", acknowledged: true }
```

## Workflow Skills

For deeper, goal-oriented workflows, read skills from `skills/INDEX.md`:
- **Core**: vulcan-shared, vulcan-risk-management, vulcan-error-recovery
- **Trading**: vulcan-trade-execution, vulcan-lot-size-calculator, vulcan-tpsl-management
- **Portfolio**: vulcan-portfolio-intel, vulcan-margin-operations, vulcan-market-intel
- **Recipes**: recipe-emergency-flatten, recipe-open-hedged-position, recipe-morning-portfolio-check

Skills are also available as MCP resources: `vulcan://skills/<skill-name>`

## Conventions

- All commands return `Result<(), VulcanError>`. Never use `anyhow` in command return types.
- JSON envelope: `{ "ok": true, "data": ..., "meta": ... }` or `{ "ok": false, "error": { "category", "code", "message", "retryable" } }`.
- Wallet private keys are never logged, printed, or exported. The `Wallet` struct uses `Zeroize`/`ZeroizeOnDrop`.
- Dependencies: Rise SDK (`phoenix-sdk`, `phoenix-types`, `phoenix-math-utils`), Solana SDK, clap 4, rmcp.
