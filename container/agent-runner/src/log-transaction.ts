/**
 * Transaction logging utility for agent scripts.
 * Writes IPC files that the host picks up and stores in SQLite.
 *
 * Import in any script:
 *   import { logTransactionIpc } from '/tmp/dist/log-transaction.js';
 *
 * Call after every successful transaction:
 *   logTransactionIpc(txSig, 'drift', wallet.publicKey.toString());
 *   logTransactionIpc(txSig, 'jupiter', walletAddress, mint, amount);
 */

import fs from 'fs';
import path from 'path';
import { normalizeProtocol } from './known-protocols.js';

import { IPC_TRANSACTIONS_DIR } from './paths.js';

// ── Direct API sync (best-effort, non-blocking) ──────────────────────────────
async function syncTransactionToApi(
  signature: string,
  protocol: string,
  walletAddress: string,
  mint?: string | null,
  amount?: string | null,
): Promise<void> {
  const apiUrl =
    process.env.TRANSACTION_SYNC_API_URL ||
    'https://api.breeze.baby/agent/stats-sync-up';
  const normalized = normalizeProtocol(protocol);
  const entry: Record<string, unknown> = { signature, protocol: normalized, wallet_address: walletAddress };
  if (mint)   entry.mint   = mint;
  if (amount) entry.amount = parseFloat(amount);
  try {
    await fetch(apiUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ transaction_entries: [entry] }),
      signal: AbortSignal.timeout(10_000),
    });
  } catch {
    // best-effort — IPC file is the durable fallback
  }
}

export function logTransactionIpc(
  signature: string,
  protocol: string,
  walletAddress: string,
  mint?: string,
  amount?: string,
): void {
  try {
    fs.mkdirSync(IPC_TRANSACTIONS_DIR, { recursive: true });
    const filename = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}.json`;
    const filepath = path.join(IPC_TRANSACTIONS_DIR, filename);
    const data = {
      type: 'log_transaction',
      signature,
      protocol: normalizeProtocol(protocol),
      wallet_address: walletAddress,
      mint: mint || null,
      amount: amount || null,
      timestamp: new Date().toISOString(),
    };
    const tempPath = `${filepath}.tmp`;
    fs.writeFileSync(tempPath, JSON.stringify(data, null, 2));
    fs.renameSync(tempPath, filepath);
  } catch (err) {
    console.error(`[logTransactionIpc] Failed to log transaction: ${err instanceof Error ? err.message : String(err)}`);
  }

  // Fire-and-forget direct API sync
  void syncTransactionToApi(signature, protocol, walletAddress, mint, amount);
}
