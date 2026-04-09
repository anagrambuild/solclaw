/**
 * IPC tool definitions and executors.
 * Replaces the MCP server (ipc-mcp-stdio.ts) with OpenAI function-calling tools.
 *
 * Provides: send_message, schedule_task, list_tasks, pause_task, resume_task,
 *           cancel_task, register_group, log_transaction
 */

import fs from 'fs';
import path from 'path';
import { CronExpressionParser } from 'cron-parser';
import { normalizeProtocol } from './known-protocols.js';
import type { ToolDefinition } from './standard-tools.js';

import { IPC_DIR, IPC_MESSAGES_DIR, IPC_TASKS_DIR, IPC_TRANSACTIONS_DIR } from './paths.js';
const MESSAGES_DIR = IPC_MESSAGES_DIR;
const TASKS_DIR = IPC_TASKS_DIR;
const TRANSACTIONS_DIR = IPC_TRANSACTIONS_DIR;

// ── Context (set once at startup) ──────────────────────────────────────────

let chatJid = '';
let groupFolder = '';
let isMain = false;

export function setIpcContext(ctx: { chatJid: string; groupFolder: string; isMain: boolean }): void {
  chatJid = ctx.chatJid;
  groupFolder = ctx.groupFolder;
  isMain = ctx.isMain;
}

// ── IPC file writer (atomic) ───────────────────────────────────────────────

function writeIpcFile(dir: string, data: object): string {
  fs.mkdirSync(dir, { recursive: true });
  const filename = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}.json`;
  const filepath = path.join(dir, filename);
  const tempPath = `${filepath}.tmp`;
  fs.writeFileSync(tempPath, JSON.stringify(data, null, 2));
  fs.renameSync(tempPath, filepath);
  return filename;
}

// ── Tool Definitions ───────────────────────────────────────────────────────

export const IPC_TOOL_DEFINITIONS: ToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'send_message',
      description: "Send a message to the user or group immediately while you're still running. Use this for progress updates or to send multiple messages. You can call this multiple times. Note: when running as a scheduled task, your final output is NOT sent to the user — use this tool if you need to communicate with the user or group.",
      parameters: {
        type: 'object',
        properties: {
          text: { type: 'string', description: 'The message text to send' },
          sender: { type: 'string', description: 'Your role/identity name (e.g. "Researcher"). When set, messages appear from a dedicated bot in Telegram.' },
        },
        required: ['text'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'schedule_task',
      description: `Schedule a recurring or one-time task. The task will run as a full agent with access to all tools.

CONTEXT MODE:
• "group": Task runs in the group's conversation context.
• "isolated": Task runs in a fresh session with no conversation history.

MESSAGING BEHAVIOR: Include guidance in the prompt about whether the agent should always send a message, only when there's something to report, or never.

SCHEDULE VALUE FORMAT (all times are LOCAL timezone):
• cron: Standard cron expression (e.g., "0 9 * * *" for daily at 9am)
• interval: Milliseconds between runs (e.g., "300000" for 5 minutes)
• once: Local time WITHOUT "Z" suffix (e.g., "2026-02-01T15:30:00")`,
      parameters: {
        type: 'object',
        properties: {
          prompt: { type: 'string', description: 'What the agent should do when the task runs.' },
          schedule_type: { type: 'string', enum: ['cron', 'interval', 'once'], description: 'cron=recurring at specific times, interval=recurring every N ms, once=run once at specific time' },
          schedule_value: { type: 'string', description: 'cron: "*/5 * * * *" | interval: milliseconds like "300000" | once: local timestamp like "2026-02-01T15:30:00"' },
          context_mode: { type: 'string', enum: ['group', 'isolated'], description: 'group=runs with chat history, isolated=fresh session (default: group)' },
          target_group_jid: { type: 'string', description: '(Main group only) JID of the group to schedule the task for. Defaults to the current group.' },
        },
        required: ['prompt', 'schedule_type', 'schedule_value'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'list_tasks',
      description: "List all scheduled tasks. From main: shows all tasks. From other groups: shows only that group's tasks.",
      parameters: { type: 'object', properties: {}, required: [] },
    },
  },
  {
    type: 'function',
    function: {
      name: 'pause_task',
      description: 'Pause a scheduled task. It will not run until resumed.',
      parameters: {
        type: 'object',
        properties: {
          task_id: { type: 'string', description: 'The task ID to pause' },
        },
        required: ['task_id'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'resume_task',
      description: 'Resume a paused task.',
      parameters: {
        type: 'object',
        properties: {
          task_id: { type: 'string', description: 'The task ID to resume' },
        },
        required: ['task_id'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'cancel_task',
      description: 'Cancel and delete a scheduled task.',
      parameters: {
        type: 'object',
        properties: {
          task_id: { type: 'string', description: 'The task ID to cancel' },
        },
        required: ['task_id'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'register_group',
      description: 'Register a new WhatsApp group so the agent can respond to messages there. Main group only. Use available_groups.json to find the JID for a group.',
      parameters: {
        type: 'object',
        properties: {
          jid: { type: 'string', description: 'The WhatsApp JID (e.g., "120363336345536173@g.us")' },
          name: { type: 'string', description: 'Display name for the group' },
          folder: { type: 'string', description: 'Folder name for group files (lowercase, hyphens)' },
          trigger: { type: 'string', description: 'Trigger word (e.g., "@Andy")' },
        },
        required: ['jid', 'name', 'folder', 'trigger'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'log_transaction',
      description: `Log a Solana transaction after it has been successfully confirmed on-chain. Call this AFTER every successful transaction.

mint and amount: provide BOTH or NEITHER.
- Swap/transfer/stake: provide both mint and amount (e.g. mint="So11...1112", amount="1.5")
- For SOL, use wSOL mint: So11111111111111111111111111111111111111112
- Account creation or other txns with no token movement: omit both`,
      parameters: {
        type: 'object',
        properties: {
          signature: { type: 'string', description: 'The transaction signature (base58, ~88 chars)' },
          protocol: { type: 'string', description: 'Protocol name (e.g., "jupiter", "drift", "raydium"). Use base name only.' },
          wallet_address: { type: 'string', description: 'Wallet public key that signed the transaction' },
          mint: { type: 'string', description: 'Token mint address. Required with amount. For SOL use wSOL: So11111111111111111111111111111111111111112' },
          amount: { type: 'string', description: 'Human-readable amount in token units (e.g., "1.5"). Required with mint.' },
        },
        required: ['signature', 'protocol', 'wallet_address'],
      },
    },
  },
];

// ── Tool Executors ─────────────────────────────────────────────────────────

function executeSendMessage(args: { text: string; sender?: string }): string {
  writeIpcFile(MESSAGES_DIR, {
    type: 'message',
    chatJid,
    text: args.text,
    sender: args.sender || undefined,
    groupFolder,
    timestamp: new Date().toISOString(),
  });
  return 'Message sent.';
}

function executeScheduleTask(args: {
  prompt: string;
  schedule_type: 'cron' | 'interval' | 'once';
  schedule_value: string;
  context_mode?: string;
  target_group_jid?: string;
}): string {
  // Validate schedule_value
  if (args.schedule_type === 'cron') {
    try { CronExpressionParser.parse(args.schedule_value); }
    catch { return `Invalid cron: "${args.schedule_value}". Use format like "0 9 * * *" (daily 9am).`; }
  } else if (args.schedule_type === 'interval') {
    const ms = parseInt(args.schedule_value, 10);
    if (isNaN(ms) || ms <= 0) return `Invalid interval: "${args.schedule_value}". Must be positive milliseconds.`;
  } else if (args.schedule_type === 'once') {
    if (/[Zz]$/.test(args.schedule_value) || /[+-]\d{2}:\d{2}$/.test(args.schedule_value)) {
      return `Timestamp must be local time without timezone suffix. Got "${args.schedule_value}" — use format like "2026-02-01T15:30:00".`;
    }
    if (isNaN(new Date(args.schedule_value).getTime())) {
      return `Invalid timestamp: "${args.schedule_value}". Use local time format like "2026-02-01T15:30:00".`;
    }
  }

  const targetJid = isMain && args.target_group_jid ? args.target_group_jid : chatJid;

  const filename = writeIpcFile(TASKS_DIR, {
    type: 'schedule_task',
    prompt: args.prompt,
    schedule_type: args.schedule_type,
    schedule_value: args.schedule_value,
    context_mode: args.context_mode || 'group',
    targetJid,
    createdBy: groupFolder,
    timestamp: new Date().toISOString(),
  });

  return `Task scheduled (${filename}): ${args.schedule_type} - ${args.schedule_value}`;
}

function executeListTasks(): string {
  const tasksFile = path.join(IPC_DIR, 'current_tasks.json');
  try {
    if (!fs.existsSync(tasksFile)) return 'No scheduled tasks found.';
    const allTasks = JSON.parse(fs.readFileSync(tasksFile, 'utf-8'));
    const tasks = isMain ? allTasks : allTasks.filter((t: { groupFolder: string }) => t.groupFolder === groupFolder);
    if (tasks.length === 0) return 'No scheduled tasks found.';

    return 'Scheduled tasks:\n' + tasks.map(
      (t: { id: string; prompt: string; schedule_type: string; schedule_value: string; status: string; next_run: string }) =>
        `- [${t.id}] ${t.prompt.slice(0, 50)}... (${t.schedule_type}: ${t.schedule_value}) - ${t.status}, next: ${t.next_run || 'N/A'}`
    ).join('\n');
  } catch (err) {
    return `Error reading tasks: ${err instanceof Error ? err.message : String(err)}`;
  }
}

function executePauseTask(args: { task_id: string }): string {
  writeIpcFile(TASKS_DIR, { type: 'pause_task', taskId: args.task_id, groupFolder, isMain, timestamp: new Date().toISOString() });
  return `Task ${args.task_id} pause requested.`;
}

function executeResumeTask(args: { task_id: string }): string {
  writeIpcFile(TASKS_DIR, { type: 'resume_task', taskId: args.task_id, groupFolder, isMain, timestamp: new Date().toISOString() });
  return `Task ${args.task_id} resume requested.`;
}

function executeCancelTask(args: { task_id: string }): string {
  writeIpcFile(TASKS_DIR, { type: 'cancel_task', taskId: args.task_id, groupFolder, isMain, timestamp: new Date().toISOString() });
  return `Task ${args.task_id} cancellation requested.`;
}

function executeRegisterGroup(args: { jid: string; name: string; folder: string; trigger: string }): string {
  if (!isMain) return 'Only the main group can register new groups.';
  writeIpcFile(TASKS_DIR, {
    type: 'register_group',
    jid: args.jid,
    name: args.name,
    folder: args.folder,
    trigger: args.trigger,
    timestamp: new Date().toISOString(),
  });
  return `Group "${args.name}" registered. It will start receiving messages immediately.`;
}

function executeLogTransaction(args: {
  signature: string;
  protocol: string;
  wallet_address: string;
  mint?: string;
  amount?: string;
}): string {
  if (args.signature.length < 80 || args.signature.length > 100) {
    return `Invalid signature length (${args.signature.length}). Expected ~88 chars.`;
  }
  if ((args.mint && !args.amount) || (!args.mint && args.amount)) {
    return 'mint and amount must be provided together or both omitted.';
  }

  writeIpcFile(TRANSACTIONS_DIR, {
    type: 'log_transaction',
    signature: args.signature,
    protocol: normalizeProtocol(args.protocol),
    wallet_address: args.wallet_address,
    mint: args.mint || null,
    amount: args.amount || null,
    timestamp: new Date().toISOString(),
  });

  return `Transaction logged: ${args.signature.slice(0, 16)}...`;
}

// ── Dispatcher ─────────────────────────────────────────────────────────────

export function executeIpcTool(name: string, args: Record<string, unknown>): string | null {
  switch (name) {
    case 'send_message': return executeSendMessage(args as { text: string; sender?: string });
    case 'schedule_task': return executeScheduleTask(args as Parameters<typeof executeScheduleTask>[0]);
    case 'list_tasks': return executeListTasks();
    case 'pause_task': return executePauseTask(args as { task_id: string });
    case 'resume_task': return executeResumeTask(args as { task_id: string });
    case 'cancel_task': return executeCancelTask(args as { task_id: string });
    case 'register_group': return executeRegisterGroup(args as { jid: string; name: string; folder: string; trigger: string });
    case 'log_transaction': return executeLogTransaction(args as Parameters<typeof executeLogTransaction>[0]);
    default: return null; // Not an IPC tool
  }
}
