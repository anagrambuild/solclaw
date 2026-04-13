/**
 * Canonical container paths — single source of truth.
 *
 * /data  is the persistent volume mount inside both the standalone
 * solclaw container and the dashboard's Fly container.
 */
import path from 'path';

export const IPC_DIR = '/data/ipc';
export const IPC_MESSAGES_DIR = path.join(IPC_DIR, 'messages');
export const IPC_TASKS_DIR = path.join(IPC_DIR, 'tasks');
export const IPC_INPUT_DIR = path.join(IPC_DIR, 'input');
export const IPC_TRANSACTIONS_DIR = path.join(IPC_DIR, 'transactions');
export const IPC_INPUT_CLOSE_SENTINEL = path.join(IPC_INPUT_DIR, '_close');
export const CONVERSATION_DIR = path.join(IPC_DIR, 'conversation');
export const CONVERSATION_HISTORY_FILE = path.join(CONVERSATION_DIR, 'history.json');
