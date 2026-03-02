---
name: solana-agent
description: Access Solana blockchain data and execute transactions using the configured wallet. Use when user asks about balances, prices, swaps, staking, or any Solana operations. Triggers on "balance", "swap", "stake", "price", "solana", wallet addresses, token names.
---

# Solana Agent

Execute Solana operations using the configured wallet via Solana Agent Kit.

## When to Use

Use this skill when the user asks about:
- Wallet balances ("What's my balance?", "How much SOL do I have?")
- Token prices ("Price of SOL", "What's BONK worth?")
- Swapping tokens ("Swap 0.1 SOL for USDC", "Trade SOL to BONK")
- Staking ("Stake 1 SOL", "How do I stake?")
- Deploying tokens ("Create a new token")
- Minting NFTs ("Mint an NFT")
- Trending tokens ("What's trending?")
- Any Solana blockchain operation

## Configuration Check

First, verify Solana is configured:

```bash
npx tsx -e "import { isSolanaConfigured } from './src/solana/index.js'; console.log(await isSolanaConfigured());"
```

If not configured, tell user to run: `npm run setup:solana-quick -- --generate --rpc devnet`

## Get Wallet Info

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent } from './src/solana/index.js'; const c = await loadAgentConfig(); const a = createSolanaAgent(c); console.log('Public Key:', a.publicKey); console.log('RPC:', a.rpcUrl);"
```

## Operations

### Check Balance

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent } from './src/solana/index.js'; const a = createSolanaAgent(await loadAgentConfig()); const bal = await a.getBalanceSOL(); console.log(\`Balance: \${bal.toFixed(6)} SOL\`);"
```

### Swap Tokens

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent, COMMON_TOKENS } from './src/solana/index.js'; const a = createSolanaAgent(await loadAgentConfig()); const sig = await a.swap(COMMON_TOKENS.USDC, 0.1, null, 50); console.log('Swap completed:', sig);"
```

### Stake SOL

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent } from './src/solana/index.js'; const a = createSolanaAgent(await loadAgentConfig()); const sig = await a.stake(1); console.log('Staked 1 SOL:', sig);"
```

### Get Token Price

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent, COMMON_TOKENS } from './src/solana/index.js'; const a = createSolanaAgent(await loadAgentConfig()); const price = await a.getPrice(COMMON_TOKENS.SOL); console.log(\`SOL Price: $\${price.toFixed(2)}\`);"
```

### Trending Tokens

```bash
npx tsx -e "import { loadAgentConfig, createSolanaAgent } from './src/solana/index.js'; const a = createSolanaAgent(await loadAgentConfig()); const trending = await a.getTrendingTokens(); console.log('Trending:', trending.slice(0, 5));"
```

## Common Tokens

```typescript
COMMON_TOKENS = {
  SOL: 'So11111111111111111111111111111111111111112',
  USDC: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  USDT: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
  BONK: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',
  JUP: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN',
}
```

## Response Format

When responding to user:

**Balance Check:**
```
💰 Balance

1.234567 SOL

Address: 9wsmkna3YUau2oyXb62b7z373Drq2Nah1pX6WcPoqMgB
```

**Swap:**
```
✅ Swap Successful

0.1 SOL → USDC

Transaction: https://solscan.io/tx/4k2j3...abc123
```

**Price:**
```
💵 SOL Price

$142.35
```

**Trending:**
```
🔥 Trending Tokens

1. BONK
2. WIF
3. POPCAT
4. BOME
5. MYRO
```

## Error Handling

**Insufficient balance:**
```
❌ Insufficient balance

Requested: 10 SOL
Available: 1.234567 SOL
```

**Not configured:**
```
❌ Solana not configured

To use Solana features, run:
npm run setup:solana-quick -- --generate --rpc devnet

Then get free SOL:
solana airdrop 1 <PUBLIC_KEY> --url devnet
```

**RPC error:**
```
❌ Error: RPC rate limit exceeded

Try again in a moment, or configure a custom RPC:
npm run setup:solana-quick -- --key YOUR_KEY --rpc https://rpc.helius.xyz
```

## All Available Methods

The agent has 60+ methods available via `agent.methods.*`:

**Token Operations:**
- transfer, trade/swap, deployToken, stake
- getTokenDataByAddress, getTokenDataByTicker
- getTPS, getPrice

**DeFi:**
- lendAssets, stakeWithJup
- createRaydiumAmmV4, createRaydiumClmm, createRaydiumCpmm
- openDriftPosition, closeDriftPosition
- depositToAdrena, withdrawFromAdrena
- createMeteoraPool, addMeteoraLiquidity

**NFT:**
- deployCollection, mintNFT
- create3LandCollection, create3LandItem

**Misc:**
- resolveDomain, getPrimaryDomain
- fetchPrice, getTrendingTokens, getTrendingPools
- requestFaucetFunds (devnet only)
- launchPumpfunToken

## Notes

- All transactions are signed with the configured wallet
- Check balance before executing swaps/stakes
- Use devnet for testing (free SOL via airdrops)
- Mainnet operations use real SOL
- Transaction fees are automatically included
- Slippage default is 50 basis points (0.5%)
