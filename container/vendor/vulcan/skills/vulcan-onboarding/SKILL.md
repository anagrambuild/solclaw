---
name: vulcan-onboarding
version: 1.0.0
description: "New user setup: wallet creation, invite registration, first deposit, and verification."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["vulcan"]
---

# vulcan-onboarding

Use this skill for:
- First-time setup of vulcan
- Creating and configuring a wallet
- Registering a trader account
- Making the first deposit

## Prerequisites

- Solana wallet with SOL (for transaction fees) and USDC (for collateral).
- An invite code for Phoenix DEX registration.

## Step 1: Install and Configure

```bash
cargo install --path vulcan    # from repo
vulcan setup                   # interactive setup wizard
```

Setup creates `~/.vulcan/config.toml` with network endpoints.

## Step 2: Create a Wallet

Wallet operations are CLI-only (not available via MCP):

```bash
vulcan wallet create           # interactive: name, password
vulcan wallet import           # import existing Solana keypair
vulcan wallet list             # verify wallet created
vulcan wallet set-default <NAME>
```

## Step 3: Fund the Wallet

The wallet needs:
- **SOL** — for Solana transaction fees (~0.01 SOL per transaction).
- **USDC** — for trading collateral.

Check balances:

```
vulcan_wallet_balance → {}
```

## Step 4: Register Trader Account

```
vulcan_account_register → { invite_code: "YOUR_CODE", acknowledged: true }
```

## Step 5: Deposit Collateral

```
vulcan_margin_deposit → { amount: 100.0, acknowledged: true }
```

## Step 6: Verify Everything

```
vulcan_status → {}    # checks config, wallet, RPC, API, registration
```

All checks should pass. If any fail, the status output includes recovery hints.

## Step 7: First Trade (Optional)

Follow the safe order flow from the `vulcan-trade-execution` skill:

```
vulcan_market_info   → { symbol: "SOL" }
vulcan_market_ticker → { symbol: "SOL" }
vulcan_margin_status → {}
```

Then place a small test trade.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `NO_DEFAULT_WALLET` | `vulcan wallet set-default <name>` |
| `DECRYPT_FAILED` | Wrong password. Set `VULCAN_WALLET_PASSWORD` |
| `NO_TRADER_ACCOUNT` | Register with invite code |
| `CONFIG_ERROR` | Run `vulcan setup` |
| Insufficient SOL | Fund wallet with SOL for tx fees |
| Insufficient USDC | Transfer USDC to wallet address |
