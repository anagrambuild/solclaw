---
name: phoenix-perps
creator: phoenix
description: Trade perpetual futures on Phoenix DEX (Solana) via the vulcan CLI. Market data, order execution, position/margin management. Use when the user wants to trade perps, check perp positions, or interact with Phoenix DEX.
---

# Phoenix Perpetual Futures — Vulcan CLI

**This tool executes real financial transactions on Solana mainnet. Always confirm with the user before executing trades.**

You have access to the `vulcan` CLI for trading perpetual futures on Phoenix DEX.

## Invocation

Always use JSON output for machine parsing:

```bash
vulcan <command> [args...] -o json
```

- `stdout` is the only data channel (JSON).
- `stderr` is diagnostics only.
- Exit code `0` = success, non-zero = failure with JSON error envelope.
- Use `--yes` to skip interactive confirmation prompts (required for agent use).
- Use `--dry-run` to simulate without submitting transactions.

## Authentication

Wallet password is pre-configured in the container. No manual auth needed.

## Symbol Format

Uppercase ticker only: `SOL`, `BTC`, `ETH`, `DOGE`, `SUI`, `XRP`, `BNB`, `AAVE`, `ZEC`, `HYPE`, `SKR`.

No `-PERP` suffix. Discover active markets with:

```bash
vulcan market list -o json
```

## Size Units — Base Lots (CRITICAL)

The `size` parameter is in **base lots**, NOT tokens or USD. Getting this wrong means trading 100x more or less than intended.

**Before every trade**, fetch market info:

```bash
vulcan market info SOL -o json
```

Extract `base_lots_decimals` from the response, then convert:

```
base_lots = desired_tokens * 10^base_lots_decimals
```

### Worked Examples

| Market | base_lots_decimals | Want | Calculation | Size param |
|--------|-------------------|------|-------------|------------|
| SOL | 2 | 0.5 SOL | 0.5 * 100 | 50 |
| SOL | 2 | 1 SOL | 1 * 100 | 100 |
| BTC | 4 | 0.001 BTC | 0.001 * 10000 | 10 |
| ETH | 3 | 0.1 ETH | 0.1 * 1000 | 100 |

### USD to base lots

```
tokens = usd_amount / mark_price
base_lots = tokens * 10^base_lots_decimals
```

Round to nearest integer. Base lots must be whole numbers.

## Commands Reference

### Market Data (safe, read-only)

```bash
vulcan market list -o json                          # all markets
vulcan market ticker SOL -o json                    # price, funding rate, 24h volume
vulcan market info SOL -o json                      # lot sizes, fees, leverage tiers
vulcan market orderbook SOL -o json                 # L2 bids/asks
vulcan market orderbook SOL --depth 20 -o json      # deeper book
vulcan market candles SOL -o json                   # OHLCV (default 1h, 50 candles)
vulcan market candles SOL --interval 5m --limit 100 -o json
```

### Trading (DANGEROUS — always confirm with user)

```bash
# Market orders
vulcan trade market-buy SOL 50 --yes -o json
vulcan trade market-sell SOL 50 --yes -o json

# Market order with take-profit and stop-loss
vulcan trade market-buy SOL 50 --tp 160.0 --sl 140.0 --yes -o json

# Limit orders
vulcan trade limit-buy SOL 50 --price 145.00 --yes -o json
vulcan trade limit-sell SOL 50 --price 155.00 --yes -o json

# List open orders
vulcan trade orders -o json
vulcan trade orders --symbol SOL -o json

# Cancel orders
vulcan trade cancel SOL --order-ids id1,id2 --yes -o json
vulcan trade cancel-all SOL --yes -o json

# TP/SL management
vulcan trade set-tpsl SOL --tp 160.0 --sl 140.0 --yes -o json
vulcan trade cancel-tpsl SOL --yes -o json
```

### Position Management

```bash
vulcan position list -o json                        # all open positions
vulcan position show SOL -o json                    # detailed: PnL, liquidation price, TP/SL
vulcan position close SOL --yes -o json             # close entire position
vulcan position reduce SOL 25 --yes -o json         # reduce by 25 base lots
vulcan position tp-sl SOL --tp 160 --sl 140 --yes -o json  # attach TP/SL to existing
```

### Margin & Collateral

```bash
vulcan margin status -o json                        # collateral, PnL, risk state
vulcan margin deposit 100 --yes -o json             # deposit 100 USDC
vulcan margin withdraw 50 --yes -o json             # withdraw 50 USDC
vulcan margin leverage-tiers SOL -o json            # max leverage per size tier
```

### Account & Wallet

```bash
vulcan account info -o json                         # trader account status
vulcan wallet balance -o json                       # SOL and USDC balance
vulcan status -o json                               # health check: config, wallet, RPC
```

## Safe Order Flow (5 steps)

**Always follow this pattern before placing a trade:**

1. **Market info** — get lot sizes and fees:
   ```bash
   vulcan market info SOL -o json
   ```

2. **Price check** — current mark price and funding:
   ```bash
   vulcan market ticker SOL -o json
   ```

3. **Margin check** — ensure sufficient collateral:
   ```bash
   vulcan margin status -o json
   ```

4. **Position check** — know existing exposure:
   ```bash
   vulcan position list -o json
   ```

5. **Execute** (after user confirmation):
   ```bash
   vulcan trade market-buy SOL 50 --yes -o json
   ```

6. **Verify** — confirm position opened:
   ```bash
   vulcan position list -o json
   ```

Report the transaction signature to the user.

## Risk Management

### Margin Health States

| State | Meaning | Action |
|-------|---------|--------|
| `Healthy` | Sufficient collateral | Safe to trade |
| `HighRisk` | Margin getting thin | Warn user before new trades |
| `Liquidatable` | At risk of liquidation | Do NOT open new positions |

### When to Warn the User

- Risk state is anything other than Healthy
- Trade would use >50% of available margin
- Liquidation price is within 10% of mark price
- Funding rate is elevated (>0.01% per interval)
- Orderbook spread is wide (>10bps)
- Increasing an already-large position

### TP/SL Direction Rules

- **Long (buy):** TP must be ABOVE entry, SL must be BELOW entry
- **Short (sell):** TP must be BELOW entry, SL must be ABOVE entry

## Emergency Flatten

Cancel all orders and close all positions:

```bash
# 1. Cancel all orders per market
vulcan trade cancel-all SOL --yes -o json
vulcan trade cancel-all BTC --yes -o json

# 2. Close all positions
vulcan position close SOL --yes -o json
vulcan position close BTC --yes -o json

# 3. Verify flat
vulcan position list -o json
vulcan trade orders -o json
vulcan margin status -o json
```

## Error Handling

Errors return JSON with structured categories:

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

| Category | Action |
|----------|--------|
| `validation` | Fix inputs, do not retry |
| `auth` | Check wallet/password |
| `config` | Run `vulcan setup` |
| `network` | Retry with backoff |
| `rate_limit` | Wait and retry |
| `tx_failed` | **Check position state before retrying** — never blind-retry |
| `dangerous_gate` | Add `--yes` flag |

## Hard Rules

1. **Always call `vulcan market info` before trading** — never guess lot sizes.
2. **Always call `vulcan margin status` before opening positions** — ensure collateral.
3. **Always call `vulcan position list` before trading** — know existing exposure.
4. **Never execute trades without user confirmation** unless they explicitly opted into auto-execute mode.
5. **Report all transaction signatures** to the user for on-chain verification.
6. **On `tx_failed`, verify state before retrying** — the tx may have partially succeeded.
