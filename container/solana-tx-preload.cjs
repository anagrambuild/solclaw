/**
 * Node.js preload script that intercepts fetch to auto-log
 * every Solana transaction without requiring explicit logging calls.
 *
 * Loaded via NODE_OPTIONS="--require /app/solana-tx-preload.cjs"
 *
 * Four patch layers for complete coverage:
 *  1. globalThis.fetch — native fetch (undici), @solana/kit, @solana/web3.js v1, cross-fetch
 *  2. Module._load('node-fetch') — CJS require('node-fetch') from any nested dep
 *  3. Module._load('undici') — CJS require('undici').fetch from any nested dep
 *  4. http/https.request — ESM imports of node-fetch, axios, got, any HTTP lib
 *     (with gzip/deflate/brotli decompression)
 *
 * Catches:
 *  - Any JSON-RPC "sendTransaction" call
 *  - Jupiter Ultra POST /execute responses
 *
 * Duplicate IPC files from overlapping layers are deduplicated by signature
 * in the host's ipc.ts processor.
 */

'use strict';

const fs = require('fs');
const path = require('path');
const Module = require('module');
const http = require('http');
const https = require('https');
const zlib = require('zlib');

const IPC_DIR = '/workspace/ipc/transactions';

function writeIpcFile(signature) {
  try {
    fs.mkdirSync(IPC_DIR, { recursive: true });
    const filename = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}.json`;
    const filepath = path.join(IPC_DIR, filename);
    const data = {
      type: 'log_transaction',
      signature,
      protocol: 'auto',
      wallet_address: 'auto',
      mint: null,
      amount: null,
      timestamp: new Date().toISOString(),
    };
    const tmp = filepath + '.tmp';
    fs.writeFileSync(tmp, JSON.stringify(data));
    fs.renameSync(tmp, filepath);
  } catch (_) {
    // Silent — never break the actual request
  }
}

// Base58 signature pattern: 87-88 chars of [1-9A-HJ-NP-Za-km-z]
const SIG_RE = /^[1-9A-HJ-NP-Za-km-z]{87,88}$/;

function isSolanaSignature(s) {
  return typeof s === 'string' && SIG_RE.test(s);
}

/**
 * Extract signatures from parsed JSON-RPC response(s) and write IPC files.
 */
function extractSignatures(json) {
  const responses = Array.isArray(json) ? json : [json];
  for (const resp of responses) {
    const sig = resp && resp.result;
    if (isSolanaSignature(sig)) {
      writeIpcFile(sig);
    }
  }
}

/**
 * Check if a body string contains a sendTransaction JSON-RPC call.
 */
function bodyHasSendTransaction(bodyStr) {
  if (!bodyStr || !bodyStr.includes('"sendTransaction"')) return false;
  try {
    const parsed = JSON.parse(bodyStr);
    if (Array.isArray(parsed)) {
      return parsed.some((r) => r && r.method === 'sendTransaction');
    }
    return parsed && parsed.method === 'sendTransaction';
  } catch (_) {
    return false;
  }
}

/**
 * Decompress a buffer based on content-encoding header.
 */
function decompressSync(buf, encoding) {
  switch ((encoding || '').toLowerCase()) {
    case 'gzip':
    case 'x-gzip':
      return zlib.gunzipSync(buf);
    case 'deflate':
      return zlib.inflateSync(buf);
    case 'br':
      return zlib.brotliDecompressSync(buf);
    default:
      return buf;
  }
}

// ============================================================
// Patch 1: globalThis.fetch
// Catches: native fetch (undici), @solana/kit v2, @solana/web3.js
// v1 on Node 18+, cross-fetch, axios with adapter:'fetch'
// ============================================================

function classifyFetchRequest(input, init) {
  let url = '';
  let bodyStr = '';
  try {
    if (typeof input === 'string') {
      url = input;
    } else if (input instanceof URL) {
      url = input.href;
    } else if (input && typeof input === 'object' && 'url' in input) {
      url = String(input.url);
    }
    if (init && init.body) {
      bodyStr = typeof init.body === 'string' ? init.body : '';
    } else if (input && typeof input === 'object' && 'body' in input && input.body) {
      bodyStr = typeof input.body === 'string' ? String(input.body) : '';
    }
  } catch (_) {
    // Fall through
  }

  return {
    isRpcSend: bodyHasSendTransaction(bodyStr),
    isJupExecute: url.includes('jup.ag') && url.includes('/execute'),
  };
}

function wrapFetch(originalFn) {
  const wrapped = async function wrappedFetch(input, init) {
    const { isRpcSend, isJupExecute } = classifyFetchRequest(input, init);

    if (!isRpcSend && !isJupExecute) {
      return originalFn.apply(this, arguments);
    }

    const response = await originalFn.apply(this, arguments);

    // Clone so we can read without consuming the caller's body
    try {
      const clone = response.clone();
      const text = await clone.text();
      const json = JSON.parse(text);

      if (isRpcSend) {
        extractSignatures(json);
      } else if (isJupExecute) {
        const sig = json && (json.signature || json.txSignature);
        if (isSolanaSignature(sig)) {
          writeIpcFile(sig);
        }
      }
    } catch (_) {
      // Silent
    }

    return response;
  };

  // Preserve .name for debugging (Object.defineProperty since functions are non-writable)
  try { Object.defineProperty(wrapped, 'name', { value: originalFn.name || 'fetch' }); } catch (_) {}
  return wrapped;
}

if (typeof globalThis.fetch === 'function') {
  globalThis.fetch = wrapFetch(globalThis.fetch);
}

// ============================================================
// Patch 2: node-fetch via Module._load
// Catches: CJS require('node-fetch') from any nested dependency
// ============================================================

let wrappedNodeFetch = null;
let wrappedUndici = null;
const origLoad = Module._load;

Module._load = function patchedLoad(request) {
  const exports = origLoad.apply(this, arguments);

  // --- node-fetch ---
  if (request === 'node-fetch') {
    if (!wrappedNodeFetch) {
      // v2 (CJS): module.exports = fetch function
      // v3 (ESM via --experimental-require-module): exports.default = fetch function
      const origFn = typeof exports === 'function' ? exports : (exports && exports.default);
      if (typeof origFn === 'function') {
        const wrapped = wrapFetch(origFn);
        if (typeof exports === 'function') {
          Object.assign(wrapped, exports);
          wrappedNodeFetch = wrapped;
        } else {
          wrappedNodeFetch = Object.assign({}, exports, { default: wrapped });
        }
      }
    }
    return wrappedNodeFetch || exports;
  }

  // --- undici ---
  if (request === 'undici') {
    if (!wrappedUndici && exports && typeof exports.fetch === 'function') {
      wrappedUndici = Object.assign({}, exports, { fetch: wrapFetch(exports.fetch) });
    }
    return wrappedUndici || exports;
  }

  return exports;
};

// ============================================================
// Patch 3: http/https.request
// Catches: ESM imports of node-fetch (uses http/https internally),
// axios, got, cross-fetch, any HTTP library.
// Native fetch (undici) does NOT go through http/https — Patch 1
// covers that.
//
// Handles gzip/deflate/brotli compressed responses since
// node-fetch sends Accept-Encoding by default.
// ============================================================

function patchHttpModule(mod) {
  const origRequest = mod.request;

  mod.request = function patchedRequest() {
    const req = origRequest.apply(this, arguments);

    const bodyChunks = [];
    const origWrite = req.write;
    const origEnd = req.end;

    req.write = function patchedWrite(chunk) {
      if (chunk != null) {
        bodyChunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : chunk);
      }
      return origWrite.apply(this, arguments);
    };

    req.end = function patchedEnd(chunk) {
      if (chunk != null && typeof chunk !== 'function') {
        bodyChunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : chunk);
      }

      try {
        const body = Buffer.concat(bodyChunks).toString();
        if (bodyHasSendTransaction(body)) {
          // Add response listener BEFORE calling origEnd (which sends the request)
          req.on('response', function onTxResponse(res) {
            const resChunks = [];
            res.on('data', (c) => resChunks.push(c));
            res.on('end', () => {
              try {
                const raw = Buffer.concat(resChunks);
                const encoding = res.headers && res.headers['content-encoding'];
                const decompressed = decompressSync(raw, encoding);
                const json = JSON.parse(decompressed.toString());
                extractSignatures(json);
              } catch (_) {
                // Silent
              }
            });
          });
        }
      } catch (_) {
        // Silent
      }

      return origEnd.apply(this, arguments);
    };

    return req;
  };
}

patchHttpModule(http);
patchHttpModule(https);
