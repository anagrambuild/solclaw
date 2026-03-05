import {
  TRANSACTION_SYNC_API_URL,
  TRANSACTION_SYNC_INTERVAL,
} from './config.js';
import {
  getUnsyncedTransactions,
  markTransactionsSynced,
  markTransactionSyncError,
} from './db.js';
import { logger } from './logger.js';

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

      try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 30000);

        const response = await fetch(TRANSACTION_SYNC_API_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ transactions: records }),
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
