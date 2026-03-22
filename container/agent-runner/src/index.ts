/**
 * SolClaw Agent Runner
 * Runs inside a container, receives config via stdin, outputs result to stdout.
 *
 * Uses the OpenRouter SDK for multi-model support (Claude, GPT-4o, Gemini, etc.)
 * with function-calling tools replacing the Claude Agent SDK + MCP approach.
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
import { normalizeProtocol } from './known-protocols.js';
import { STANDARD_TOOL_DEFINITIONS, executeStandardTool, setCwd } from './standard-tools.js';
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

const ALL_TOOL_DEFINITIONS = [...STANDARD_TOOL_DEFINITIONS, ...IPC_TOOL_DEFINITIONS];

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
  client: OpenRouter,
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

    const response = await client.chat.send({
      httpReferer: 'https://solclaw.ai',
      xTitle: 'SolClaw Agent',
      chatGenerationParams: {
        model: modelId,
        messages: messages as Parameters<typeof client.chat.send>[0]['chatGenerationParams']['messages'],
        tools: ALL_TOOL_DEFINITIONS as Parameters<typeof client.chat.send>[0]['chatGenerationParams']['tools'],
        maxTokens: 16384,
        stream: false,
      },
    });

    const choice = response.choices?.[0];
    if (!choice?.message) break;

    const assistantMessage = choice.message;

    const storedMessage: ConversationMessage = {
      role: 'assistant',
      content: assistantMessage.content as string | null ?? null,
    };

    if (assistantMessage.toolCalls?.length) {
      storedMessage.toolCalls = assistantMessage.toolCalls.map((tc) => ({
        id: tc.id,
        type: 'function' as const,
        function: { name: tc.function.name, arguments: tc.function.arguments },
      }));
    }

    messages.push(storedMessage);

    if (typeof assistantMessage.content === 'string' && assistantMessage.content) {
      lastTextResponse = assistantMessage.content;
    }

    if (!assistantMessage.toolCalls?.length) {
      saveConversation(messages);
      return { textResponse: lastTextResponse, closedDuringQuery };
    }

    // Execute tool calls
    for (const toolCall of assistantMessage.toolCalls) {
      const toolName = toolCall.function.name;
      let toolArgs: Record<string, unknown>;
      try { toolArgs = JSON.parse(toolCall.function.arguments); }
      catch { toolArgs = {}; }

      log(`Tool: ${toolName} (${JSON.stringify(toolArgs).slice(0, 200)})`);

      let result: string;
      try { result = await executeTool(toolName, toolArgs); }
      catch (err) { result = `Tool error: ${err instanceof Error ? err.message : String(err)}`; }

      messages.push({ role: 'tool', content: result, toolCallId: toolCall.id });
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

  // Extract API key and model from secrets/env
  // All key types route through OpenRouter — provider-specific keys (Anthropic,
  // OpenAI, Google) are passed as the API key and OpenRouter forwards them.
  const apiKey = containerInput.secrets?.OPENROUTER_API_KEY
    || process.env.OPENROUTER_API_KEY
    || '';

  const modelId = containerInput.secrets?.MODEL_ID
    || process.env.MODEL_ID
    || 'anthropic/claude-opus-4-6';

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
  log(`API key: ${apiKey ? 'set' : 'MISSING'}`);

  // Initialize OpenRouter client (all key types route through OpenRouter)
  const client = new OpenRouter({ apiKey });

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
        client, modelId, prompt, systemPrompt, containerInput.assistantName, isNewSession,
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
