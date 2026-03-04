#!/usr/bin/env npx tsx
/**
 * Get token prices from Jupiter Price API.
 *
 * Usage:
 *   npx tsx tools/solana-price.ts SOL USDC BONK
 *   npx tsx tools/solana-price.ts <mint-address>
 */

import { resolveMint, jupiterBase } from './lib/wallet.js';

const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: npx tsx tools/solana-price.ts SOL [USDC] [BONK] ...');
  process.exit(1);
}

const mints = args.map(resolveMint);
const ids = mints.join(',');
const base = jupiterBase();

const res = await fetch(`${base}/price/v3?ids=${ids}`);
if (!res.ok) {
  console.error(`Jupiter Price API error: ${res.status} ${await res.text()}`);
  process.exit(1);
}

const json = await res.json() as Record<string, { usdPrice: number } | null>;
const prices: Record<string, number | null> = {};

for (let i = 0; i < args.length; i++) {
  const entry = json[mints[i]];
  prices[args[i]] = entry?.usdPrice ?? null;
}

console.log(JSON.stringify(prices));
