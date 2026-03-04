#!/usr/bin/env npx tsx
/**
 * Check SOL and SPL token balances.
 *
 * Usage:
 *   npx tsx tools/solana-balance.ts                  # SOL balance
 *   npx tsx tools/solana-balance.ts --token USDC     # SPL token balance
 *   npx tsx tools/solana-balance.ts --token <mint>   # SPL token by mint
 */

import { PublicKey } from '@solana/web3.js';
import { loadWallet, resolveMint, COMMON_TOKENS } from './lib/wallet.js';

const args = process.argv.slice(2);
const tokenIdx = args.indexOf('--token');
const tokenArg = tokenIdx !== -1 ? args[tokenIdx + 1] : null;

const { connection, publicKey } = loadWallet();

if (!tokenArg) {
  // SOL balance
  const lamports = await connection.getBalance(publicKey);
  console.log(JSON.stringify({ sol: lamports / 1e9, address: publicKey.toBase58() }));
} else {
  const mint = resolveMint(tokenArg);
  const mintPubkey = new PublicKey(mint);

  const accounts = await connection.getTokenAccountsByOwner(publicKey, { mint: mintPubkey });

  if (accounts.value.length === 0) {
    console.log(JSON.stringify({ token: tokenArg, mint, balance: 0, address: publicKey.toBase58() }));
  } else {
    // Parse token account data (SPL Token layout: 64 bytes offset for amount, u64 LE)
    const data = accounts.value[0].account.data;
    const amount = data.readBigUInt64LE(64);
    // Assume 6 decimals for USDC/USDT, 9 for others — or fetch from mint account
    const knownDecimals: Record<string, number> = {
      [COMMON_TOKENS.USDC]: 6,
      [COMMON_TOKENS.USDT]: 6,
      [COMMON_TOKENS.BONK]: 5,
      [COMMON_TOKENS.JUP]: 6,
    };
    const decimals = knownDecimals[mint] ?? 9;
    const balance = Number(amount) / 10 ** decimals;

    console.log(JSON.stringify({ token: tokenArg, mint, balance, address: publicKey.toBase58() }));
  }
}
