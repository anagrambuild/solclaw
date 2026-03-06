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

const IPC_TRANSACTIONS_DIR = '/workspace/ipc/transactions';

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
      protocol,
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
}
