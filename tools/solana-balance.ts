#!/usr/bin/env npx tsx
/**
 * Check SOL and SPL token balances.
 *
 * Usage:
 *   npx tsx tools/solana-balance.ts                  # SOL balance
 *   npx tsx tools/solana-balance.ts --token USDC     # SPL token balance
 *   npx tsx tools/solana-balance.ts --token <mint>   # SPL token by mint
 *   npx tsx tools/solana-balance.ts --breeze-lending  # Breeze yield positions (Fly.io only)
 */

import { PublicKey } from '@solana/web3.js';
import { loadWallet, resolveMint, COMMON_TOKENS } from './lib/wallet.js';

const args = process.argv.slice(2);
const tokenIdx = args.indexOf('--token');
const tokenArg = tokenIdx !== -1 ? args[tokenIdx + 1] : null;

// --breeze-lending: fetch Breeze yield positions via the dashboard proxy.
// Only available on Fly.io agent machines — not for local/CLI/TG/WhatsApp use.
if (args.includes('--breeze-lending')) {
  if (!process.env.AGENT_ID) {
    console.error(JSON.stringify({
      error: '--breeze-lending is only available on Fly.io agent machines. Use the Breeze MCP tools or x402 endpoint when running locally.',
    }));
    process.exit(1);
  }

  const bs58 = await import('bs58');
  const nacl = await import('tweetnacl');

  const { keypair, publicKey } = loadWallet();
  const wallet = publicKey.toBase58();
  const timestamp = Date.now().toString();
  const message = new TextEncoder().encode(`${wallet}:${timestamp}`);
  const signature = bs58.default.encode(nacl.default.sign.detached(message, keypair.secretKey));

  const url = 'https://www.solclaw.ai/api/agent-proxy/breeze-balances';
  const res = await fetch(url, {
    headers: { 'x-wallet': wallet, 'x-timestamp': timestamp, 'x-signature': signature },
    signal: AbortSignal.timeout(20_000),
  });

  const data = await res.json();
  if (!res.ok) {
    console.error(JSON.stringify({ error: data.error ?? 'Failed to fetch breeze lending balances', status: res.status }));
    process.exit(1);
  }
  console.log(JSON.stringify(data));
  process.exit(0);
}

const { connection, publicKey } = loadWallet();

if (!tokenArg) {
  // SOL balance + all SPL token accounts
  const lamports = await connection.getBalance(publicKey);
  const result: Record<string, unknown> = { sol: lamports / 1e9, address: publicKey.toBase58() };

  // Scan all SPL token accounts (Token Program + Token-2022)
  const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
  const TOKEN_2022_PROGRAM_ID = new PublicKey('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');
  try {
    const [legacyAccounts, token2022Accounts] = await Promise.all([
      connection.getParsedTokenAccountsByOwner(publicKey, { programId: TOKEN_PROGRAM_ID }),
      connection.getParsedTokenAccountsByOwner(publicKey, { programId: TOKEN_2022_PROGRAM_ID }),
    ]);
    const tokenAccounts = { value: [...legacyAccounts.value, ...token2022Accounts.value] };
    const tokens: Array<{ mint: string; symbol: string | null; balance: number; decimals: number }> = [];

    // Reverse lookup: mint → symbol
    const mintToSymbol: Record<string, string> = {};
    for (const [symbol, mint] of Object.entries(COMMON_TOKENS)) {
      mintToSymbol[mint] = symbol;
    }

    for (const { account } of tokenAccounts.value) {
      const parsed = account.data.parsed?.info;
      if (!parsed) continue;
      const balance = parsed.tokenAmount?.uiAmount ?? 0;
      if (balance === 0) continue; // skip zero-balance accounts
      const mint = parsed.mint as string;
      tokens.push({
        mint,
        symbol: mintToSymbol[mint] ?? null,
        balance,
        decimals: parsed.tokenAmount?.decimals ?? 0,
      });
    }

    if (tokens.length > 0) {
      result.tokens = tokens;
    }
  } catch {
    // Token scan failed — still return SOL balance
  }

  console.log(JSON.stringify(result));
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
