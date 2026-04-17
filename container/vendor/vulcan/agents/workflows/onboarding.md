# Onboarding Workflow

> **Note:** This workflow is superseded by the richer skill at `skills/vulcan-onboarding/SKILL.md`. This file is kept for backward compatibility with existing MCP resource URIs.

Follow this checklist to set up a new user on Phoenix Perpetuals DEX.

## Prerequisites

The user needs:
- **SOL** in their Solana wallet (for transaction fees, ~0.01 SOL minimum)
- **USDC** in their Solana wallet (for trading collateral)
- **Invite code** from an existing Phoenix user

## Step 1: Create or Import a Wallet

```bash
vulcan wallet create --name <NAME>         # generate a new keypair
vulcan wallet import --name <NAME> <SRC>   # or import existing key
vulcan wallet set-default <NAME>           # set as active wallet
```

The wallet is encrypted with a password on creation. For agent/MCP use, set `VULCAN_WALLET_PASSWORD` to avoid interactive prompts.

After creation, give the user their public key so they can fund it with SOL and USDC before proceeding.

## Step 2: Fund the Wallet

The wallet needs:
1. **SOL** — at least 0.01 SOL for transaction fees (registration + deposit = 2 transactions)
2. **USDC** — the amount the user wants as trading collateral

Funding happens outside Vulcan (wallet transfer, exchange withdrawal, etc). Confirm the user has funded before proceeding.

## Step 3: Register with Invite Code

```
vulcan account register --invite-code <CODE>
```

This does two things:
1. Activates the invite code via the Phoenix API
2. Creates the on-chain trader account (PDA at subaccount index 0, cross-margin)

If the trader is already registered, the command skips the on-chain transaction and reports the existing account.

After registration, verify with:
```
vulcan account info
```

You should see the trader PDA, state, and zero collateral.

## Step 4: Deposit Collateral

```
vulcan margin deposit <AMOUNT> --yes
```

`AMOUNT` is in USDC (e.g., `100` for $100). The `--yes` flag skips the confirmation prompt. Use `--dry-run` first to simulate without submitting.

Verify the deposit:
```
vulcan margin status
```

Collateral should reflect the deposited amount and risk state should be `Healthy`.

## Step 5: Verify Setup

Run these checks to confirm everything is working:

```
vulcan account info          # trader registered, collateral > 0
vulcan market list           # markets load successfully
vulcan market ticker SOL     # price data flows
```

## After Onboarding

The user is ready to trade. Point them to:
- `vulcan trade market-buy SOL <size> --yes` — place a market order
- `vulcan position list` — view positions
- `vulcan trade orders` — view open orders

For agent workflows, read `vulcan://agents/workflows/trade` before placing any orders.

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `REGISTER_API_FAILED: 404` | Wrong API URL | Check `~/.vulcan/config.toml` — `api_url` should point to the active Phoenix API |
| `REGISTER_API_FAILED: 400/403` | Invalid or used invite code | Get a new invite code |
| `TX_SEND_FAILED: no record of a prior credit` | Wallet has no SOL | Fund the wallet with SOL for tx fees |
| `TX_SEND_FAILED: invalid account data` | Trader already registered | Run `vulcan account info` to confirm — this is safe to ignore |
| `CONFIRMATION_REQUIRED` | Missing `--yes` flag on deposit | Add `--yes` or `--dry-run` |
