# Vulcan

AI-native CLI for the [Phoenix Perpetuals DEX](https://phoenix.trade) on Solana. Provides both human and agent interfaces.

## Prerequisites

- **Rust** 1.75+ — [install via rustup](https://rustup.rs)
- **Rise SDK** — clone [`rise`](https://github.com/ellipsis-labs/rise) alongside this repo so it lives at `../rise/`

Your directory layout should look like:

```
phoenix-fullstack/
├── rise/         # Rise SDK (phoenix-sdk, phoenix-types, etc.)
└── vulcan/       # This repo
```

## Quick Start

```bash
# install 
cargo install --path vulcan

# Verify
vulcan version

# Run the setup wizard (creates wallet + config)
vulcan setup

# Check markets
vulcan market list
vulcan market ticker SOL
```

## CLI Usage

### Commands

| Command    | Description                                       |
| ---------- | ------------------------------------------------- |
| `setup`    | Interactive wizard — wallet, config, connectivity |
| `wallet`   | Create, import, list, and manage encrypted wallets |
| `market`   | Prices, orderbooks, candles, funding rates         |
| `trade`    | Place, cancel, and manage orders                   |
| `position` | View and manage open positions                     |
| `margin`   | Deposit, withdraw, and monitor collateral          |
| `account`  | Registration, info, subaccounts                    |
| `mcp`      | Start MCP server over stdio (for AI agents)        |
| `version`  | Print version info                                 |

### Global Flags

| Flag              | Description                        |
| ----------------- | ---------------------------------- |
| `-o, --output`    | Output format: `table` (default) or `json` |
| `--dry-run`       | Simulate without submitting a tx   |
| `-y, --yes`       | Skip confirmation prompts          |
| `-w, --wallet`    | Wallet name or path override       |
| `--rpc-url`       | Solana RPC endpoint override       |
| `--api-url`       | Phoenix API endpoint override      |
| `--api-key`       | Phoenix API key override           |
| `-v, --verbose`   | Debug logging to stderr            |


### JSON Envelope

All commands support `-o json`. Responses follow a consistent envelope:

```json
{ "ok": true,  "data": { ... }, "meta": { ... } }
{ "ok": false, "error": { "category": "...", "code": "...", "message": "...", "retryable": false } }
```

## MCP Server

Vulcan exposes an MCP server over stdio so AI agents (Claude, etc.) can call trading tools directly via the [Model Context Protocol](https://modelcontextprotocol.io).

### Starting the server

```bash
# Read-only mode (market data + positions only)
vulcan mcp

# Full access (includes trading, deposits, withdrawals)
vulcan mcp --allow-dangerous

# Filter to specific tool groups
vulcan mcp --allow-dangerous --groups market,trade
```

### Wallet authentication

The MCP server unlocks the wallet once at startup (stdin is reserved for JSON-RPC):

```bash
# Option 1: Environment variable
export VULCAN_WALLET_PASSWORD=your-password
vulcan mcp --allow-dangerous

# Option 2: Stderr prompt (reads from /dev/tty, not stdin)
vulcan mcp --allow-dangerous
# → "Wallet password (for MCP session): " appears on stderr
```

### Claude Code configuration

Add to your Claude Code MCP settings (`~/.claude/settings.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "vulcan": {
      "command": "/path/to/vulcan",
      "args": ["mcp", "--allow-dangerous"],
      "env": {
        "VULCAN_WALLET_PASSWORD": "your-password"
      }
    }
  }
}
```

Dangerous tools require `--allow-dangerous` on the server **and** `"acknowledged": true` in every call.

### Tool groups

Filter exposed tools with `--groups` (comma-separated):

- **market** — Price data and market info (4 tools)
- **trade** — Order placement and cancellation (7 tools)
- **position** — Position monitoring and closing (3 tools)
- **margin** — Collateral management (3 tools)

## Project Structure

```
vulcan/
├── vulcan/           # Binary crate — CLI entry point
└── vulcan-lib/       # Library crate — all logic
    ├── cli/          # Clap derive structs
    ├── commands/     # Command execution (market, trade, position, margin, account)
    ├── mcp/          # MCP server, tool registry, session wallet
    │   ├── server.rs       # rmcp ServerHandler implementation
    │   ├── registry.rs     # Static tool definitions with JSON schemas
    │   └── session_wallet.rs # Pre-decrypted wallet for MCP session
    ├── output/       # JSON envelopes and table formatting
    ├── wallet/       # Encrypted wallet storage (AES-256-GCM + Argon2id)
    ├── crypto/       # Encryption primitives
    ├── config/       # ~/.vulcan/config.toml
    ├── context.rs    # Shared AppContext
    └── error.rs      # Categorized errors with exit codes
```

## Configuration

Config lives at `~/.vulcan/config.toml`:

```toml
[network]
rpc_url = "https://api.mainnet-beta.solana.com"
api_url = "https://public-api.phoenix.trade"
# api_key = "your-api-key"

[wallet]
default = "my-wallet"

[trading]
default_slippage_bps = 50
confirm_trades = true
```

## Development

```bash
cargo build            # Build all crates
cargo test             # Run all tests
cargo run -- --help    # Show CLI help
```
