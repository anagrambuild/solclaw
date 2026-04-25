/**
 * Standard tool definitions and executors.
 * Provides: Bash, Read, Write, Edit, Glob, Grep, WebFetch, WebSearch
 */

import fs from 'fs';
import path from 'path';
import { execFile } from 'child_process';

// ── Tool Definitions (OpenAI function-calling format) ──────────────────────

export interface ToolDefinition {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
  };
}

export const STANDARD_TOOL_DEFINITIONS: ToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'Bash',
      description: 'Execute a bash command. Use for git, npm, system commands. Working directory persists across calls.',
      parameters: {
        type: 'object',
        properties: {
          command: { type: 'string', description: 'The bash command to execute' },
          timeout: { type: 'number', description: 'Timeout in milliseconds (max 600000). Default 120000.' },
        },
        required: ['command'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'Read',
      description: 'Read a file from the filesystem. Returns contents with line numbers.',
      parameters: {
        type: 'object',
        properties: {
          file_path: { type: 'string', description: 'Absolute path to the file to read' },
          offset: { type: 'number', description: 'Line number to start reading from (1-based)' },
          limit: { type: 'number', description: 'Number of lines to read' },
        },
        required: ['file_path'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'Write',
      description: 'Write content to a file, creating parent directories as needed.',
      parameters: {
        type: 'object',
        properties: {
          file_path: { type: 'string', description: 'Absolute path to the file to write' },
          content: { type: 'string', description: 'The content to write' },
        },
        required: ['file_path', 'content'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'Edit',
      description: 'Replace an exact string in a file. old_string must be unique in the file.',
      parameters: {
        type: 'object',
        properties: {
          file_path: { type: 'string', description: 'Absolute path to the file to edit' },
          old_string: { type: 'string', description: 'The exact text to find and replace' },
          new_string: { type: 'string', description: 'The replacement text' },
        },
        required: ['file_path', 'old_string', 'new_string'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'Glob',
      description: 'Find files matching a glob pattern. Returns matching paths.',
      parameters: {
        type: 'object',
        properties: {
          pattern: { type: 'string', description: 'Glob pattern (e.g. "**/*.ts", "src/**/*.json")' },
          path: { type: 'string', description: 'Directory to search in. Defaults to cwd.' },
        },
        required: ['pattern'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'Grep',
      description: 'Search file contents using a regex pattern. Returns matching lines.',
      parameters: {
        type: 'object',
        properties: {
          pattern: { type: 'string', description: 'Regex pattern to search for' },
          path: { type: 'string', description: 'File or directory to search in. Defaults to cwd.' },
          glob: { type: 'string', description: 'Glob to filter files (e.g. "*.ts")' },
        },
        required: ['pattern'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'WebFetch',
      description: 'Fetch content from a URL and return it as text.',
      parameters: {
        type: 'object',
        properties: {
          url: { type: 'string', description: 'The URL to fetch' },
          prompt: { type: 'string', description: 'What to extract from the page' },
        },
        required: ['url'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'WebSearch',
      description: 'Search the web using DuckDuckGo. Returns search result snippets.',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'The search query' },
        },
        required: ['query'],
      },
    },
  },
];

// ── Secrets to strip from Bash subprocess environments ─────────────────────

const SECRET_ENV_VARS = new Set([
  'OPENROUTER_API_KEY',
  'ANTHROPIC_API_KEY',
  'CLAUDE_CODE_OAUTH_TOKEN',
  'DFLOW_API_KEY',
  'JUPITER_API_KEY',
  'BREEZE_API_KEY',
  'HELIUS_API_KEY',
  'SOLCLAW_WALLET_PRIVATE_KEY',
]);

function sanitizedEnv(): Record<string, string> {
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (value !== undefined && !SECRET_ENV_VARS.has(key)) {
      env[key] = value;
    }
  }
  return env;
}

// ── Tool Executors ─────────────────────────────────────────────────────────

const MAX_OUTPUT = 30_000;
const DEFAULT_TIMEOUT = 120_000;
const MAX_TIMEOUT = 600_000;
const MAX_READ_LINES = 2000;
const MAX_FETCH_CHARS = 15_000;

let currentCwd = '/workspace/group';

export function setCwd(cwd: string): void { currentCwd = cwd; }
export function getCwd(): string { return currentCwd; }

async function executeBash(args: { command: string; timeout?: number }): Promise<string> {
  const timeout = Math.min(args.timeout ?? DEFAULT_TIMEOUT, MAX_TIMEOUT);
  const wrappedCommand = `${args.command}\n__EXIT_CODE=$?\necho ""\necho "__CWD__=$(pwd)"\nexit $__EXIT_CODE`;

  return new Promise((resolve) => {
    const child = execFile(
      '/bin/bash',
      ['-c', wrappedCommand],
      {
        cwd: currentCwd,
        timeout,
        maxBuffer: 10 * 1024 * 1024,
        env: sanitizedEnv(),
      },
      (error, stdout, stderr) => {
        let output = stdout ?? '';

        const cwdMatch = output.match(/__CWD__=(.+)/);
        if (cwdMatch) {
          currentCwd = cwdMatch[1].trim();
          output = output.replace(/\n?__CWD__=.+/, '');
        }
        output = output.replace(/\n+$/, '\n');

        if (error) {
          const errMsg = stderr?.trim() || error.message;
          output = output.trim() ? `${output.trim()}\n\nSTDERR:\n${errMsg}` : errMsg;
        }

        if (output.length > MAX_OUTPUT) {
          output = output.slice(0, MAX_OUTPUT) + `\n\n[output truncated — ${output.length} chars total]`;
        }

        resolve(output.trim());
      }
    );

    setTimeout(() => { try { child.kill('SIGKILL'); } catch { /* already exited */ } }, timeout + 5000);
  });
}

function executeRead(args: { file_path: string; offset?: number; limit?: number }): string {
  if (!fs.existsSync(args.file_path)) return `Error: file not found: ${args.file_path}`;
  const stat = fs.statSync(args.file_path);
  if (stat.isDirectory()) return `Error: ${args.file_path} is a directory, not a file`;

  const content = fs.readFileSync(args.file_path, 'utf-8');
  const lines = content.split('\n');
  const offset = Math.max((args.offset ?? 1) - 1, 0);
  const limit = args.limit ?? MAX_READ_LINES;
  const slice = lines.slice(offset, offset + limit);

  const padWidth = String(offset + slice.length).length;
  return slice.map((line, i) => {
    const lineNum = String(offset + i + 1).padStart(padWidth);
    const truncated = line.length > 2000 ? line.slice(0, 2000) + '...' : line;
    return `${lineNum}\t${truncated}`;
  }).join('\n');
}

function executeWrite(args: { file_path: string; content: string }): string {
  fs.mkdirSync(path.dirname(args.file_path), { recursive: true });
  fs.writeFileSync(args.file_path, args.content);
  return `Wrote ${args.content.length} bytes to ${args.file_path}`;
}

function executeEdit(args: { file_path: string; old_string: string; new_string: string }): string {
  if (!fs.existsSync(args.file_path)) return `Error: file not found: ${args.file_path}`;
  const content = fs.readFileSync(args.file_path, 'utf-8');

  let count = 0;
  let idx = 0;
  while ((idx = content.indexOf(args.old_string, idx)) !== -1) { count++; idx += args.old_string.length; }

  if (count === 0) return `Error: old_string not found in ${args.file_path}`;
  if (count > 1) return `Error: old_string found ${count} times in ${args.file_path} — must be unique.`;

  fs.writeFileSync(args.file_path, content.replace(args.old_string, args.new_string));
  return `Edited ${args.file_path}`;
}

async function executeGlob(args: { pattern: string; path?: string }): Promise<string> {
  const searchDir = args.path || currentCwd;
  return new Promise((resolve) => {
    execFile('/bin/bash', ['-c', `find ${searchDir} -path '*/${args.pattern}' -o -name '${args.pattern}' 2>/dev/null | head -100`],
      { timeout: 10_000, maxBuffer: 1024 * 1024 },
      (error, stdout) => {
        if (!stdout?.trim()) {
          execFile('/bin/bash', ['-c', `find ${searchDir} -name '${args.pattern}' 2>/dev/null | head -100`],
            { timeout: 10_000, maxBuffer: 1024 * 1024 },
            (_err2, stdout2) => { resolve(stdout2?.trim() || 'No matches found'); });
          return;
        }
        resolve(stdout.trim().split('\n').slice(0, 100).join('\n'));
      });
  });
}

async function executeGrep(args: { pattern: string; path?: string; glob?: string }): Promise<string> {
  const searchPath = args.path || currentCwd;
  return new Promise((resolve) => {
    const grepArgs = ['-rn', '--color=never'];
    if (args.glob) grepArgs.push('--include', args.glob);
    grepArgs.push(args.pattern, searchPath);

    execFile('grep', grepArgs, { timeout: 15_000, maxBuffer: 2 * 1024 * 1024 }, (error, stdout) => {
      if (!stdout?.trim()) { resolve('No matches found'); return; }
      const lines = stdout.trim().split('\n');
      const result = lines.slice(0, 50);
      if (lines.length > 50) result.push(`\n[... truncated — showing 50 of ${lines.length} matches]`);
      resolve(result.join('\n'));
    });
  });
}

function stripHtml(html: string): string {
  return html
    .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/g, ' ').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'")
    .replace(/\s+/g, ' ').trim();
}

async function executeWebFetch(args: { url: string; prompt?: string }): Promise<string> {
  try {
    const res = await fetch(args.url, {
      headers: { 'User-Agent': 'SolClaw-Agent/1.0', Accept: 'text/html,application/json,text/plain' },
      signal: AbortSignal.timeout(15_000),
      redirect: 'follow',
    });
    if (!res.ok) return `Error: HTTP ${res.status} ${res.statusText}`;
    const contentType = res.headers.get('content-type') ?? '';
    const text = await res.text();
    let content = contentType.includes('html') ? stripHtml(text) : text;
    if (content.length > MAX_FETCH_CHARS) content = content.slice(0, MAX_FETCH_CHARS) + `\n\n[truncated — ${content.length} chars total]`;
    return content;
  } catch (err) {
    return `Error fetching ${args.url}: ${err instanceof Error ? err.message : String(err)}`;
  }
}

async function executeWebSearch(args: { query: string }): Promise<string> {
  try {
    const params = new URLSearchParams({ q: args.query });
    const res = await fetch(`https://lite.duckduckgo.com/lite/?${params}`, {
      headers: { 'User-Agent': 'Mozilla/5.0 (compatible; SolClaw-Agent/1.0)', Accept: 'text/html' },
      signal: AbortSignal.timeout(10_000),
    });
    if (!res.ok) return `Search error: HTTP ${res.status}`;
    const text = stripHtml(await res.text());
    return text.length > 8000 ? text.slice(0, 8000) + '\n\n[truncated]' : (text || 'No results found');
  } catch (err) {
    return `Search error: ${err instanceof Error ? err.message : String(err)}`;
  }
}

// ── Dispatcher ─────────────────────────────────────────────────────────────

export async function executeStandardTool(name: string, args: Record<string, unknown>): Promise<string | null> {
  switch (name) {
    case 'Bash': return executeBash(args as { command: string; timeout?: number });
    case 'Read': return executeRead(args as { file_path: string; offset?: number; limit?: number });
    case 'Write': return executeWrite(args as { file_path: string; content: string });
    case 'Edit': return executeEdit(args as { file_path: string; old_string: string; new_string: string });
    case 'Glob': return executeGlob(args as { pattern: string; path?: string });
    case 'Grep': return executeGrep(args as { pattern: string; path?: string; glob?: string });
    case 'WebFetch': return executeWebFetch(args as { url: string; prompt?: string });
    case 'WebSearch': return executeWebSearch(args as { query: string });
    default: return null; // Not a standard tool
  }
}
