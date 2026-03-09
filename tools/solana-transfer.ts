#!/usr/bin/env npx tsx
/**
 * Transfer SOL or SPL tokens.
 *
 * Usage:
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 0.5
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 10 --token USDC
 *   npx tsx tools/solana-transfer.ts --to <address> --amount 10 --token <mint>
 */

import {
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
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

const { to, amount, token } = parseArgs(process.argv.slice(2));
const { keypair, connection, publicKey } = loadWallet();
const recipient = new PublicKey(to);

if (!token) {
  // SOL transfer
  const lamports = Math.round(parseFloat(amount) * LAMPORTS_PER_SOL);
  const tx = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: publicKey,
      toPubkey: recipient,
      lamports,
    }),
  );

  const signature = await sendAndConfirmTransaction(connection, tx, [keypair]);
  logTransactionIpc(signature, 'system', publicKey.toBase58(), COMMON_TOKENS.SOL, amount);
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

  const senderAta = await getAssociatedTokenAddress(mint, publicKey);
  const recipientAta = await getAssociatedTokenAddress(mint, recipient);

  const tx = new Transaction();

  // Create recipient ATA if it doesn't exist
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

  const signature = await sendAndConfirmTransaction(connection, tx, [keypair]);
  logTransactionIpc(signature, 'spl-transfer', publicKey.toBase58(), mint.toBase58(), amount);
  console.log(JSON.stringify({
    signature,
    amount: `${amount} ${token}`,
    to: recipient.toBase58(),
    explorer: `https://solscan.io/tx/${signature}`,
  }));
}
