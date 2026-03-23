/**
 * SolClaw Agent Runner
 * Runs inside a container, receives config via stdin, outputs result to stdout.
 *
 * Multi-provider support:
 *   - "openrouter" → OpenRouter SDK (any model)
 *   - "anthropic"  → Anthropic SDK direct (sk-ant-* keys)
 *   - "openai"     → OpenRouter SDK pointed at api.openai.com (sk-* keys)
 *
 * Input protocol:
 *   Stdin: Full ContainerInput JSON (read until EOF)
 *   IPC:   Follow-up messages written as JSON files to /workspace/ipc/input/
 *          Sentinel: /workspace/ipc/input/_close — signals session end
 *
 * Stdout protocol:
 *   Each result is wrapped in OUTPUT_START_MARKER / OUTPUT_END_MARKER pairs.
 */

import fs from 'fs';
import path from 'path';
import { OpenRouter } from '@openrouter/sdk';
import Anthropic from '@anthropic-ai/sdk';
import { normalizeProtocol } from './known-protocols.js';
import { STANDARD_TOOL_DEFINITIONS, executeStandardTool, setCwd, type ToolDefinition } from './standard-tools.js';
import { IPC_TOOL_DEFINITIONS, executeIpcTool, setIpcContext } from './ipc-tools.js';
import {
  type ConversationMessage,
  loadConversation,
  saveConversation,
  clearConversation,
  trimConversation,
} from './conversation.js';

// ── Types ──────────────────────────────────────────────────────────────────

interface ContainerInput {
  prompt: string;
  sessionId?: string;
  groupFolder: string;
  chatJid: string;
  isMain: boolean;
  isScheduledTask?: boolean;
  assistantName?: string;
  secrets?: Record<string, string>;
}

interface ContainerOutput {
  status: 'success' | 'error';
  result: string | null;
  newSessionId?: string;
  error?: string;
}

// ── Constants ──────────────────────────────────────────────────────────────

const IPC_INPUT_DIR = '/workspace/ipc/input';
const IPC_INPUT_CLOSE_SENTINEL = path.join(IPC_INPUT_DIR, '_close');
const IPC_TRANSACTIONS_DIR = '/workspace/ipc/transactions';
const IPC_POLL_MS = 500;
const SYNC_API_URL = 'https://api.breeze.baby/agent/stats-sync-up';
const MAX_ROUNDS = 50;

const ALL_TOOL_DEFINITIONS: ToolDefinition[] = [...STANDARD_TOOL_DEFINITIONS, ...IPC_TOOL_DEFINITIONS];

type KeyType = 'openrouter' | 'anthropic' | 'openai' | 'google';

/** Normalized result from any provider. */
interface TurnCallResult {
  text: string | null;
  toolCalls: Array<{ id: string; name: string; arguments: string }>;
}

function stripProviderPrefix(modelId: string): string {
  const slash = modelId.indexOf('/');
  return slash >= 0 ? modelId.slice(slash + 1) : modelId;
}

/** Convert tool definitions to Anthropic format. */
function toAnthropicTools(defs: ToolDefinition[]): Anthropic.Tool[] {
  return defs.map((td) => ({
    name: td.function.name,
    description: td.function.description,
    input_schema: td.function.parameters as Anthropic.Tool['input_schema'],
  }));
}

/** Convert conversation messages to Anthropic format. */
function toAnthropicMessages(messages: ConversationMessage[]): Anthropic.MessageParam[] {
  const result: Anthropic.MessageParam[] = [];
  for (const msg of messages) {
    if (msg.role === 'system') continue;
    if (msg.role === 'user') {
      result.push({ role: 'user', content: msg.content ?? '' });
      continue;
    }
    if (msg.role === 'assistant') {
      const content: Anthropic.ContentBlockParam[] = [];
      if (msg.content) content.push({ type: 'text', text: msg.content });
      if (msg.toolCalls) {
        for (const tc of msg.toolCalls) {
          let input: Record<string, unknown>;
          try { input = JSON.parse(tc.function.arguments); } catch { input = {}; }
          content.push({ type: 'tool_use', id: tc.id, name: tc.function.name, input });
        }
      }
      if (content.length > 0) result.push({ role: 'assistant', content });
      continue;
    }
    if (msg.role === 'tool' && msg.toolCallId) {
      result.push({
        role: 'user',
        content: [{ type: 'tool_result', tool_use_id: msg.toolCallId, content: msg.content ?? '' }],
      });
    }
  }
  return result;
}

/** Call Anthropic Messages API directly. */
async function callAnthropic(
  client: Anthropic, modelId: string, messages: ConversationMessage[], tools: ToolDefinition[],
): Promise<TurnCallResult> {
  const systemMsg = messages.find((m) => m.role === 'system');
  const systemPrompt = typeof systemMsg?.content === 'string' ? systemMsg.content : '';
  const response = await client.messages.create({
    model: stripProviderPrefix(modelId),
    max_tokens: 16384,
    system: systemPrompt,
    messages: toAnthropicMessages(messages),
    tools: toAnthropicTools(tools),
  });
  let text: string | null = null;
  const toolCalls: TurnCallResult['toolCalls'] = [];
  for (const block of response.content) {
    if (block.type === 'text') text = (text ?? '') + block.text;
    else if (block.type === 'tool_use') {
      toolCalls.push({ id: block.id, name: block.name, arguments: JSON.stringify(block.input) });
    }
  }
  return { text, toolCalls };
}

/** Call OpenRouter SDK. */
async function callOpenRouterAPI(
  client: OpenRouter, modelId: string, messages: ConversationMessage[], tools: ToolDefinition[],
): Promise<TurnCallResult> {
  const response = await client.chat.send({
    httpReferer: 'https://solclaw.ai',
    xTitle: 'SolClaw Agent',
    chatGenerationParams: {
      model: modelId,
      messages: messages as Parameters<typeof client.chat.send>[0]['chatGenerationParams']['messages'],
      tools: tools as Parameters<typeof client.chat.send>[0]['chatGenerationParams']['tools'],
      maxTokens: 16384,
      stream: false,
    },
  });
  const choice = response.choices?.[0];
  if (!choice?.message) return { text: null, toolCalls: [] };
  const msg = choice.message;
  return {
    text: typeof msg.content === 'string' ? msg.content : null,
    toolCalls: (msg.toolCalls ?? []).map((tc) => ({ id: tc.id, name: tc.function.name, arguments: tc.function.arguments })),
  };
}

/** Call OpenAI API directly via fetch (OpenRouter SDK has validation that rejects OpenAI responses). */
async function callOpenAIDirect(
  apiKey: string, modelId: string, messages: ConversationMessage[], tools: ToolDefinition[],
): Promise<TurnCallResult> {
  const openaiMessages = messages.map((m) => {
    if (m.role === 'tool') {
      return { role: 'tool' as const, content: m.content ?? '', tool_call_id: m.toolCallId };
    }
    if (m.role === 'assistant' && m.toolCalls) {
      return {
        role: 'assistant' as const,
        content: m.content ?? null,
        tool_calls: m.toolCalls.map((tc) => ({
          id: tc.id, type: 'function' as const,
          function: { name: tc.function.name, arguments: tc.function.arguments },
        })),
      };
    }
    return { role: m.role, content: m.content ?? '' };
  });

  const res = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiKey}` },
    body: JSON.stringify({
      model: stripProviderPrefix(modelId),
      messages: openaiMessages,
      tools,
      max_tokens: 16384,
    }),
  });

  if (!res.ok) {
    const errText = await res.text();
    throw new Error(`OpenAI API error ${res.status}: ${errText.slice(0, 500)}`);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const data: any = await res.json();
  const choice = data.choices?.[0];
  if (!choice?.message) return { text: null, toolCalls: [] };
  const msg = choice.message;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return {
    text: typeof msg.content === 'string' ? msg.content : null,
    toolCalls: (msg.tool_calls ?? []).map((tc: any) => ({ id: tc.id, name: tc.function.name, arguments: tc.function.arguments })),
  };
}

const OUTPUT_START_MARKER = '---NANOCLAW_OUTPUT_START---';
const OUTPUT_END_MARKER = '---NANOCLAW_OUTPUT_END---';

// ── Helpers ────────────────────────────────────────────────────────────────

function writeOutput(output: ContainerOutput): void {
  console.log(OUTPUT_START_MARKER);
  console.log(JSON.stringify(output));
  console.log(OUTPUT_END_MARKER);
}

function log(message: string): void {
  console.error(`[agent-runner] ${message}`);
}

async function readStdin(): Promise<string> {
  return new Promise((resolve, reject) => {
    let data = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', chunk => { data += chunk; });
    process.stdin.on('end', () => resolve(data));
    process.stdin.on('error', reject);
  });
}

// ── IPC Input ──────────────────────────────────────────────────────────────

function shouldClose(): boolean {
  if (fs.existsSync(IPC_INPUT_CLOSE_SENTINEL)) {
    try { fs.unlinkSync(IPC_INPUT_CLOSE_SENTINEL); } catch { /* ignore */ }
    return true;
  }
  return false;
}

function drainIpcInput(): string[] {
  try {
    fs.mkdirSync(IPC_INPUT_DIR, { recursive: true });
    const files = fs.readdirSync(IPC_INPUT_DIR).filter(f => f.endsWith('.json')).sort();
    const messages: string[] = [];
    for (const file of files) {
      const filePath = path.join(IPC_INPUT_DIR, file);
      try {
        const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
        fs.unlinkSync(filePath);
        if (data.type === 'message' && data.text) messages.push(data.text);
      } catch (err) {
        log(`Failed to process input file ${file}: ${err instanceof Error ? err.message : String(err)}`);
        try { fs.unlinkSync(filePath); } catch { /* ignore */ }
      }
    }
    return messages;
  } catch (err) {
    log(`IPC drain error: ${err instanceof Error ? err.message : String(err)}`);
    return [];
  }
}

function waitForIpcMessage(): Promise<string | null> {
  return new Promise((resolve) => {
    const poll = () => {
      if (shouldClose()) { resolve(null); return; }
      const messages = drainIpcInput();
      if (messages.length > 0) { resolve(messages.join('\n')); return; }
      setTimeout(poll, IPC_POLL_MS);
    };
    poll();
  });
}

// ── System Prompt Builder ──────────────────────────────────────────────────

function buildSystemPrompt(containerInput: ContainerInput): string {
  const parts: string[] = [];

  // Base agent identity
  parts.push(`You are a SolClaw AI agent running in a container. You have access to Bash, filesystem tools, web tools, and IPC tools for communicating with the host system.`);
  parts.push('');

  // CLAUDE.md files from group working directory
  const groupClaudeMd = '/workspace/group/CLAUDE.md';
  if (fs.existsSync(groupClaudeMd)) {
    parts.push('## Project Context (from group CLAUDE.md)');
    parts.push(fs.readFileSync(groupClaudeMd, 'utf-8'));
    parts.push('');
  }

  // Additional directories' CLAUDE.md files
  const extraBase = '/workspace/extra';
  if (fs.existsSync(extraBase)) {
    for (const entry of fs.readdirSync(extraBase)) {
      const mdPath = path.join(extraBase, entry, 'CLAUDE.md');
      if (fs.existsSync(mdPath)) {
        parts.push(`## Additional Context (from ${entry}/CLAUDE.md)`);
        parts.push(fs.readFileSync(mdPath, 'utf-8'));
        parts.push('');
      }
    }
  }

  // Global CLAUDE.md (for non-main groups)
  if (!containerInput.isMain) {
    const globalClaudeMd = '/workspace/global/CLAUDE.md';
    if (fs.existsSync(globalClaudeMd)) {
      parts.push('## Global Context');
      parts.push(fs.readFileSync(globalClaudeMd, 'utf-8'));
      parts.push('');
    }
  }

  // Transaction logging instruction
  parts.push(`## MANDATORY: Transaction Logging

After EVERY successful on-chain Solana transaction, you MUST log it using one of these methods:

1. **Tool (preferred):** Call the \`log_transaction\` tool with signature, protocol, and wallet_address.
2. **Code-level:** Import and call \`logTransactionIpc\` from \`/tmp/dist/log-transaction.js\`:
   \`\`\`typescript
   import { logTransactionIpc } from '/tmp/dist/log-transaction.js';
   logTransactionIpc(signature, 'drift', walletPublicKey);
   \`\`\`

**Valid protocol names:** breeze, coingecko, crossmint, dflow, drift, glam, helius, jupiter, kamino, manifest, marginfi, metaplex, meteora, orca, pumpfun, raydium, swig, system-program, token-program.

For SOL transactions, use wSOL mint: So11111111111111111111111111111111111111112
This applies to ALL transaction types: swaps, transfers, stakes, account creation, lending, borrowing, NFT mints, etc.`);

  return parts.join('\n');
}

// ── Tool Execution ─────────────────────────────────────────────────────────

async function executeTool(name: string, args: Record<string, unknown>): Promise<string> {
  // Try standard tools first, then IPC tools
  const standardResult = await executeStandardTool(name, args);
  if (standardResult !== null) return standardResult;

  const ipcResult = executeIpcTool(name, args);
  if (ipcResult !== null) return ipcResult;

  return `Unknown tool: ${name}`;
}

// ── Agentic Loop ───────────────────────────────────────────────────────────

async function runAgenticTurn(
  callLLMFn: (messages: ConversationMessage[], tools: ToolDefinition[]) => Promise<TurnCallResult>,
  modelId: string,
  userMessage: string,
  systemPrompt: string,
  assistantName: string | undefined,
  isNewSession: boolean,
): Promise<{ textResponse: string | null; closedDuringQuery: boolean }> {
  let messages: ConversationMessage[];
  if (isNewSession) {
    clearConversation();
    messages = [{ role: 'system', content: systemPrompt }];
  } else {
    messages = loadConversation();
    if (messages.length === 0) {
      messages = [{ role: 'system', content: systemPrompt }];
    }
  }

  messages.push({ role: 'user', content: userMessage });
  messages = trimConversation(messages, modelId, assistantName);
  saveConversation(messages);

  let closedDuringQuery = false;
  let lastTextResponse: string | null = null;

  for (let round = 0; round < MAX_ROUNDS; round++) {
    // Check for close sentinel between rounds
    if (round > 0 && shouldClose()) {
      log('Close sentinel detected between rounds');
      closedDuringQuery = true;
      break;
    }

    // Drain any IPC messages that arrived during tool execution
    if (round > 0) {
      const newMessages = drainIpcInput();
      for (const text of newMessages) {
        log(`Piping IPC message into conversation (${text.length} chars)`);
        messages.push({ role: 'user', content: text });
      }
    }

    const result = await callLLMFn(messages, ALL_TOOL_DEFINITIONS);

    // Store assistant message
    const storedMessage: ConversationMessage = {
      role: 'assistant',
      content: result.text,
    };

    if (result.toolCalls.length > 0) {
      storedMessage.toolCalls = result.toolCalls.map((tc) => ({
        id: tc.id,
        type: 'function' as const,
        function: { name: tc.name, arguments: tc.arguments },
      }));
    }

    messages.push(storedMessage);

    if (result.text) {
      lastTextResponse = result.text;
    }

    if (result.toolCalls.length === 0) {
      saveConversation(messages);
      return { textResponse: lastTextResponse, closedDuringQuery };
    }

    // Execute tool calls
    for (const toolCall of result.toolCalls) {
      let toolArgs: Record<string, unknown>;
      try { toolArgs = JSON.parse(toolCall.arguments); }
      catch { toolArgs = {}; }

      log(`Tool: ${toolCall.name} (${JSON.stringify(toolArgs).slice(0, 200)})`);

      let toolResult: string;
      try { toolResult = await executeTool(toolCall.name, toolArgs); }
      catch (err) { toolResult = `Tool error: ${err instanceof Error ? err.message : String(err)}`; }

      messages.push({ role: 'tool', content: toolResult, toolCallId: toolCall.id });
    }

    saveConversation(messages);
  }

  saveConversation(messages);
  return { textResponse: lastTextResponse, closedDuringQuery };
}

// ── Transaction Drain ──────────────────────────────────────────────────────

async function drainIpcTransactions(): Promise<void> {
  let files: string[];
  try { files = fs.readdirSync(IPC_TRANSACTIONS_DIR).filter(f => f.endsWith('.json')); }
  catch { return; }
  if (files.length === 0) return;

  log(`Draining ${files.length} leftover IPC transaction file(s)...`);
  const entries: Record<string, unknown>[] = [];

  for (const file of files) {
    const filePath = path.join(IPC_TRANSACTIONS_DIR, file);
    try {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
      if (data.signature) {
        const entry: Record<string, unknown> = {
          signature: data.signature,
          protocol: data.protocol ? normalizeProtocol(data.protocol) : null,
          wallet_address: data.wallet_address || data.wallet || null,
        };
        if (data.mint) entry.mint = data.mint;
        if (data.amount) entry.amount = parseFloat(data.amount);
        entries.push(entry);
      }
      fs.unlinkSync(filePath);
    } catch { try { fs.unlinkSync(filePath); } catch { /* ignore */ } }
  }

  if (entries.length === 0) return;

  try {
    await fetch(SYNC_API_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ transaction_entries: entries }),
      signal: AbortSignal.timeout(15_000),
    });
    log(`Synced ${entries.length} leftover transaction(s) to API`);
  } catch (err) {
    log(`Failed to sync leftover transactions: ${err instanceof Error ? err.message : String(err)}`);
  }
}

// ── Main ───────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  let containerInput: ContainerInput;

  try {
    const stdinData = await readStdin();
    containerInput = JSON.parse(stdinData);
    try { fs.unlinkSync('/tmp/input.json'); } catch { /* may not exist */ }
    log(`Received input for group: ${containerInput.groupFolder}`);
  } catch (err) {
    writeOutput({
      status: 'error',
      result: null,
      error: `Failed to parse input: ${err instanceof Error ? err.message : String(err)}`,
    });
    process.exit(1);
  }

  // Extract API key, model, and key type from secrets/env
  const apiKey = containerInput.secrets?.OPENROUTER_API_KEY
    || process.env.OPENROUTER_API_KEY
    || '';

  const modelId = containerInput.secrets?.MODEL_ID
    || process.env.MODEL_ID
    || 'anthropic/claude-opus-4-6';

  const keyType = (containerInput.secrets?.KEY_TYPE
    || process.env.KEY_TYPE
    || 'openrouter') as KeyType;

  if (!apiKey) {
    writeOutput({ status: 'error', result: null, error: 'No API key provided (OPENROUTER_API_KEY)' });
    process.exit(1);
  }

  // Inject secrets into process.env so tool scripts can access API keys
  // (only protocol-specific keys, NOT the LLM API key)
  for (const [key, value] of Object.entries(containerInput.secrets || {})) {
    if (key !== 'OPENROUTER_API_KEY' && key !== 'CLAUDE_CODE_OAUTH_TOKEN') {
      process.env[key] = value;
    }
  }

  log(`Model: ${modelId}`);
  log(`Key type: ${keyType}`);
  log(`API key: ${apiKey ? 'set' : 'MISSING'}`);

  // Build provider-specific LLM call function
  let callLLM: (messages: ConversationMessage[], tools: ToolDefinition[]) => Promise<TurnCallResult>;

  if (keyType === 'anthropic') {
    const anthropicClient = new Anthropic({ apiKey });
    callLLM = (msgs, tools) => callAnthropic(anthropicClient, modelId, msgs, tools);
  } else if (keyType === 'openai') {
    callLLM = (msgs, tools) => callOpenAIDirect(apiKey, modelId, msgs, tools);
  } else {
    // "openrouter", "google", and any unknown key types
    const orClient = new OpenRouter({ apiKey });
    callLLM = (msgs, tools) => callOpenRouterAPI(orClient, modelId, msgs, tools);
  }

  // Set IPC context for tools
  setIpcContext({
    chatJid: containerInput.chatJid,
    groupFolder: containerInput.groupFolder,
    isMain: containerInput.isMain,
  });

  // Set working directory
  setCwd('/workspace/group');

  fs.mkdirSync(IPC_INPUT_DIR, { recursive: true });
  try { fs.unlinkSync(IPC_INPUT_CLOSE_SENTINEL); } catch { /* ignore */ }

  // Build initial prompt
  let prompt = containerInput.prompt;
  if (containerInput.isScheduledTask) {
    prompt = `[SCHEDULED TASK - The following message was sent automatically and is not coming directly from the user or group.]\n\n${prompt}`;
  }
  const pending = drainIpcInput();
  if (pending.length > 0) {
    log(`Draining ${pending.length} pending IPC messages into initial prompt`);
    prompt += '\n' + pending.join('\n');
  }

  // Build system prompt
  const systemPrompt = buildSystemPrompt(containerInput);

  // Periodically drain IPC transaction files
  const txDrainInterval = setInterval(() => { drainIpcTransactions().catch(() => {}); }, 60_000);

  // Use a conversation ID as the session identifier (stable across turns)
  const conversationId = `${containerInput.groupFolder}-${Date.now()}`;

  // Query loop
  let isNewSession = true;
  try {
    while (true) {
      log(`Starting turn (model: ${modelId}, new: ${isNewSession})...`);

      const result = await runAgenticTurn(
        callLLM, modelId, prompt, systemPrompt, containerInput.assistantName, isNewSession,
      );

      isNewSession = false;

      writeOutput({
        status: 'success',
        result: result.textResponse,
        newSessionId: conversationId,
      });

      if (result.closedDuringQuery) {
        log('Close sentinel consumed during query, exiting');
        break;
      }

      log('Turn ended, waiting for next IPC message...');
      const nextMessage = await waitForIpcMessage();
      if (nextMessage === null) {
        log('Close sentinel received, exiting');
        break;
      }

      log(`Got new message (${nextMessage.length} chars), starting new turn`);
      prompt = nextMessage;
    }
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    log(`Agent error: ${errorMessage}`);
    writeOutput({
      status: 'error',
      result: null,
      newSessionId: conversationId,
      error: errorMessage,
    });
    clearInterval(txDrainInterval);
    await drainIpcTransactions();
    process.exit(1);
  }

  clearInterval(txDrainInterval);
  await drainIpcTransactions();
}

main();
