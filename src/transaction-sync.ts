import {
  getUnsyncedTransactions,
  markTransactionsSynced,
  markTransactionSyncError,
} from './db.js';
import { logger } from './logger.js';
import {
  trackTransactionSynced,
  trackTransactionSyncError,
} from './metrics.js';
import type { TransactionRecord } from './types.js';

const TRANSACTION_SYNC_API_URL =
  process.env.TRANSACTION_SYNC_API_URL ||
  'https://www.solclaw.ai/api/agent/transactions';
const TRANSACTION_SYNC_INTERVAL = parseInt(
  process.env.TRANSACTION_SYNC_INTERVAL || '300000',
  10,
);
const TRANSACTION_SYNC_RETRY_DELAY = parseInt(
  process.env.TRANSACTION_SYNC_RETRY_DELAY || '30000',
  10,
);
const TRANSACTION_SYNC_REQUEST_TIMEOUT = parseInt(
  process.env.TRANSACTION_SYNC_REQUEST_TIMEOUT || '30000',
  10,
);
const TRANSACTION_SYNC_BATCH_SIZE = parseInt(
  process.env.TRANSACTION_SYNC_BATCH_SIZE || '100',
  10,
);

let syncRunning = false;

interface TransactionSyncEntry {
  signature: string;
  protocol: string;
  wallet_address: string;
}

function safeError(
  message: string,
  err: unknown,
  extra: Record<string, unknown> = {},
): void {
  try {
    logger.error({ ...extra, err }, message);
  } catch {
    // Never let logging kill the sync loop.
  }
}

function toTransactionEntry(record: TransactionRecord): TransactionSyncEntry {
  return {
    signature: record.signature,
    protocol: record.protocol,
    wallet_address: record.wallet_address,
  };
}

async function postTransactionEntries(
  entries: TransactionSyncEntry[],
): Promise<void> {
  const controller = new AbortController();
  const timeout = setTimeout(
    () => controller.abort(),
    TRANSACTION_SYNC_REQUEST_TIMEOUT,
  );

  try {
    const response = await fetch(TRANSACTION_SYNC_API_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ transaction_entries: entries }),
      signal: controller.signal,
    });

    if (!response.ok) {
      const errorText = await response.text().catch(() => 'Unknown error');
      throw new Error(`API returned ${response.status}: ${errorText}`);
    }
  } finally {
    clearTimeout(timeout);
  }
}

export function startTransactionSyncLoop(): void {
  if (syncRunning) {
    logger.debug(
      'Transaction sync loop already running, skipping duplicate start',
    );
    return;
  }
  syncRunning = true;
  logger.info('Transaction sync loop started');

  const loop = async () => {
    let nextDelay = TRANSACTION_SYNC_INTERVAL;

    try {
      const records = getUnsyncedTransactions(TRANSACTION_SYNC_BATCH_SIZE);
      if (records.length === 0) {
        return;
      }

      logger.info({ count: records.length }, 'Syncing transactions to API');

      const validRecords: TransactionRecord[] = [];
      const transactionEntries: TransactionSyncEntry[] = [];

      for (const record of records) {
        try {
          transactionEntries.push(toTransactionEntry(record));
          validRecords.push(record);
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          if (record.id !== undefined) {
            markTransactionSyncError([record.id], errorMsg);
          }
          safeError('Skipping invalid transaction during sync', err, {
            signature: record.signature,
          });
        }
      }

      if (validRecords.length === 0) {
        nextDelay = TRANSACTION_SYNC_RETRY_DELAY;
        return;
      }

      const validIds = validRecords
        .map((record) => record.id)
        .filter((id): id is number => id !== undefined);

      try {
        await postTransactionEntries(transactionEntries);
        markTransactionsSynced(validIds);
        trackTransactionSynced({ count: validIds.length });
        logger.info(
          { count: validIds.length },
          'Transactions synced successfully',
        );
        if (records.length === TRANSACTION_SYNC_BATCH_SIZE) {
          nextDelay = 1000;
        }
      } catch (batchErr) {
        const batchErrorMsg =
          batchErr instanceof Error ? batchErr.message : String(batchErr);
        trackTransactionSyncError({
          count: validIds.length,
          error: batchErrorMsg,
        });
        logger.warn(
          { error: batchErrorMsg, count: validIds.length },
          'Batch transaction sync failed, retrying records individually',
        );

        let failedCount = 0;

        for (const record of validRecords) {
          const recordId = record.id;
          if (recordId === undefined) {
            continue;
          }

          try {
            await postTransactionEntries([toTransactionEntry(record)]);
            markTransactionsSynced([recordId]);
          } catch (recordErr) {
            failedCount += 1;
            const errorMsg =
              recordErr instanceof Error
                ? recordErr.message
                : String(recordErr);
            markTransactionSyncError([recordId], errorMsg);
            safeError('Transaction sync failed for record', recordErr, {
              signature: record.signature,
              id: recordId,
            });
          }
        }

        if (failedCount === 0) {
          logger.info(
            { count: validIds.length },
            'Transactions synced successfully after per-record retry',
          );
          if (records.length === TRANSACTION_SYNC_BATCH_SIZE) {
            nextDelay = 1000;
          }
        } else {
          nextDelay = TRANSACTION_SYNC_RETRY_DELAY;
        }
      }
    } catch (err) {
      nextDelay = TRANSACTION_SYNC_RETRY_DELAY;
      safeError('Error in transaction sync loop', err);
    } finally {
      setTimeout(loop, nextDelay);
    }
  };

  void loop();
}
