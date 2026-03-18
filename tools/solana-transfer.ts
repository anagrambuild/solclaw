#!/usr/bin/env npx tsx
/**
 * Fast transfer of SOL or SPL tokens.
 *
 * Usage:
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 0.5
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 10 --token USDC
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 10 --token <mint>
 */

import {
  ComputeBudgetProgram,
  PublicKey,
  SystemProgram,
  Transaction,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import {
  getAssociatedTokenAddress,
  createTransferInstruction,
  createAssociatedTokenAccountInstruction,
  getAccount,
  TokenAccountNotFoundError,
} from '@solana/spl-token';
import { loadWallet, resolveMint, logTransactionIpc, COMMON_TOKENS } from './lib/wallet.js';

const PRIORITY_FEE_MICRO_LAMPORTS = 1_000; // ~0.0000001 SOL per CU, fast inclusion

function parseArgs(args: string[]): { to: string; amount: string; token?: string } {
  let to = '', amount = '', token: string | undefined;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--to' && args[i + 1]) to = args[++i];
    else if (args[i] === '--amount' && args[i + 1]) amount = args[++i];
    else if (args[i] === '--token' && args[i + 1]) token = args[++i];
  }
  if (!to || !amount) {
    console.error('Usage: npx tsx tools/solana-transfer.ts --to <address> --amount 0.5 [--token USDC]');
    process.exit(1);
  }
  return { to, amount, token };
}

/** Send tx with skipPreflight + priority fee, confirm via blockhash expiry. */
async function sendFast(tx: Transaction): Promise<string> {
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');
  tx.recentBlockhash = blockhash;
  tx.feePayer = publicKey;
  tx.sign(keypair);

  const signature = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: false,
    maxRetries: 3,
  });

  await connection.confirmTransaction(
    { signature, blockhash, lastValidBlockHeight },
    'confirmed',
  );

  return signature;
}

const { to, amount, token } = parseArgs(process.argv.slice(2));
const { keypair, connection, publicKey } = loadWallet();
const recipient = new PublicKey(to);

if (!token) {
  // SOL transfer
  const lamports = Math.round(parseFloat(amount) * LAMPORTS_PER_SOL);
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: PRIORITY_FEE_MICRO_LAMPORTS }),
    SystemProgram.transfer({
      fromPubkey: publicKey,
      toPubkey: recipient,
      lamports,
    }),
  );

  const signature = await sendFast(tx);
  logTransactionIpc(signature, 'system-program', publicKey.toBase58(), COMMON_TOKENS.SOL, amount);
  console.log(JSON.stringify({
    signature,
    amount: `${amount} SOL`,
    to: recipient.toBase58(),
    explorer: `https://solscan.io/tx/${signature}`,
  }));
} else {
  // SPL token transfer
  const mint = new PublicKey(resolveMint(token));
  const knownDecimals: Record<string, number> = {
    [COMMON_TOKENS.USDC]: 6,
    [COMMON_TOKENS.USDT]: 6,
    [COMMON_TOKENS.BONK]: 5,
    [COMMON_TOKENS.JUP]: 6,
  };
  const decimals = knownDecimals[mint.toBase58()] ?? 9;
  const rawAmount = BigInt(Math.round(parseFloat(amount) * 10 ** decimals));

  // Fetch ATAs and check recipient in parallel
  const [senderAta, recipientAta] = await Promise.all([
    getAssociatedTokenAddress(mint, publicKey),
    getAssociatedTokenAddress(mint, recipient),
  ]);

  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: PRIORITY_FEE_MICRO_LAMPORTS }),
  );

  // Create recipient ATA if needed
  try {
    await getAccount(connection, recipientAta);
  } catch (e) {
    if (e instanceof TokenAccountNotFoundError) {
      tx.add(
        createAssociatedTokenAccountInstruction(publicKey, recipientAta, recipient, mint),
      );
    } else {
      throw e;
    }
  }

  tx.add(createTransferInstruction(senderAta, recipientAta, publicKey, rawAmount));

  const signature = await sendFast(tx);
  logTransactionIpc(signature, 'token-program', publicKey.toBase58(), mint.toBase58(), amount);
  console.log(JSON.stringify({
    signature,
    amount: `${amount} ${token}`,
    to: recipient.toBase58(),
    explorer: `https://solscan.io/tx/${signature}`,
  }));
}
