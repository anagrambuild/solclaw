/**
 * Axiom Metrics for SolClaw
 *
 * Design: every metric is a point-in-time event ingested into Axiom.
 * Axiom handles aggregation (count, distinct_count, sum, avg, percentiles)
 * at query time via APL.
 *
 * Key metrics enabled:
 *   - Users: distinct_count(userId) over time windows → DAU / WAU / MAU
 *   - Agents: distinct_count(groupFolder) → total agents
 *   - Agents per user: group by userId, distinct_count(groupFolder)
 *   - Volume: count of message.inbound / message.outbound events
 *   - Transactions: count of transaction.logged events, sum(amount)
 *   - Retention: 7d/30d activity = users with agent.invocation events in rolling windows
 *   - DAU gauge: periodic snapshot emitted every 5 min for dashboard gauges
 *
 * Axiom buffers events in memory and flushes periodically.
 * If AXIOM_TOKEN is not set, all emit calls are silent no-ops.
 */

import { Axiom } from '@axiomhq/js';

import { readEnvFile } from './env.js';
import { logger } from './logger.js';

// Read Axiom config from .env (not process.env, following SolClaw conventions)
const axiomEnv = readEnvFile(['AXIOM_TOKEN', 'AXIOM_DATASET']);
const AXIOM_TOKEN = process.env.AXIOM_TOKEN || axiomEnv.AXIOM_TOKEN || '';
const AXIOM_DATASET =
  process.env.AXIOM_DATASET || axiomEnv.AXIOM_DATASET || 'solclaw';

let axiom: Axiom | null = null;

if (AXIOM_TOKEN) {
  axiom = new Axiom({ token: AXIOM_TOKEN });
  logger.info({ dataset: AXIOM_DATASET }, 'Axiom metrics enabled');
} else {
  logger.info('Axiom metrics disabled (no AXIOM_TOKEN)');
}

// Auto-flush every 30 seconds
let flushInterval: ReturnType<typeof setInterval> | null = null;
if (axiom) {
  flushInterval = setInterval(() => {
    axiom!.flush().catch((err) => {
      logger.warn({ err }, 'Axiom flush error');
    });
  }, 30_000);
  // Don't keep process alive just for flushing
  if (flushInterval.unref) flushInterval.unref();
}

/**
 * Ingest a single event into Axiom.
 * Safe to call even if Axiom is not configured — silently no-ops.
 */
function emit(event: string, data: Record<string, unknown> = {}): void {
  if (!axiom) return;
  try {
    axiom.ingest(AXIOM_DATASET, [{ _event: event, ...data }]);
  } catch (err) {
    // Never let metrics kill the app
    logger.debug({ err, event }, 'Axiom ingest error');
  }
}

/**
 * Flush pending events. Call on shutdown.
 */
export async function flushMetrics(): Promise<void> {
  if (!axiom) return;
  if (flushInterval) clearInterval(flushInterval);
  try {
    await axiom.flush();
  } catch (err) {
    logger.warn({ err }, 'Axiom final flush error');
  }
}

// ─── Primary Metrics ────────────────────────────────────────────────

/**
 * User sent a message (inbound). Tracks:
 *   - Number of users: distinct_count(userId)
 *   - Volume: count of events
 *   - DAU: distinct_count(userId) where _time > now(-1d)
 */
export function trackMessageInbound(data: {
  userId: string;
  userName: string;
  groupFolder: string;
  chatJid: string;
  channel: string;
  isGroup: boolean;
}): void {
  emit('message.inbound', data);
}

/**
 * Bot sent a message (outbound).
 */
export function trackMessageOutbound(data: {
  chatJid: string;
  channel: string;
  textLength: number;
}): void {
  emit('message.outbound', data);
}

/**
 * Agent (container) was invoked. Tracks:
 *   - Number of agents: distinct_count(groupFolder)
 *   - Agents per user: group by triggeredBy, distinct_count(groupFolder)
 *   - Retention: recurring groupFolder activity over time
 */
export function trackAgentInvocation(data: {
  groupFolder: string;
  groupName: string;
  chatJid: string;
  isMain: boolean;
  messageCount: number;
  triggeredBy: string; // userId of the triggering user
}): void {
  emit('agent.invocation', data);
}

/**
 * Agent completed (container exited). Tracks duration, success/error.
 */
export function trackAgentComplete(data: {
  groupFolder: string;
  groupName: string;
  status: 'success' | 'error' | 'timeout';
  durationMs: number;
  exitCode: number | null;
  hadOutput: boolean;
}): void {
  emit('agent.complete', data);
}

/**
 * Transaction was logged. Tracks:
 *   - Total transactions: count
 *   - Transactions over time: count by _time
 *   - Volume by protocol: group by protocol
 */
export function trackTransaction(data: {
  signature: string;
  protocol: string;
  walletAddress: string;
  mint: string | null;
  amount: string | null;
  groupFolder: string;
}): void {
  emit('transaction.logged', {
    ...data,
    amountNumeric: data.amount ? parseFloat(data.amount) || 0 : 0,
  });
}

/**
 * Transaction synced to API successfully.
 */
export function trackTransactionSynced(data: { count: number }): void {
  emit('transaction.synced', data);
}

/**
 * Transaction sync failed.
 */
export function trackTransactionSyncError(data: {
  count: number;
  error: string;
}): void {
  emit('transaction.sync_error', data);
}

// ─── User Activity (for retention) ──────────────────────────────────

/**
 * Emitted once per user per agent invocation. Used for retention:
 *   - 7-day retention: distinct_count(userId) WHERE _time > now(-7d) AND _event = 'user.active'
 *   - 30-day retention: distinct_count(userId) WHERE _time > now(-30d)
 *   - Same agent recurring: group by userId, groupFolder over 7d/30d windows
 */
export function trackUserActive(data: {
  userId: string;
  userName: string;
  groupFolder: string;
  channel: string;
}): void {
  emit('user.active', data);
}

// ─── Group / Registration Metrics ───────────────────────────────────

export function trackGroupRegistered(data: {
  jid: string;
  name: string;
  folder: string;
  channel: string;
}): void {
  emit('group.registered', data);
}

// ─── Container Metrics ──────────────────────────────────────────────

export function trackContainerSpawn(data: {
  groupFolder: string;
  groupName: string;
  containerName: string;
  isMain: boolean;
  mountCount: number;
  isScheduledTask: boolean;
}): void {
  emit('container.spawn', data);
}

export function trackContainerTimeout(data: {
  groupFolder: string;
  groupName: string;
  containerName: string;
  durationMs: number;
  hadOutput: boolean;
}): void {
  emit('container.timeout', data);
}

// ─── Task Metrics ───────────────────────────────────────────────────

export function trackTaskCreated(data: {
  taskId: string;
  groupFolder: string;
  scheduleType: string;
  contextMode: string;
}): void {
  emit('task.created', data);
}

export function trackTaskRun(data: {
  taskId: string;
  groupFolder: string;
  status: 'success' | 'error';
  durationMs: number;
}): void {
  emit('task.run', data);
}

// ─── Channel Metrics ────────────────────────────────────────────────

export function trackChannelConnected(data: { channel: string }): void {
  emit('channel.connected', data);
}

export function trackChannelDisconnected(data: {
  channel: string;
  reason?: string;
}): void {
  emit('channel.disconnected', data);
}

// ─── Queue / Concurrency Metrics ────────────────────────────────────

export function trackQueueEvent(data: {
  event: 'enqueue' | 'at_limit' | 'retry' | 'max_retries';
  groupJid: string;
  activeCount: number;
  retryCount?: number;
}): void {
  emit('queue.event', data);
}

// ─── IPC Metrics ────────────────────────────────────────────────────

export function trackIpcMessage(data: {
  sourceGroup: string;
  targetJid: string;
  authorized: boolean;
}): void {
  emit('ipc.message', data);
}

// ─── App Lifecycle ──────────────────────────────────────────────────

export function trackAppStart(): void {
  emit('app.start', { version: process.env.npm_package_version || 'unknown' });
}

export function trackAppShutdown(data: { signal: string }): void {
  emit('app.shutdown', data);
}

// ─── Gauge Snapshots (periodic) ─────────────────────────────────────
//
// Emitted every GAUGE_INTERVAL_MS. These let you build "current state"
// dashboards in Axiom by querying the latest gauge.snapshot event.
// For DAU, Axiom can also compute distinct_count(userId) over a day
// from the raw user.active events, but the gauge provides a pre-computed
// snapshot for faster dashboard loads.
//

const GAUGE_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
let gaugeInterval: ReturnType<typeof setInterval> | null = null;
let gaugeSupplier: (() => GaugeSnapshot) | null = null;

export interface GaugeSnapshot {
  registeredGroupCount: number;
  activeContainerCount: number;
  scheduledTaskCount: number;
  totalTransactionCount: number;
  // Users who sent at least one message in the tracked window
  // These are computed from DB queries on each snapshot
  dailyActiveUsers: number;
  weeklyActiveUsers: number;
  monthlyActiveUsers: number;
  // Agents (groups) that had at least one invocation in the window
  dailyActiveAgents: number;
  weeklyActiveAgents: number;
  monthlyActiveAgents: number;
}

/**
 * Start periodic gauge emission. Call once at startup with a supplier
 * function that returns the current snapshot.
 */
export function startGaugeEmitter(supplier: () => GaugeSnapshot): void {
  if (!axiom) return;
  gaugeSupplier = supplier;

  // Emit immediately on startup
  emitGauge();

  gaugeInterval = setInterval(emitGauge, GAUGE_INTERVAL_MS);
  if (gaugeInterval.unref) gaugeInterval.unref();
}

function emitGauge(): void {
  if (!gaugeSupplier) return;
  try {
    const snapshot = gaugeSupplier();
    emit('gauge.snapshot', snapshot as unknown as Record<string, unknown>);
  } catch (err) {
    logger.debug({ err }, 'Error computing gauge snapshot');
  }
}

export function stopGaugeEmitter(): void {
  if (gaugeInterval) {
    clearInterval(gaugeInterval);
    gaugeInterval = null;
  }
}
