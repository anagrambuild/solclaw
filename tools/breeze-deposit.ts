#!/usr/bin/env npx tsx
/**
 * Deposit into Breeze strategy via x402 payment-gated API.
 *
 * Usage:
 *   npx tsx tools/breeze-deposit.ts --amount 1 --token SOL
 *   npx tsx tools/breeze-deposit.ts --amount 10 --token USDC
 *   npx tsx tools/breeze-deposit.ts --amount 5 --token JitoSOL
 *   npx tsx tools/breeze-deposit.ts --amount 100 --token USDC --strategy <id>
 */

import { wrap } from '@faremeter/fetch';
import { createPaymentHandler } from '@faremeter/payment-solana/exact';
import { createLocalWallet } from '@faremeter/wallet-solana';
import { PublicKey, VersionedTransaction, Transaction } from '@solana/web3.js';
import { loadWallet, logTransactionIpc } from './lib/wallet.js';

const SUPPORTED_TOKENS: Record<string, { mint: string; decimals: number }> = {
  USDC:    { mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', decimals: 6 },
  USDT:    { mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', decimals: 6 },
  USDS:    { mint: 'USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA', decimals: 6 },
  SOL:     { mint: 'So11111111111111111111111111111111111111112', decimals: 9 },
  JITOSOL: { mint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn', decimals: 9 },
  MSOL:    { mint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So', decimals: 9 },
  JUPSOL:  { mint: 'jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v', decimals: 9 },
  JLP:     { mint: '27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4', decimals: 6 },
};

const DEFAULT_STRATEGY = '43620ba3-354c-456b-aa3c-5bf7fa46a6d4';
const API_URL = (process.env.X402_API_URL ?? 'https://x402.breeze.baby').replace(/\/$/, '');
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

function parseArgs(args: string[]) {
  let amount = '', token = 'SOL', strategy = DEFAULT_STRATEGY;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--amount' && args[i + 1]) amount = args[++i];
    else if (args[i] === '--token' && args[i + 1]) token = args[++i].toUpperCase();
    else if (args[i] === '--strategy' && args[i + 1]) strategy = args[++i];
  }
  if (!amount) {
    console.error('Usage: npx tsx tools/breeze-deposit.ts --amount 1 --token SOL [--strategy <id>]');
    console.error(`Supported tokens: ${Object.keys(SUPPORTED_TOKENS).join(', ')}`);
    process.exit(1);
  }
  return { amount, token, strategy };
}

const { amount, token, strategy } = parseArgs(process.argv.slice(2));
const tokenInfo = SUPPORTED_TOKENS[token];
if (!tokenInfo) {
  console.error(`Unsupported token: ${token}. Supported: ${Object.keys(SUPPORTED_TOKENS).join(', ')}`);
  process.exit(1);
}

const { keypair, connection, publicKey } = loadWallet();
const baseUnits = Math.floor(parseFloat(amount) * 10 ** tokenInfo.decimals);

// Setup x402 payment
const wallet = await createLocalWallet('mainnet-beta', keypair);
const paymentHandler = createPaymentHandler(wallet, new PublicKey(USDC_MINT), connection);
const fetchWithPayment = wrap(fetch, { handlers: [paymentHandler] });

// Request deposit transaction
const res = await fetchWithPayment(`${API_URL}/deposit`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    amount: baseUnits,
    user_key: publicKey.toBase58(),
    strategy_id: strategy,
    base_asset: tokenInfo.mint,
  }),
});

if (!res.ok) {
  const text = await res.text();
  console.error(`Deposit failed (${res.status}): ${text}`);
  process.exit(1);
}

// Parse and sign transaction
const raw = (await res.text()).trim();
const txString = raw.startsWith('"') ? JSON.parse(raw) : raw;
const bytes = Buffer.from(txString, 'base64');

const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');

let sig: string;
try {
  const tx = VersionedTransaction.deserialize(bytes);
  tx.sign([keypair]);
  sig = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: false, maxRetries: 5 });
} catch {
  const tx = Transaction.from(bytes);
  tx.partialSign(keypair);
  sig = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: false, maxRetries: 5 });
}

logTransactionIpc(sig, 'breeze', publicKey.toBase58(), tokenInfo.mint, amount);

// Confirm in background — don't block output
connection.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, 'confirmed')
  .then(() => console.error(`[confirmed] ${sig}`))
  .catch((err) => console.error(`[confirm-failed] ${sig}: ${err}`));

console.log(JSON.stringify({
  action: 'deposit',
  signature: sig,
  amount: `${amount} ${token}`,
  baseUnits,
  strategy,
  explorer: `https://solscan.io/tx/${sig}`,
}));
process.exit(0);
