---
name: orca
creator: raunit-dev
description: Complete guide for Orca - Solana's leading concentrated liquidity AMM (CLMM). Covers Whirlpools SDK for swaps, liquidity provision, pool creation, position management, and fee harvesting on Solana and Eclipse networks.
---

# Orca Whirlpools Development Guide

Orca is the most trusted DEX on Solana and Eclipse, built on a concentrated liquidity automated market maker (CLMM) called Whirlpools. This guide covers the Whirlpools SDK for building trading, liquidity provision, and pool management applications.

## CRITICAL: Two API Levels — Wrapper Functions vs Instructions Functions

The Orca SDK v7 has **two API levels**. Using the wrong function signatures is the #1 cause of errors.

### Wrapper Functions (swap, openConcentratedPosition, etc.)
These use **global config** set by `setRpc()` and `setPayerFromBytes()`. They do **NOT** take `rpc` or `wallet` arguments. They return `{ ..., callback }` — call `callback()` to send the transaction.

```typescript
// Setup (once)
await setWhirlpoolsConfig("solanaMainnet");
await setRpc(config.preferences.rpcUrl);
const wallet = await setPayerFromBytes(keypairBytes);

// swap() — NO rpc, NO wallet args!
const result = await swap(
  { inputAmount: 10_000_000n, mint: SOL_MINT },  // input param
  poolAddress,                                      // pool
  100                                               // slippage bps (1%)
);
console.log("Quote:", result.quote.tokenEstOut);
const txId = await result.callback();  // sends the transaction
```

```typescript
// ❌ WRONG — passing rpc and wallet shifts all params and causes RPC errors!
const result = await swap(rpc, { inputAmount, mint }, poolAddress, slippage, wallet);
```

### Instructions Functions (swapInstructions, openPositionInstructions, etc.)
These take `rpc` and `signer/funder` explicitly. Use these when you need the quote/instructions without sending.

```typescript
import { createSolanaRpc } from "@solana/kit";
const rpc = createSolanaRpc(config.preferences.rpcUrl);

const { instructions, quote } = await swapInstructions(
  rpc,                                                // explicit rpc
  { inputAmount: 10_000_000n, mint: SOL_MINT },       // input param
  poolAddress,                                         // pool
  100,                                                 // slippage bps
  wallet                                               // explicit signer
);
```

### Complete Function Reference

**Wrapper functions** (use global config, return `{ ..., callback }`):
| Function | Arguments (NO rpc, NO wallet) |
|----------|-------------------------------|
| `swap` | `(input, poolAddress, slippage?)` |
| `openConcentratedPosition` | `(poolAddress, param, lowerPrice, upperPrice, slippage?, withMetadata?)` |
| `openFullRangePosition` | `(poolAddress, param, slippage?, withMetadata?)` |
| `increasePosLiquidity` | `(positionMint, param, slippage?)` |
| `decreaseLiquidity` | `(positionMint, param, slippage?)` |
| `harvestPosition` | `(positionMint)` |
| `closePosition` | `(positionMint, slippage?)` |
| `createSplashPool` | `(tokenMintA, tokenMintB, initialPrice?)` |
| `createConcentratedLiquidityPool` | `(tokenMintA, tokenMintB, tickSpacing, initialPrice?)` |

**Instructions functions** (take rpc + signer explicitly):
| Function | First arg | Last arg |
|----------|-----------|----------|
| `swapInstructions` | `rpc` | `signer` |
| `openPositionInstructions` | `rpc` | `funder` |
| `openFullRangePositionInstructions` | `rpc` | `funder` |
| `increaseLiquidityInstructions` | `rpc` | `authority` |
| `decreaseLiquidityInstructions` | `rpc` | `authority` |
| `harvestPositionInstructions` | `rpc` | `authority` |
| `closePositionInstructions` | `rpc` | `authority` |

**Fetch functions** (take rpc explicitly, read-only):
| Function | Arguments |
|----------|-----------|
| `fetchConcentratedLiquidityPool` | `(rpc, mintOne, mintTwo, tickSpacing)` |
| `fetchSplashPool` | `(rpc, mintOne, mintTwo)` |
| `fetchWhirlpoolsByTokenPair` | `(rpc, mintOne, mintTwo)` |
| `fetchPositionsForOwner` | `(rpc, ownerAddress)` |
| `fetchPositionsInWhirlpool` | `(rpc, whirlpoolAddress)` |

### RPC setup
```typescript
import { createSolanaRpc } from "@solana/kit";
// For global config (wrapper functions):
await setRpc(config.preferences.rpcUrl);
// For explicit rpc (instructions/fetch functions):
const rpc = createSolanaRpc(config.preferences.rpcUrl);
```
Do NOT use `new Connection()` from `@solana/web3.js` — it is incompatible with the Orca SDK.

## Overview

Orca Whirlpools provides:

- **Token Swaps** - Efficient token exchanges with low slippage and competitive rates
- **Concentrated Liquidity** - Provide liquidity within custom price ranges for higher capital efficiency
- **Splash Pools** - Simple full-range liquidity provision for beginners
- **Position Management** - Open, increase, decrease, and close liquidity positions
- **Fee Harvesting** - Collect trading fees and rewards from positions
- **Pool Creation** - Permissionless creation of new liquidity pools

## Program IDs

| Network | Program ID |
|---------|------------|
| Solana Mainnet | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` |
| Solana Devnet | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` |
| Eclipse Mainnet | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` |
| Eclipse Testnet | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` |

## Quick Start

### Installation

**New SDK (Solana Web3.js v2):**
```bash
npm install @orca-so/whirlpools @solana/kit
```

**Legacy SDK (Solana Web3.js v1):**
```bash
npm install @orca-so/whirlpools-sdk @orca-so/common-sdk @coral-xyz/anchor@0.29.0 @solana/web3.js @solana/spl-token decimal.js
```

**Core Utilities (optional):**
```bash
npm install @orca-so/whirlpools-core @orca-so/whirlpools-client
```

### Basic Setup (New SDK)

```typescript
import {
  setWhirlpoolsConfig,
  setRpc,
  setPayerFromBytes
} from "@orca-so/whirlpools";
import { createSolanaRpc, devnet } from "@solana/kit";
import fs from "fs";

// Configure for Solana Devnet
await setWhirlpoolsConfig("solanaDevnet");
await setRpc("https://api.devnet.solana.com");

// Load wallet from keypair file
const keyPairBytes = new Uint8Array(
  JSON.parse(fs.readFileSync("./keypair.json", "utf8"))
);
const wallet = await setPayerFromBytes(keyPairBytes);

// Create RPC connection
const rpc = createSolanaRpc(devnet("https://api.devnet.solana.com"));

console.log("Wallet:", wallet.address);
```

### Basic Setup (Legacy SDK)

```typescript
import { WhirlpoolContext, buildWhirlpoolClient, ORCA_WHIRLPOOL_PROGRAM_ID } from "@orca-so/whirlpools-sdk";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { Connection, Keypair } from "@solana/web3.js";
import fs from "fs";

// Setup connection
const connection = new Connection("https://api.breeze.baby/agent/rpc-mainnet-beta", "confirmed");

// Load wallet
const secretKey = JSON.parse(fs.readFileSync("./keypair.json", "utf8"));
const wallet = new Wallet(Keypair.fromSecretKey(new Uint8Array(secretKey)));

// Create provider and context
const provider = new AnchorProvider(connection, wallet, {});
const ctx = WhirlpoolContext.from(connection, wallet, ORCA_WHIRLPOOL_PROGRAM_ID);
const client = buildWhirlpoolClient(ctx);

console.log("Wallet:", wallet.publicKey.toString());
```

---

## Token Swaps

### Swap with New SDK

```typescript
import { swap, swapInstructions, setWhirlpoolsConfig, setRpc, setPayerFromBytes } from "@orca-so/whirlpools";
import { address, createSolanaRpc } from "@solana/kit";

await setWhirlpoolsConfig("solanaMainnet");
await setRpc(rpcUrl);
const wallet = await setPayerFromBytes(keypairBytes);

const poolAddress = address("POOL_ADDRESS_HERE");
const inputMint = address("INPUT_TOKEN_MINT");
const amount = 1_000_000n; // Amount in smallest units
const slippageTolerance = 100; // 100 bps = 1%

// Option 1: Wrapper function (uses global config — NO rpc, NO wallet)
const result = await swap(
  { inputAmount: amount, mint: inputMint },
  poolAddress,
  slippageTolerance
);
console.log("Expected output:", result.quote.tokenEstOut);
const txId = await result.callback(); // sends the transaction
console.log("Swap transaction:", txId);

// Option 2: Instructions function (explicit rpc + signer)
const rpc = createSolanaRpc(rpcUrl);
const { instructions, quote } = await swapInstructions(
  rpc,
  { inputAmount: amount, mint: inputMint },
  poolAddress,
  slippageTolerance,
  wallet
);
console.log("Expected output:", quote.tokenEstOut);
console.log("Minimum output:", quote.tokenMinOut);
```

### Exact Output Swap

```typescript
// Swap to get exact output amount (wrapper)
const result = await swap(
  { outputAmount: 1_000_000n, mint: outputMint },
  poolAddress,
  slippageTolerance
);
console.log("Max input required:", result.quote.tokenMaxIn);
const txId = await result.callback();
```

### Swap with Legacy SDK

```typescript
import { WhirlpoolContext, swapQuoteByInputToken, buildWhirlpoolClient } from "@orca-so/whirlpools-sdk";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import Decimal from "decimal.js";

// Get the pool
const whirlpool = await client.getPool(poolAddress);
const whirlpoolData = whirlpool.getData();

// Get swap quote
const inputAmount = new Decimal("1.0"); // 1 token
const quote = await swapQuoteByInputToken(
  whirlpool,
  whirlpoolData.tokenMintA,
  DecimalUtil.toBN(inputAmount, 9), // 9 decimals
  Percentage.fromFraction(1, 100), // 1% slippage
  ctx.program.programId,
  ctx.fetcher
);

console.log("Estimated output:", quote.estimatedAmountOut.toString());

// Execute swap
const tx = await whirlpool.swap(quote);
const signature = await tx.buildAndExecute();
console.log("Swap signature:", signature);
```

---

## Liquidity Provision

### Position Types

**Concentrated Liquidity Position**: Provide liquidity within a specific price range for higher capital efficiency.

**Full Range Position (Splash Pool)**: Provide liquidity across the entire price range, similar to traditional AMMs.

### Open Concentrated Liquidity Position

> **IMPORTANT: `IncreaseLiquidityParam` format in SDK v7**
>
> In SDK v7, `openPositionInstructions` / `openConcentratedPosition` internally call `getIncreaseLiquidityInstructions` which destructures `param: { tokenMaxA, tokenMaxB }` directly. Passing `{ tokenA: bigint }` will cause `TypeError: Cannot convert undefined to a BigInt` because `tokenMaxA` ends up as `undefined`.
>
> The correct pattern is to compute the quote first with `increaseLiquidityQuoteA` (or `B`) from `@orca-so/whirlpools-core`, then pass `{ tokenMaxA: quote.tokenMaxA, tokenMaxB: quote.tokenMaxB }`.

```typescript
import { openConcentratedPosition, openPositionInstructions } from "@orca-so/whirlpools";
import { increaseLiquidityQuoteA, priceToTickIndex, getInitializableTickIndex } from "@orca-so/whirlpools-core";
import { address, createSolanaRpc } from "@solana/kit";

const poolAddress = address("POOL_ADDRESS");
const slippageTolerance = 100; // 1%

// Fetch pool to get current price, sqrtPrice, and tickSpacing
const pools = await fetchWhirlpoolsByTokenPair(rpc, tokenMintA, tokenMintB);
const pool = pools.find(p => p.address === "POOL_ADDRESS");

// Compute ticks from price range (use actual token decimals from pool)
const decimalsA = 9; // verify from pool data
const decimalsB = 6;
const lowerTick = getInitializableTickIndex(
  priceToTickIndex(pool.price * 0.75, decimalsA, decimalsB),
  pool.tickSpacing, false
);
const upperTick = getInitializableTickIndex(
  priceToTickIndex(pool.price * 1.25, decimalsA, decimalsB),
  pool.tickSpacing, true
);

// Compute quote — this gives the { tokenMaxA, tokenMaxB } the SDK expects
const solAmount = 10_000_000n; // 0.01 SOL in lamports (tokenA)
const quote = increaseLiquidityQuoteA(
  solAmount,
  slippageTolerance,
  pool.sqrtPrice,
  lowerTick,
  upperTick,
);
const param = { tokenMaxA: quote.tokenMaxA, tokenMaxB: quote.tokenMaxB };

// Option 1: Wrapper (uses global config — NO rpc, NO wallet)
const result = await openConcentratedPosition(
  poolAddress, param, pool.price * 0.75, pool.price * 1.25, slippageTolerance
);
console.log("Position mint:", result.positionMint);
const txId = await result.callback();

// Option 2: Instructions function (explicit rpc + funder)
const rpc = createSolanaRpc(rpcUrl);
const { instructions, positionMint, initializationCost } =
  await openPositionInstructions(rpc, poolAddress, param, pool.price * 0.75, pool.price * 1.25, slippageTolerance, true, wallet);
```

> **SOL balance requirement for `openPositionInstructions`**
>
> `OpenPositionWithTokenExtensions` creates several on-chain accounts (position PDA, Token-2022 NFT mint with metadata, Token-2022 ATA for the NFT). This costs approximately **~10M lamports (~0.01 SOL)** in overhead, independent of the liquidity amount. The `initializationCost` returned by the SDK is incorrectly reported as 0 — do not rely on it.
>
> Budget rule: `walletSOL ≥ 0.01 (overhead) + 0.002 (keypair rent, refunded) + tokenMaxA + txFees`
>
> For a wallet with 0.019 SOL, the maximum depositable SOL is approximately 0.006 SOL.

### Open Full Range Position

```typescript
import { openFullRangePosition, openFullRangePositionInstructions } from "@orca-so/whirlpools";

const poolAddress = address("POOL_ADDRESS");
const param = { tokenA: 1_000_000_000n };
const slippageTolerance = 100;

// Wrapper (NO rpc, NO wallet)
const result = await openFullRangePosition(poolAddress, param, slippageTolerance);
console.log("Position mint:", result.positionMint);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, quote, positionMint, callback } =
  await openFullRangePositionInstructions(rpc, poolAddress, param, slippageTolerance, true, wallet);
const txId2 = await callback();
```

### Increase Position Liquidity

```typescript
import { increasePosLiquidity, increaseLiquidityInstructions } from "@orca-so/whirlpools";

const positionMint = address("POSITION_MINT");
const param = { tokenA: 500_000_000n }; // Add 0.5 tokens
const slippageTolerance = 100;

// Wrapper (NO rpc, NO wallet)
const result = await increasePosLiquidity(positionMint, param, slippageTolerance);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, quote } = await increaseLiquidityInstructions(rpc, positionMint, param, slippageTolerance, wallet);
console.log("Additional Token A:", quote.tokenEstA);
```

### Decrease Position Liquidity

```typescript
import { decreaseLiquidity, decreaseLiquidityInstructions } from "@orca-so/whirlpools";

const positionMint = address("POSITION_MINT");
const param = { liquidity: 1000000n }; // Or use tokenA/tokenB
const slippageTolerance = 100;

// Wrapper (NO rpc, NO wallet)
const result = await decreaseLiquidity(positionMint, param, slippageTolerance);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, quote } = await decreaseLiquidityInstructions(rpc, positionMint, param, slippageTolerance, wallet);
console.log("Token A received:", quote.tokenEstA);
```

### Harvest Fees and Rewards

```typescript
import { harvestPosition, harvestPositionInstructions } from "@orca-so/whirlpools";

const positionMint = address("POSITION_MINT");

// Wrapper (NO rpc, NO wallet)
const result = await harvestPosition(positionMint);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, feesQuote, rewardsQuote } = await harvestPositionInstructions(rpc, positionMint, wallet);
console.log("Fee Token A:", feesQuote.feeOwedA);
console.log("Fee Token B:", feesQuote.feeOwedB);
```

### Close Position

```typescript
import { closePosition, closePositionInstructions } from "@orca-so/whirlpools";

const positionMint = address("POSITION_MINT");
const slippageTolerance = 100;

// Wrapper (NO rpc, NO wallet)
const result = await closePosition(positionMint, slippageTolerance);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, quote, feesQuote } = await closePositionInstructions(rpc, positionMint, slippageTolerance, wallet);
console.log("Token A returned:", quote.tokenEstA);
```

---

## Pool Management

### Create Splash Pool (Full Range)

```typescript
import { createSplashPool, createSplashPoolInstructions } from "@orca-so/whirlpools";
import { address, createSolanaRpc } from "@solana/kit";

const tokenMintA = address("TOKEN_A_MINT");
const tokenMintB = address("TOKEN_B_MINT");
const initialPrice = 1.5;

// Wrapper (NO rpc, NO wallet)
const result = await createSplashPool(tokenMintA, tokenMintB, initialPrice);
console.log("Pool address:", result.poolAddress);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, poolAddress, initializationCost } =
  await createSplashPoolInstructions(rpc, tokenMintA, tokenMintB, initialPrice, wallet);
```

### Create Concentrated Liquidity Pool

```typescript
import { createConcentratedLiquidityPool, createConcentratedLiquidityPoolInstructions } from "@orca-so/whirlpools";

const tokenMintA = address("TOKEN_A_MINT");
const tokenMintB = address("TOKEN_B_MINT");
const tickSpacing = 64; // Common values: 1, 8, 64, 128
const initialPrice = 1.5;

// Wrapper (NO rpc, NO wallet)
const result = await createConcentratedLiquidityPool(tokenMintA, tokenMintB, tickSpacing, initialPrice);
const txId = await result.callback();

// Or instructions variant
const rpc = createSolanaRpc(rpcUrl);
const { instructions, poolAddress } =
  await createConcentratedLiquidityPoolInstructions(rpc, tokenMintA, tokenMintB, tickSpacing, initialPrice, wallet);
```

### Fetch Pool Data

```typescript
import {
  fetchSplashPool,
  fetchConcentratedLiquidityPool,
  fetchWhirlpoolsByTokenPair
} from "@orca-so/whirlpools";

// Fetch specific pool
const pool = await fetchConcentratedLiquidityPool(rpc, poolAddress);
console.log("Current price:", pool.price);
console.log("Liquidity:", pool.liquidity);
console.log("Fee rate:", pool.feeRate);

// Fetch all pools for a token pair
const pools = await fetchWhirlpoolsByTokenPair(rpc, tokenMintA, tokenMintB);
for (const pool of pools) {
  console.log("Pool:", pool.address, "Tick spacing:", pool.tickSpacing);
}
```

### Fetch Positions

```typescript
import { fetchPositionsForOwner, fetchPositionsInWhirlpool } from "@orca-so/whirlpools";

// Get all positions owned by a wallet
const positions = await fetchPositionsForOwner(rpc, wallet.address);
for (const position of positions) {
  console.log("Position:", position.positionMint);
  console.log("Liquidity:", position.liquidity);
  console.log("Lower tick:", position.tickLowerIndex);
  console.log("Upper tick:", position.tickUpperIndex);
}

// Get all positions in a specific pool
const poolPositions = await fetchPositionsInWhirlpool(rpc, poolAddress);
```

---

## SDK Configuration

### Network Configuration

```typescript
import { setWhirlpoolsConfig } from "@orca-so/whirlpools";

// Built-in presets
await setWhirlpoolsConfig("solanaMainnet");
await setWhirlpoolsConfig("solanaDevnet");
await setWhirlpoolsConfig("eclipseMainnet");
await setWhirlpoolsConfig("eclipseTestnet");

// Custom configuration
await setWhirlpoolsConfig({
  whirlpoolsConfigAddress: address("CUSTOM_CONFIG_ADDRESS"),
  // ... other custom settings
});
```

### Transaction Settings

```typescript
import {
  setRpc,
  setPriorityFeeSetting,
  setJitoTipSetting,
  setComputeUnitMarginMultiplier,
  setDefaultSlippageToleranceBps,
  setDefaultFunder,
  setNativeMintWrappingStrategy
} from "@orca-so/whirlpools";

// Set RPC endpoint
await setRpc("https://api.breeze.baby/agent/rpc-mainnet-beta");

// Configure priority fees
await setPriorityFeeSetting({
  type: "dynamic",
  percentile: 75, // Use 75th percentile of recent fees
});

// Or set fixed priority fee
await setPriorityFeeSetting({
  type: "fixed",
  microLamports: 10000,
});

// Configure Jito tips for MEV protection
await setJitoTipSetting({
  type: "dynamic",
  percentile: 50,
});

// Set compute unit margin (for larger transactions)
await setComputeUnitMarginMultiplier(1.2);

// Set default slippage tolerance (in bps)
await setDefaultSlippageToleranceBps(100); // 1%

// Set default funder for transactions
await setDefaultFunder(wallet);

// Configure SOL wrapping behavior — default is "keypair" (recommended)
// WARNING: "ata" strategy has a bug when SOL is tokenA — it reads the USDC
// account balance instead of the wSOL balance, causing InsufficientFunds (Custom 1).
// Do NOT use "ata" for SOL/X pools where SOL is tokenA. Use default "keypair".
await setNativeMintWrappingStrategy("none"); // only if wSOL ATA already funded
```

### Reset Configuration

```typescript
import { resetConfiguration } from "@orca-so/whirlpools";

// Reset all settings to defaults
await resetConfiguration();
```

---

## Tick Spacing Reference

Tick spacing determines the granularity of price ranges:

| Tick Spacing | Use Case | Fee Tier |
|--------------|----------|----------|
| 1 | Stablecoins (e.g., USDC/USDT) | 0.01% |
| 8 | Correlated pairs | 0.04% |
| 64 | Standard pairs | 0.30% |
| 128 | Exotic/volatile pairs | 1.00% |

---

## Best Practices

### Security

1. **Never expose private keys** - Use environment variables or secure key management
2. **Verify pool addresses** - Always confirm you're interacting with legitimate pools
3. **Use slippage protection** - Set appropriate slippage tolerance to prevent front-running
4. **Test on devnet first** - Validate all operations before mainnet deployment

### Performance

1. **Use priority fees** - Essential for mainnet transactions during high congestion
2. **Batch operations** - Combine multiple instructions when possible
3. **Pre-fetch data** - Cache pool and position data to reduce RPC calls

### Liquidity Provision

1. **Monitor positions** - Track when positions go out of range
2. **Rebalance strategically** - Consider gas costs when adjusting positions
3. **Harvest regularly** - Collect fees and rewards to compound returns
4. **Understand impermanent loss** - Concentrated positions have amplified IL risk

### Error Handling

```typescript
try {
  const result = await swap(swapParams, poolAddress, slippage);
  const txId = await result.callback();
} catch (error) {
  if (error.message.includes("SlippageExceeded")) {
    console.error("Slippage tolerance exceeded, try increasing slippage");
  } else if (error.message.includes("InsufficientFunds")) {
    console.error("Not enough tokens in wallet");
  } else if (error.message.includes("TickArrayNotInitialized")) {
    console.error("Price range tick arrays not initialized");
  } else {
    throw error;
  }
}
```

---

## Resources

### Official Documentation
- [Orca Developer Docs](https://dev.orca.so/)
- [TypeScript SDK Reference](https://dev.orca.so/ts/)
- [Rust SDK Reference](https://dev.orca.so/orca_whirlpools_docs/)
- [Legacy SDK Reference](https://dev.orca.so/legacy/)

### GitHub Repositories
- [Whirlpools Monorepo](https://github.com/orca-so/whirlpools)
- [Legacy SDK](https://github.com/orca-so/whirlpool-sdk)
- [Transaction Sender](https://github.com/orca-so/tx-sender)

### NPM Packages
- [@orca-so/whirlpools](https://www.npmjs.com/package/@orca-so/whirlpools) - New SDK
- [@orca-so/whirlpools-sdk](https://www.npmjs.com/package/@orca-so/whirlpools-sdk) - Legacy SDK
- [@orca-so/whirlpools-core](https://www.npmjs.com/package/@orca-so/whirlpools-core) - Core utilities
- [@orca-so/whirlpools-client](https://www.npmjs.com/package/@orca-so/whirlpools-client) - Low-level client

### Security Audits
- Kudelski Security audit
- Neodyme audit

---

## Skill Structure

```
orca/
├── SKILL.md                              # This file
├── resources/
│   ├── api-reference.md                  # Complete SDK function reference
│   └── program-addresses.md              # Program IDs and common pool addresses
├── examples/
│   ├── swap/
│   │   └── token-swap.ts                 # Token swap examples
│   ├── liquidity/
│   │   ├── open-position.ts              # Open liquidity positions
│   │   ├── manage-position.ts            # Increase/decrease liquidity
│   │   └── harvest-fees.ts               # Collect fees and rewards
│   └── pools/
│       └── create-pool.ts                # Pool creation examples
├── templates/
│   └── setup.ts                          # Ready-to-use client template
└── docs/
    └── troubleshooting.md                # Common issues and solutions
```
