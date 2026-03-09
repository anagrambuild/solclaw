#!/usr/bin/env npx tsx
/**
 * Find SOL/USDC pools on Orca Whirlpools
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createSolanaRpc, address } from '@solana/kit';
import { fetchWhirlpoolsByTokenPair } from '@orca-so/whirlpools';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const config = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../config/solana-config.json'), 'utf8'));
const rpcUrl = config.preferences?.rpcUrl || 'https://api.mainnet-beta.solana.com';
const rpc = createSolanaRpc(rpcUrl);

const SOL = address('So11111111111111111111111111111111111111112');
const USDC = address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');

const pools = await fetchWhirlpoolsByTokenPair(rpc, SOL, USDC);
const out = pools.map((p: any) => ({
  address: p.address,
  tickSpacing: p.tickSpacing,
  liquidity: p.liquidity?.toString(),
  price: p.price,
  feeRate: p.feeRate,
})).sort((a: any, b: any) => Number(BigInt(b.liquidity || 0) - BigInt(a.liquidity || 0)));

console.log(JSON.stringify(out.slice(0, 5), null, 2));
