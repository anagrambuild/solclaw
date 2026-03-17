# Manifest Troubleshooting Guide

Common issues and solutions when integrating with the Manifest SDK.

## Setup And Account Errors

### "Read only"

**Cause:** A write method was called on a read-only client, or the client was created without wrapper/payer context.

**Solution:**

```typescript
const setup = await ManifestClient.getSetupIxs(
  connection,
  marketPk,
  walletPublicKey
);

if (setup.setupNeeded) {
  // execute setup first, then create the wallet-aware client
}

const client = await ManifestClient.getClientForMarketNoPrivateKey(
  connection,
  marketPk,
  walletPublicKey
);
```

For signer-controlled scripts and bots, use `getClientForMarket(...)` instead.

### Setup still required

**Cause:** Wrapper creation and/or market seat claim has not been completed yet.

**Solution:**

- Call `ManifestClient.getSetupIxs(...)`
- Execute the returned setup instructions
- If `wrapperKeypair` is returned, partially sign the transaction with it
- Only then call `getClientForMarketNoPrivateKey(...)`

### Missing market seat behavior

**Cause:** Market-local flows require a trader seat on the market, but the wallet has not claimed one yet.

**Solution:**

Use the `getSetupIxs(...)` path. Do not assume seat existence from wallet connection alone.

## Global Account Errors

### Global order funded incorrectly

**Cause:** `OrderType.Global` was used without a funded global account for the supporting token.

**Solution:**

```typescript
const addTraderIx = await ManifestClient.createGlobalAddTraderIx(
  trader.publicKey,
  mint
);

const depositIx = await ManifestClient.globalDepositIx(
  connection,
  trader.publicKey,
  mint,
  100
);
```

After setup and deposit, place the order with `OrderType.Global`.

### Market-local balance confusion

**Cause:** Local deposited balances and global balances are being treated as interchangeable.

**Solution:**

- Market-local orders depend on wrapper balances on that market
- Global orders depend on token-level global balances
- Keep those two accounting paths separate in application logic

## Order Management Errors

### Reverse/global orders remain after cancel-all

**Cause:** `cancelAllIx()` is wrapper-based and does not fully clean up all core-level reverse/global edge cases.

**Solution:**

Use `cancelAllOnCoreIx()` when full cleanup is required.

### Unexpected orderbook display values

**Cause:** UI code is using raw orderbook helpers when display-ready levels were intended.

**Solution:**

Use:

```typescript
await client.market.reload(connection);
const bids = client.market.bidsL2();
const asks = client.market.asksL2();
```

Prefer `bidsL2()` / `asksL2()` for UI and price-display flows.

## Integration Advice

- Use `Market.loadFromAddress(...)` or `getClientReadOnly(...)` for read-only pages
- Use `getSetupIxs(...)` for wallet-adapter flows
- Use `getClientForMarket(...)` for signer-controlled automation
- Use `placeOrderWithRequiredDepositIxs(...)` when you want the SDK to help calculate missing funding for a local or global order

---

## Agent/Bot Integration Patterns (from real-world usage)

### ESM/CJS interop: use `createRequire`

`@bonasa-tech/manifest-sdk` is a CJS package. In ESM TypeScript projects (`"type": "module"`), import it with `createRequire`:

```typescript
// @ts-nocheck
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { ManifestClient, OrderType, Market } = require('@bonasa-tech/manifest-sdk');
```

### RPC without WebSocket support (405 errors)

`getClientForMarket(...)` calls `sendAndConfirmTransaction` internally, which requires a WebSocket connection. If your RPC returns 405 on WS upgrades, patch `connection.confirmTransaction` before calling the SDK:

```typescript
connection.confirmTransaction = async (sigOrStrategy: any) => {
  const sig = typeof sigOrStrategy === 'string' ? sigOrStrategy : sigOrStrategy?.signature ?? sigOrStrategy;
  const start = Date.now();
  while (Date.now() - start < 90000) {
    const { value } = await connection.getSignatureStatus(sig, { searchTransactionHistory: true });
    if (value?.confirmationStatus === 'confirmed' || value?.confirmationStatus === 'finalized') {
      return { context: { slot: value.slot }, value: value.err ?? null } as any;
    }
    await new Promise(r => setTimeout(r, 2000));
  }
  return { context: { slot: 0 }, value: null } as any;
};
```

### `depositIx` panics: "range end index 32 out of range for slice of length 0"

**Cause:** For native SOL (wSOL mint), `depositIx` expects the trader's wSOL ATA to already exist and be funded. The wrapper program panics reading empty account data when the ATA is missing.

**Solution:** Create and fund the wSOL ATA before calling `depositIx`:

```typescript
import { createAssociatedTokenAccountIdempotentInstruction, createSyncNativeInstruction, createCloseAccountInstruction, TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { SystemProgram } from '@solana/web3.js';

const SOL_MINT = new PublicKey('So11111111111111111111111111111111111111112');
const wsolAta = getAssociatedTokenAddressSync(SOL_MINT, payer.publicKey, false, TOKEN_PROGRAM_ID);
const lamports = Math.round(amountSol * 1e9);

// Step 1: Create ATA, transfer SOL, sync native
const wrapTx = new Transaction()
  .add(createAssociatedTokenAccountIdempotentInstruction(payer.publicKey, wsolAta, payer.publicKey, SOL_MINT))
  .add(SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: wsolAta, lamports }))
  .add(createSyncNativeInstruction(wsolAta, TOKEN_PROGRAM_ID));
// send wrapTx...

// Step 2: Now depositIx will work
const depositIx = client.depositIx(payer.publicKey, SOL_MINT, amountSol);

// Step 3: After withdrawing SOL back, close the ATA to reclaim rent
const closeTx = new Transaction().add(
  createCloseAccountInstruction(wsolAta, payer.publicKey, payer.publicKey, [], TOKEN_PROGRAM_ID)
);
```

### Picking the right market from `listMarketsForMints`

`listMarketsForMints` returns all markets for a pair — many may have no liquidity. Always pick the one with active bids and asks:

```typescript
const markets = await ManifestClient.listMarketsForMints(connection, baseMint, quoteMint);
let marketPk = markets[0];
let market = await Market.loadFromAddress({ connection, address: marketPk });

for (const mkt of markets.slice(1)) {
  const m = await Market.loadFromAddress({ connection, address: mkt });
  if (m.bestBidPrice() !== undefined && m.bestAskPrice() !== undefined) {
    marketPk = mkt;
    market = m;
    break;
  }
}
```

### `withdrawAllIx()` returns empty array — "No instructions" error

`withdrawAllIx()` returns nothing if the wrapper has no pending balance. After a trade, reload market state and use `withdrawIx` per token:

```typescript
await client.market.reload(connection);
const usdcBal = client.market.getWithdrawableBalanceTokens(payer.publicKey, false); // false = quote
const solBal  = client.market.getWithdrawableBalanceTokens(payer.publicKey, true);  // true = base

const ixs = [];
if (usdcBal > 0) ixs.push(client.withdrawIx(payer.publicKey, USDC_MINT, usdcBal));
if (solBal  > 0) ixs.push(client.withdrawIx(payer.publicKey, SOL_MINT,  solBal));
if (ixs.length > 0) await sendTx(new Transaction().add(...ixs));
```

### IOC orders partially fill — always withdraw both tokens

`OrderType.ImmediateOrCancel` fills against available orderbook depth and cancels the remainder. The unfilled base tokens are returned to the wrapper balance. Always withdraw both base and quote after an IOC sell order.

### Wrapper seat is market-specific

Each market requires its own seat on the wrapper. `getClientForMarket(...)` auto-claims a seat for the given market if needed. When switching between markets in a script, call `getClientForMarket(...)` fresh for each market rather than reusing the same client.
