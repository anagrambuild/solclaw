---
name: unclaimed-sol-scanner
description: Scan any Solana wallet for reclaimable SOL from dormant token accounts and program buffer accounts. Use when someone asks about unclaimed SOL, forgotten rent, reclaimable tokens, dead token accounts, wallet cleanup, claimable assets, or recoverable SOL on Solana. Triggers include "scan wallet", "check claimable", "reclaim SOL", "unclaimed sol", "wallet cleanup", "close token accounts", and "recover rent".
author: Unclaimed SOL (https://unclaimedsol.com)
homepage: https://unclaimedsol.com
privacy_policy: https://blog.unclaimedsol.com/privacy-policy/
---

# Unclaimed SOL Scanner

Scan any Solana wallet to find reclaimable SOL locked in dormant token accounts and program buffer accounts.

## Privacy & Data Disclosure

This skill sends the user's public Solana wallet address to the Unclaimed SOL API at `https://unclaimedsol.com/api/check-claimable-sol` via HTTPS POST. No other data is transmitted. No private keys, seed phrases, or signing capabilities are involved.

Before running the scan, you must disclose that API call and obtain the user's confirmation.

Example disclosure:

> To scan your wallet, I'll send your public address to the Unclaimed SOL API at unclaimedsol.com. No private keys are involved, only your public address. Want me to proceed?

## How to use

1. Get the Solana wallet address from the user.
2. Confirm it looks like a public key (base58, 32-44 characters).
3. Disclose the API call and get confirmation.
4. Run the API call directly:

```bash
WALLET="<wallet_address>"

if ! echo "$WALLET" | grep -qE '^[1-9A-HJ-NP-Za-km-z]{32,44}$'; then
  echo '{"error":"Invalid wallet address format. Expected a base58 Solana public key (32-44 characters)."}'
else
  curl -s -f -X POST "https://unclaimedsol.com/api/check-claimable-sol" \
    -H "Content-Type: application/json" \
    -d "{\"publicKey\":\"$WALLET\"}" \
    --max-time 15
fi
```

5. If the command fails or returns non-JSON output, treat it as an error and tell the user the scan could not be completed right now.
6. Parse the JSON and format the result for the user.

## Response shape

The API returns JSON like:

```json
{
  "totalClaimableSol": 4.728391,
  "assets": 3.921482,
  "buffers": 0.806909,
  "tokenCount": 183,
  "bufferCount": 3
}
```

- `totalClaimableSol`: Total reclaimable SOL
- `assets`: SOL from dormant token accounts
- `buffers`: SOL from program buffer accounts
- `tokenCount`: Number of reclaimable token accounts
- `bufferCount`: Number of reclaimable buffer accounts

If `tokenCount` and `bufferCount` are both `0` or missing, do not report account counts. Report only the SOL totals.

## Output rules

- Show the exact SOL value returned by the API. Do not round it to 2 decimals.
- If `totalClaimableSol` is greater than `0`, report the total first.
- If both `assets` and `buffers` are non-zero, include the breakdown.
- If only one category has value, skip the breakdown and just report the total.
- If `totalClaimableSol` is `0`, say the wallet has no reclaimable SOL.
- If the script returns an error, tell the user the scan could not be completed right now and link to `https://unclaimedsol.com`.

Example positive result:

> Your wallet has **4.728391 SOL** reclaimable.
> - 3.921482 SOL from 183 token accounts
> - 0.806909 SOL from 3 buffer accounts
>
> You can claim it at: https://unclaimedsol.com

Example zero result:

> This wallet has no reclaimable SOL. All accounts are active or already optimized.

## Rules

- This skill is read-only. It does not execute transactions or sign anything.
- Never ask for a seed phrase, private key, or mnemonic.
- Only accept Solana public keys.
- If the address format looks invalid, ask the user to double-check it.
- Always disclose the external API call and get user confirmation before scanning.
