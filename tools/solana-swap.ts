#!/usr/bin/env npx tsx
/**
 * Swap tokens via Jupiter Ultra API.
 *
 * Usage:
 *   npx tsx tools/solana-swap.ts --from SOL --to USDC --amount 0.03
 *   npx tsx tools/solana-swap.ts --from USDC --to SOL --amount 5
 */

import { VersionedTransaction } from '@solana/web3.js';
import { loadWallet, resolveMint, jupiterBase, COMMON_TOKENS } from './lib/wallet.js';

function parseArgs(args: string[]): { from: string; to: string; amount: string } {
  let from = '', to = '', amount = '';
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--from' && args[i + 1]) from = args[++i];
    else if (args[i] === '--to' && args[i + 1]) to = args[++i];
    else if (args[i] === '--amount' && args[i + 1]) amount = args[++i];
  }
  if (!from || !to || !amount) {
    console.error('Usage: npx tsx tools/solana-swap.ts --from SOL --to USDC --amount 0.03');
    process.exit(1);
  }
  return { from, to, amount };
}

const { from, to, amount } = parseArgs(process.argv.slice(2));
const { keypair, connection, publicKey } = loadWallet();
const base = jupiterBase();

const inputMint = resolveMint(from);
const outputMint = resolveMint(to);

// Determine decimals for input token
const knownDecimals: Record<string, number> = {
  [COMMON_TOKENS.USDC]: 6,
  [COMMON_TOKENS.USDT]: 6,
  [COMMON_TOKENS.BONK]: 5,
  [COMMON_TOKENS.JUP]: 6,
};
const inputDecimals = knownDecimals[inputMint] ?? 9;
const inputAmount = Math.round(parseFloat(amount) * 10 ** inputDecimals);

// Step 1: Get order
const headers: Record<string, string> = {};
if (process.env.JUPITER_API_KEY) {
  headers['x-api-key'] = process.env.JUPITER_API_KEY;
}

const orderUrl = `${base}/ultra/v1/order?inputMint=${inputMint}&outputMint=${outputMint}&amount=${inputAmount}&taker=${publicKey.toBase58()}`;
const orderRes = await fetch(orderUrl, { headers });

if (!orderRes.ok) {
  const text = await orderRes.text();
  console.error(`Jupiter order error: ${orderRes.status} ${text}`);
  process.exit(1);
}

const order = await orderRes.json() as any;

if (!order.transaction) {
  console.error('Jupiter returned no transaction:', JSON.stringify(order));
  process.exit(1);
}

// Step 2: Sign the transaction
const txBuf = Buffer.from(order.transaction, 'base64');
const tx = VersionedTransaction.deserialize(txBuf);
tx.sign([keypair]);

const signedTxBase64 = Buffer.from(tx.serialize()).toString('base64');

// Step 3: Execute
const executeRes = await fetch(`${base}/ultra/v1/execute`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', ...headers },
  body: JSON.stringify({
    signedTransaction: signedTxBase64,
    requestId: order.requestId,
  }),
});

if (!executeRes.ok) {
  const text = await executeRes.text();
  console.error(`Jupiter execute error: ${executeRes.status} ${text}`);
  process.exit(1);
}

const result = await executeRes.json() as any;

console.log(JSON.stringify({
  signature: result.signature ?? result.txid,
  status: result.status ?? 'submitted',
  inputAmount: `${amount} ${from}`,
  outputAmount: result.outputAmount ?? order.outAmount ?? null,
  explorer: result.signature ? `https://solscan.io/tx/${result.signature}` : null,
}));
