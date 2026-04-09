/**
 * Conversation persistence — save/load/trim/archive message history.
 *
 * Messages stored as JSON at /data/ipc/conversation/history.json.
 * Replaces Claude Agent SDK's session resumption with explicit persistence.
 */

import fs from 'fs';
import path from 'path';

const CONVERSATION_DIR = '/data/ipc/conversation';
const HISTORY_FILE = path.join(CONVERSATION_DIR, 'history.json');

const CHARS_PER_TOKEN = 4;

const MODEL_CONTEXT_LIMITS: Record<string, number> = {
  // Anthropic
  'anthropic/claude-opus-4-6': 200_000,
  'anthropic/claude-sonnet-4-5': 200_000,
  'anthropic/claude-sonnet-4': 200_000,
  'anthropic/claude-haiku-3.5': 200_000,
  // OpenAI
  'openai/gpt-5.4': 1_050_000,
  'openai/gpt-5.4-pro': 1_050_000,
  'openai/gpt-5.4-mini': 400_000,
  'openai/gpt-5.4-nano': 400_000,
  'openai/o4-mini': 200_000,
  // Google
  'google/gemini-2.5-pro-preview': 1_000_000,
  'google/gemini-2.5-flash-preview': 1_000_000,
  'google/gemini-2.0-flash': 1_000_000,
  // DeepSeek
  'deepseek/deepseek-chat-v3-0324': 64_000,
  // Meta
  'meta-llama/llama-4-maverick': 1_000_000,
  // Mistral
  'mistralai/mistral-large-latest': 128_000,
};

const DEFAULT_CONTEXT_LIMIT = 128_000;
const CONTEXT_USAGE_RATIO = 0.7;

export interface ConversationMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | null;
  toolCalls?: Array<{
    id: string;
    type: 'function';
    function: { name: string; arguments: string };
  }>;
  toolCallId?: string;
  name?: string;
}

function getContextLimit(modelId: string): number {
  return MODEL_CONTEXT_LIMITS[modelId] ?? DEFAULT_CONTEXT_LIMIT;
}

function estimateTokens(messages: ConversationMessage[]): number {
  let chars = 0;
  for (const msg of messages) {
    if (typeof msg.content === 'string') chars += msg.content.length;
    if (msg.toolCalls) {
      for (const tc of msg.toolCalls) {
        chars += tc.function.name.length + tc.function.arguments.length;
      }
    }
  }
  return Math.ceil(chars / CHARS_PER_TOKEN);
}

export function loadConversation(): ConversationMessage[] {
  try {
    if (!fs.existsSync(HISTORY_FILE)) return [];
    return JSON.parse(fs.readFileSync(HISTORY_FILE, 'utf-8'));
  } catch { return []; }
}

export function saveConversation(messages: ConversationMessage[]): void {
  fs.mkdirSync(CONVERSATION_DIR, { recursive: true });
  fs.writeFileSync(HISTORY_FILE, JSON.stringify(messages));
}

export function clearConversation(): void {
  try { fs.unlinkSync(HISTORY_FILE); } catch { /* not found */ }
}

/**
 * Archive the current conversation to a markdown file.
 * Replaces the PreCompact hook from Claude Agent SDK.
 */
export function archiveConversation(messages: ConversationMessage[], assistantName?: string): void {
  const conversationsDir = '/workspace/group/conversations';
  fs.mkdirSync(conversationsDir, { recursive: true });

  const userMessages = messages.filter(m => m.role === 'user' || m.role === 'assistant');
  if (userMessages.length === 0) return;

  const date = new Date().toISOString().split('T')[0];
  const time = new Date();
  const fallbackName = `conversation-${time.getHours().toString().padStart(2, '0')}${time.getMinutes().toString().padStart(2, '0')}`;

  // Use first user message as title (sanitized)
  const firstMsg = userMessages.find(m => m.role === 'user');
  const title = firstMsg?.content
    ? firstMsg.content.slice(0, 50).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')
    : fallbackName;

  const filename = `${date}-${title || fallbackName}.md`;
  const filePath = path.join(conversationsDir, filename);

  const lines: string[] = ['# Conversation', '', `Archived: ${time.toLocaleString('en-US', { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit', hour12: true })}`, '', '---', ''];

  for (const msg of userMessages) {
    if (typeof msg.content !== 'string') continue;
    const sender = msg.role === 'user' ? 'User' : (assistantName || 'Assistant');
    const content = msg.content.length > 2000 ? msg.content.slice(0, 2000) + '...' : msg.content;
    lines.push(`**${sender}**: ${content}`, '');
  }

  fs.writeFileSync(filePath, lines.join('\n'));
}

/**
 * Trim conversation to fit within the model's context window.
 * Archives before dropping old messages.
 */
export function trimConversation(
  messages: ConversationMessage[],
  modelId: string,
  assistantName?: string,
): ConversationMessage[] {
  const maxTokens = Math.floor(getContextLimit(modelId) * CONTEXT_USAGE_RATIO);
  if (estimateTokens(messages) <= maxTokens) return messages;

  // Archive before trimming
  archiveConversation(messages, assistantName);

  const head = messages.slice(0, 2);
  const rest = messages.slice(2);
  const headTokens = estimateTokens(head);
  const remainingBudget = maxTokens - headTokens;

  const kept: ConversationMessage[] = [];
  let usedTokens = 0;
  for (let i = rest.length - 1; i >= 0; i--) {
    const msgTokens = estimateTokens([rest[i]]);
    if (usedTokens + msgTokens > remainingBudget) break;
    kept.unshift(rest[i]);
    usedTokens += msgTokens;
  }

  return [...head, ...kept];
}
