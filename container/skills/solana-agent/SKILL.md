---
name: solana-agent
description: Access Solana blockchain data and execute transactions using the configured wallet. Use when user asks about balances, prices, swaps, transfers, or any Solana operations. Triggers on "balance", "swap", "transfer", "send", "price", "solana", wallet addresses, token names.
---

# Solana Agent

Execute Solana operations using CLI tools (standard signing), Swig MCP tools (smart wallet signing), or Crossmint MCP tools (custodial signing).

## When to Use

Use this skill when the user asks about:

- Wallet balances ("What's my balance?", "How much SOL do I have?")
- Token prices ("Price of SOL", "What's BONK worth?")
- Swapping tokens ("Swap 0.1 SOL for USDC", "Trade SOL to BONK")
- Transferring tokens ("Send 0.5 SOL to ...", "Transfer USDC to ...")
- Any Solana blockchain operation

For advanced DeFi (staking, lending, perps, LPs), use the protocol-specific skills (Jupiter, Drift, Kamino, Lulo, Meteora, etc.).

## Step 1: Detect Signing Method

```bash
node -e "const c=JSON.parse(require('fs').readFileSync('config/solana-config.json','utf8')); console.log(c.wallet.signingMethod || (c.wallet.provider === 'solana-agent-kit' ? 'standard' : c.wallet.signingMethod || 'standard'))"
```

- If output is `standard` → use **CLI Tools** (below)
- If output is `swig` → use **Swig MCP Tools** (below)
- If output is `crossmint` → use **Crossmint MCP Tools** (bottom of this file)
- If file not found → tell user to run setup: `npm run setup`

## Standard Path: CLI Tools

All tools are run from the project root. They output JSON.

### Check Balance

```bash
cd /workspace/project && npx tsx tools/solana-balance.ts
```

Output: `{ "sol": 1.234, "address": "..." }`

SPL token balance:

```bash
cd /workspace/project && npx tsx tools/solana-balance.ts --token USDC
```

### Get Prices

```bash
cd /workspace/project && npx tsx tools/solana-price.ts SOL USDC BONK
```

Output: `{ "SOL": "142.35", "USDC": "1.00", "BONK": "0.00002" }`

### Swap Tokens

```bash
cd /workspace/project && npx tsx tools/solana-swap.ts --from SOL --to USDC --amount 0.03
```

Output: `{ "signature": "...", "status": "submitted", "inputAmount": "0.03 SOL", "outputAmount": "...", "explorer": "https://solscan.io/tx/..." }`

### Transfer Tokens

SOL:

```bash
cd /workspace/project && npx tsx tools/solana-transfer.ts --to <address> --amount 0.5
```

SPL Token:

```bash
cd /workspace/project && npx tsx tools/solana-transfer.ts --to <address> --amount 10 --token USDC
```

Output: `{ "signature": "...", "amount": "0.5 SOL", "to": "...", "explorer": "https://solscan.io/tx/..." }`

## Swig Path: MCP Tools

If signing method is `swig`, use the Swig MCP server instead of the local CLI tools.

### Required startup flow

1. Read `config/solana-config.json` to get:
   - `preferences.rpcUrl`
   - `wallet.authorityPublicKey`
   - `wallet.swigWalletAddress`
   - `wallet.swigAccountAddress`
   - `wallet.feeMode`
2. Call `configure_rpc` with the configured RPC URL.
3. If `SWIG_AUTHORITY_PRIVATE_KEY` is available, call `configure_agent_keypair` with it.
4. If fee mode is `paymaster`, call `configure_paymaster` using:
   - `SWIG_PAYMASTER_API_KEY` from the environment
   - `wallet.swigPaymasterPubkey`
   - `wallet.swigPaymasterNetwork`
5. If fee mode is `gas-sponsor`, call `configure_gas_sponsor` with `wallet.gasSponsorUrl`.

### Primary Swig tools

- `get_balance` — check the authority or wallet balance
- `create_swig_wallet` — create a new Swig wallet when one does not exist yet
- `fetch_swig_wallet` — inspect the current wallet, authorities, and addresses
- `add_authority` — add a new authority with permissions
- `remove_authority` — remove an authority
- `update_authority` — update authority permissions
- `transact_sol_transfer` — transfer SOL from the Swig wallet
- `transact_custom` — execute arbitrary instructions through the Swig wallet

### Persistence rule

If `create_swig_wallet` returns new Swig addresses, update `config/solana-config.json` so future sessions can reuse:

- `wallet.swigWalletAddress`
- `wallet.swigAccountAddress`
- `wallet.publicKey` (set this to the Swig wallet address)

## Crossmint Path: MCP Tools

If signing method is `crossmint`, use these MCP tools instead of CLI tools:

- `crossmint_get_balance` — check wallet balance
- `crossmint_transfer` — send SOL or SPL tokens
- `crossmint_sign_transaction` — sign and submit a transaction

For swaps with Crossmint, get a quote from Jupiter first, then sign with `crossmint_sign_transaction`.

## Common Tokens

The CLI tools resolve these symbols automatically. You can also pass raw mint addresses.

| Symbol   | Mint Address                                 |
| -------- | -------------------------------------------- |
| SOL      | So11111111111111111111111111111111111111112  |
| USDC     | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v |
| USDT     | Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB |
| PYUSD    | 2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo |
| EURC     | HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr |
| JUP      | JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN  |
| RAY      | 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R |
| ORCA     | orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE  |
| DRIFT    | DriFtupJYLTosbwoN8koMbEYSx54aFAVLddWsbksjwg7 |
| KMNO     | KMNo3nJsBXfcpJTVhZcXLW7RmTwTt4GVFE7suUBo9sS  |
| MSOL     | mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So  |
| JITOSOL  | J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn |
| JTO      | jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL  |
| JLP      | 27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4 |
| PYTH     | HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3 |
| W        | 85VBFQZC9TZkfaptBWjvUw7YbZjy52A6mjtPGjstQAmQ |
| TNSR     | TNSRxcUxoT9xBG3de7PiJyTDYu7kskLqcpddxnEJAS6  |
| PENGU    | 2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv |
| RENDER   | rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof  |
| HNT      | hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux  |
| MOBILE   | mb1eu7TzEc71KxDpsmsKoucSSuuoGLv1drys1oP2jh6  |
| GRASS    | Grass7B4RdKfBCjTKgSqnXkqjwiGvQyFbuSCUJr3XXjs |
| BONK     | DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 |
| WIF      | EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm |
| POPCAT   | 7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr |
| MEW      | MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5  |
| TRUMP    | 6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN |
| FARTCOIN | 9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump |
| WBTC     | 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh |
| ETH      | 7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs |

See `tools/lib/wallet.ts` for the full list of 50+ supported symbols.

## Response Format

When responding to user:

**Balance Check:**

```
Balance: 1.234567 SOL
Address: 9wsmkna3YUau2oyXb62b7z373Drq2Nah1pX6WcPoqMgB
```

**Swap:**

```
Swap Successful

0.1 SOL -> USDC

Transaction: https://solscan.io/tx/4k2j3...abc123
```

**Price:**

```
SOL: $142.35
USDC: $1.00
```

**Transfer:**

```
Sent 0.5 SOL to 9wsmkna...

Transaction: https://solscan.io/tx/4k2j3...abc123
```

## Error Handling

**Insufficient balance:** Check wallet balance first. If not enough, also check Breeze lending positions — on Fly.io agents (`AGENT_ID` set): `cd /workspace/project && npx tsx tools/solana-balance.ts --breeze-lending`; elsewhere use `breeze_get_balances` MCP tool. If wallet + Breeze combined covers it, withdraw the shortfall from Breeze automatically, then proceed. Only report "insufficient funds" if the total across both is not enough.

**Not configured:** Tell user to run `npm run setup` (step 10 configures Solana).

**RPC error / rate limit:** Retry once after a brief pause. If persistent, suggest configuring a custom RPC provider.

## Installing Protocol SDKs & Creating New Tools

The `tools/` directory has its own `package.json` and `node_modules/`, isolated from the host. When you need a protocol SDK that isn't already available:

1. Install it into `tools/`:

```bash
cd /workspace/project/tools && npm install @whatever/sdk
```

2. Create a new script in `tools/` following the existing pattern:

```bash
cat > /workspace/project/tools/solana-my-action.ts << 'EOF'
#!/usr/bin/env npx tsx
import { loadWallet } from './lib/wallet.js';
// import from tools/node_modules — isolated deps, no version conflicts
import { SomeSDK } from '@whatever/sdk';

const { keypair, connection, publicKey } = loadWallet();
// ... build and sign transaction, output JSON
EOF
```

3. Run it:

```bash
cd /workspace/project && npx tsx tools/solana-my-action.ts
```

The `tools/lib/wallet.ts` helper gives you `keypair`, `connection`, `publicKey`, `rpcUrl` from the user's config. Always output JSON so results are easy to parse.

**Already installed in `tools/node_modules`:** `@kamino-finance/klend-sdk`, `@solana/web3.js`, `@solana/spl-token`, `bs58`, `decimal.js`

## Notes

- All transactions are signed with the configured wallet
- Check balance before executing swaps/transfers
- Use devnet for testing (free SOL via airdrops)
- Mainnet operations use real SOL
- The CLI tools use Jupiter Ultra API (`lite-api.jup.ag`, free, no key required) for swaps/prices
- If `JUPITER_API_KEY` is set, tools use `api.jup.ag` for higher rate limits
