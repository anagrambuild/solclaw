import {
  getUnsyncedTransactions,
  markTransactionsSynced,
  markTransactionSyncError,
} from './db.js';
import { logger } from './logger.js';

const TRANSACTION_SYNC_API_URL = process.env.TRANSACTION_SYNC_API_URL || 'https://api.breeze.baby/agent/stats-sync-up';
const TRANSACTION_SYNC_INTERVAL = parseInt(process.env.TRANSACTION_SYNC_INTERVAL || '3600000', 10);

let syncRunning = false;

export function startTransactionSyncLoop(): void {
  if (syncRunning) {
    logger.debug('Transaction sync loop already running, skipping duplicate start');
    return;
  }
  syncRunning = true;
  logger.info('Transaction sync loop started');

  const loop = async () => {
    try {
      if (!TRANSACTION_SYNC_API_URL) {
        // No API configured — skip sync, records accumulate locally
        setTimeout(loop, TRANSACTION_SYNC_INTERVAL);
        return;
      }

      const records = getUnsyncedTransactions();
      if (records.length === 0) {
        setTimeout(loop, TRANSACTION_SYNC_INTERVAL);
        return;
      }

      logger.info({ count: records.length }, 'Syncing transactions to API');

      const ids = records.map((r) => r.id!);

      const transactionEntries = records.map((r) => {
        const entry: {
          signature: string;
          protocol: string;
          wallet_address: string;
          mint?: string;
          amount?: number;
        } = {
          signature: r.signature,
          protocol: r.protocol,
          wallet_address: r.wallet_address,
        };
        if (r.mint) entry.mint = r.mint;
        if (r.amount) entry.amount = parseFloat(r.amount);
        return entry;
      });

      try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 30000);

        const response = await fetch(TRANSACTION_SYNC_API_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ transaction_entries: transactionEntries }),
          signal: controller.signal,
        });

        clearTimeout(timeout);

        if (!response.ok) {
          const errorText = await response.text().catch(() => 'Unknown error');
          throw new Error(`API returned ${response.status}: ${errorText}`);
        }

        markTransactionsSynced(ids);
        logger.info({ count: ids.length }, 'Transactions synced successfully');
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        markTransactionSyncError(ids, errorMsg);
        logger.error({ error: errorMsg, count: ids.length }, 'Transaction sync failed');
      }
    } catch (err) {
      logger.error({ err }, 'Error in transaction sync loop');
    }

    setTimeout(loop, TRANSACTION_SYNC_INTERVAL);
  };

  loop();
}
