#!/usr/bin/env npx tsx
import { wrap } from '@faremeter/fetch';
import { createPaymentHandler } from '@faremeter/payment-solana/exact';
import { createLocalWallet } from '@faremeter/wallet-solana';
import { Connection, Keypair, PublicKey, VersionedTransaction, Transaction } from '@solana/web3.js';
import { loadWallet } from './lib/wallet.js';

async function depositToBreeze(amountUSDC: number) {
  try {
    const { keypair, rpcUrl } = loadWallet();

    const API_URL = 'https://x402.breeze.baby';
    const STRATEGY_ID = '43620ba3-354c-456b-aa3c-5bf7fa46a6d4';
    const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

    // Convert to base units (USDC has 6 decimals)
    const DEPOSIT_AMOUNT = Math.floor(amountUSDC * 1_000_000);

    console.error(`Depositing ${amountUSDC} USDC (${DEPOSIT_AMOUNT} base units) to Breeze...`);

    // Setup
    const connection = new Connection(rpcUrl);
    const wallet = await createLocalWallet('mainnet-beta', keypair);
    const paymentHandler = createPaymentHandler(wallet, new PublicKey(USDC_MINT), connection);
    const fetchWithPayment = wrap(fetch, { handlers: [paymentHandler] });

    // Build deposit transaction
    console.error('Building deposit transaction...');
    const res = await fetchWithPayment(`${API_URL}/deposit`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        amount: DEPOSIT_AMOUNT,
        user_key: keypair.publicKey.toBase58(),
        strategy_id: STRATEGY_ID,
        base_asset: USDC_MINT,
      }),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Deposit failed (${res.status}): ${text}`);
    }

    // Parse transaction string (may be bare or JSON-wrapped)
    const raw = (await res.text()).trim();
    const txString = raw.startsWith('"') ? JSON.parse(raw) : raw;

    console.error('Signing and sending transaction...');

    // Sign and send (try versioned tx first, fall back to legacy)
    const bytes = Buffer.from(txString, 'base64');
    let sig: string;

    try {
      const tx = VersionedTransaction.deserialize(bytes);
      tx.sign([keypair]);
      sig = await connection.sendRawTransaction(tx.serialize());
    } catch {
      const tx = Transaction.from(bytes);
      tx.partialSign(keypair);
      sig = await connection.sendRawTransaction(tx.serialize());
    }

    console.error('Confirming transaction...');
    await connection.confirmTransaction(sig, 'confirmed');

    console.log(JSON.stringify({
      success: true,
      action: 'deposit',
      amount: amountUSDC,
      baseUnits: DEPOSIT_AMOUNT,
      token: 'USDC',
      signature: sig,
      explorer: `https://solscan.io/tx/${sig}`,
    }, null, 2));
  } catch (error: any) {
    console.error('Error:', error.message);
    console.log(JSON.stringify({
      success: false,
      error: error.message,
    }, null, 2));
    process.exit(1);
  }
}

// Get amount from command line or default to all USDC
const amount = process.argv[2] ? parseFloat(process.argv[2]) : 1.0;
depositToBreeze(amount);
