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
 * Every caught transaction is:
 *  1. Written as an IPC JSON file (for host-side ipc.ts watcher)
 *  2. POSTed directly to the Breeze stats-sync-up API (for Fly.io where no watcher runs)
 *
 * Protocol is detected from the transaction's program IDs — never 'auto'.
 * Wallet address is read from the Solana config keypair — never 'auto'.
 *
 * Duplicate IPC files from overlapping layers are deduplicated by signature
 * in the host's ipc.ts processor and by the stats-sync-up endpoint.
 */

'use strict';

const fs = require('fs');
const path = require('path');
const Module = require('module');
const http = require('http');
const https = require('https');
const zlib = require('zlib');

const IPC_DIR = '/workspace/ipc/transactions';
const SYNC_API_URL = process.env.TRANSACTION_SYNC_API_URL || 'https://api.breeze.baby/agent/stats-sync-up';

// Save a reference to the original fetch BEFORE we patch it,
// so syncToApi can make HTTP calls without triggering our interceptor.
const _origFetch = typeof globalThis.fetch === 'function' ? globalThis.fetch.bind(globalThis) : null;

// ============================================================
// Minimal Base58 codec (Bitcoin/Solana alphabet)
// ============================================================

const B58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
const B58_MAP = {};
for (let i = 0; i < B58_ALPHABET.length; i++) B58_MAP[B58_ALPHABET[i]] = BigInt(i);

function b58decode(str) {
  let num = 0n;
  for (const ch of str) {
    const val = B58_MAP[ch];
    if (val === undefined) return null;
    num = num * 58n + val;
  }
  let leadingZeros = 0;
  for (const ch of str) {
    if (ch !== '1') break;
    leadingZeros++;
  }
  if (num === 0n) return Buffer.alloc(leadingZeros);
  const hex = num.toString(16);
  const padded = hex.length % 2 ? '0' + hex : hex;
  return Buffer.concat([Buffer.alloc(leadingZeros), Buffer.from(padded, 'hex')]);
}

function b58encode(buf) {
  let num = 0n;
  for (const byte of buf) num = num * 256n + BigInt(byte);
  let str = '';
  while (num > 0n) {
    str = B58_ALPHABET[Number(num % 58n)] + str;
    num /= 58n;
  }
  for (const byte of buf) {
    if (byte !== 0) break;
    str = '1' + str;
  }
  return str || '1';
}

// ============================================================
// Wallet address resolution (cached, lazy)
// Reads the Solana config to extract the public key from the
// Ed25519 keypair (bytes 32-63 of the 64-byte secret key).
// ============================================================

let _cachedWalletAddress = null;

function getWalletAddress() {
  if (_cachedWalletAddress) return _cachedWalletAddress;
  const configPaths = [
    '/workspace/group/config/solana-config.json',
    '/workspace/project/config/solana-config.json',
    path.join(process.cwd(), 'config/solana-config.json'),
  ];
  for (const configPath of configPaths) {
    try {
      const raw = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      const pk = raw.wallet && raw.wallet.privateKey;
      if (pk && typeof pk === 'string') {
        const decoded = b58decode(pk);
        if (decoded && decoded.length === 64) {
          _cachedWalletAddress = b58encode(decoded.slice(32));
          return _cachedWalletAddress;
        }
      }
    } catch (_) {
      // Try next path
    }
  }
  return null;
}

// ============================================================
// Protocol detection from transaction program IDs
// ============================================================

const KNOWN_PROGRAMS = {
  // Jupiter
  'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN': 'jupiter',
  // Drift
  'dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH': 'drift',
  // Breeze
  'brzp8fNvHCBRi8UcCnTQUgQ2bQ4JnJTJtCvPzpKf2ty': 'breeze',
  // Swig
  'swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB': 'swig',
  // Kamino
  'KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD': 'kamino',
  '6LtLpnUFNByNXLyCoK9wA2MykKAmQNZKBdY8s47dehDc': 'kamino',
  'FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr': 'kamino',
  // Marginfi
  'MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA': 'marginfi',
  // Orca
  'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc': 'orca',
  // Raydium
  '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8': 'raydium',
  'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK': 'raydium',
  'CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C': 'raydium',
  'routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS': 'raydium',
  // Meteora
  'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo': 'meteora',
  'cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG': 'meteora',
  'dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN': 'meteora',
  '24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi': 'meteora',
  'FEESngU3neckdwib9X3KWqdL7Mjmqk9XNp3uh5JbP4KP': 'meteora',
  // PumpFun
  '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P': 'pumpfun',
  'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA': 'pumpfun',
  // Metaplex
  'CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d': 'metaplex',
  'metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s': 'metaplex',
  'BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY': 'metaplex',
  'CndyV3LdqHUfDLmE5naZjVN8rBZz4tqhdefbAnjHG3JR': 'metaplex',
  'Guard1JwRhJkVH6XZhzoYxeBVQe872VH6QggF4BWmS9g': 'metaplex',
  'CMACYFENjoBMHzapRXyo1JZkVS6EtaDDzkjMrmQLvr4J': 'metaplex',
  'CMAGAKJ67e9hRZgfC5SFTbZH8MgEmtqazKXjmkaJjWTJ': 'metaplex',
  'MPL4o4wMzndgh8T1NVDxELQCj5UQfYTYEkabX3wNKtb': 'metaplex',
  '1NSCRfGeyo7wPUazGbaPBUsTM49e1k2aXewHGARfzSo': 'metaplex',
  // DFlow
  'DF1ow3DqMj3HvTj8i8J9yM2hE9hCrLLXpdbaKZu4ZPnz': 'dflow',
};

// System/infra programs to skip when falling back to first program ID
const SYSTEM_PROGRAMS = new Set([
  '11111111111111111111111111111111',                 // System Program
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',    // SPL Token
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',   // SPL Token 2022
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',   // Associated Token Account
  'ComputeBudget111111111111111111111111111111',       // Compute Budget
  'SysvarRent111111111111111111111111111111111',       // Rent Sysvar
  'SysvarC1ock11111111111111111111111111111111',       // Clock Sysvar
  'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr',   // Memo v2
  'Memo1UhkJBfCR6MNB2Gp46kUY4R4iorgqS7GEJTr8Lj',   // Memo v1
]);

/** Read compact-u16 from a buffer at offset. Returns { value, size }. */
function readCompactU16(buf, off) {
  const b0 = buf[off];
  if (b0 < 0x80) return { value: b0, size: 1 };
  const b1 = buf[off + 1];
  if (b1 < 0x80) return { value: (b0 & 0x7f) | (b1 << 7), size: 2 };
  return { value: (b0 & 0x7f) | ((b1 & 0x7f) << 7) | (buf[off + 2] << 14), size: 3 };
}

/**
 * Parse a base64-encoded Solana transaction to extract program IDs,
 * then match against known protocols.
 * Returns protocol name string or null.
 */
function detectProtocolFromTx(base64Tx) {
  try {
    const buf = Buffer.from(base64Tx, 'base64');
    let off = 0;

    // Versioned transactions have high bit set on first byte
    if ((buf[0] & 0x80) !== 0) off = 1;

    // Skip signatures
    const sigs = readCompactU16(buf, off);
    off += sigs.size + sigs.value * 64;

    // Message header (3 bytes)
    off += 3;

    // Account keys
    const keys = readCompactU16(buf, off);
    off += keys.size;
    const accountKeys = [];
    for (let i = 0; i < keys.value; i++) {
      accountKeys.push(b58encode(buf.slice(off, off + 32)));
      off += 32;
    }

    // Skip recent blockhash
    off += 32;

    // Instructions — collect program IDs
    const ixCount = readCompactU16(buf, off);
    off += ixCount.size;

    const programIds = [];
    for (let i = 0; i < ixCount.value; i++) {
      const progIdx = buf[off]; off += 1;
      if (progIdx < accountKeys.length) {
        programIds.push(accountKeys[progIdx]);
      }
      // Skip account indexes
      const accts = readCompactU16(buf, off);
      off += accts.size + accts.value;
      // Skip instruction data
      const data = readCompactU16(buf, off);
      off += data.size + data.value;
    }

    // Match against known protocols (first match wins)
    for (const pid of programIds) {
      if (KNOWN_PROGRAMS[pid]) return KNOWN_PROGRAMS[pid];
    }

    // Fallback: use first non-system program ID as the protocol identifier
    for (const pid of programIds) {
      if (!SYSTEM_PROGRAMS.has(pid)) return pid;
    }

    return 'transfer';
  } catch (_) {
    return null;
  }
}

/**
 * Extract base64 tx payloads from a sendTransaction request body
 * and detect protocol from the first one.
 */
function detectProtocolFromBody(bodyStr) {
  try {
    const parsed = JSON.parse(bodyStr);
    const requests = Array.isArray(parsed) ? parsed : [parsed];
    for (const req of requests) {
      if (req && req.method === 'sendTransaction' && req.params && req.params[0]) {
        const protocol = detectProtocolFromTx(req.params[0]);
        if (protocol) return protocol;
      }
    }
  } catch (_) {}
  return null;
}

// ============================================================
// Direct API sync (fire-and-forget, best-effort)
// On Fly.io there is no host-side IPC watcher or sync loop,
// so the preload itself POSTs every tx to the Breeze API.
// ============================================================

function syncToApi(signature, protocol) {
  const walletAddress = getWalletAddress();
  if (!walletAddress || !_origFetch) return;
  try {
    const entry = { signature, wallet_address: walletAddress };
    if (protocol) entry.protocol = protocol;
    _origFetch(SYNC_API_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ transaction_entries: [entry] }),
      signal: AbortSignal.timeout(10000),
    }).catch(function () {});
  } catch (_) {
    // Never break the actual request
  }
}

// ============================================================
// IPC file writer + API sync
// ============================================================

function writeIpcFile(signature, protocol) {
  const walletAddress = getWalletAddress();
  try {
    fs.mkdirSync(IPC_DIR, { recursive: true });
    const filename = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}.json`;
    const filepath = path.join(IPC_DIR, filename);
    const data = {
      type: 'log_transaction',
      signature,
      protocol: protocol || null,
      wallet_address: walletAddress || null,
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

  // Fire-and-forget direct API sync (works on Fly.io where no IPC watcher runs)
  syncToApi(signature, protocol);
}

// Base58 signature pattern: 87-88 chars of [1-9A-HJ-NP-Za-km-z]
const SIG_RE = /^[1-9A-HJ-NP-Za-km-z]{87,88}$/;

function isSolanaSignature(s) {
  return typeof s === 'string' && SIG_RE.test(s);
}

/**
 * Extract signatures from parsed JSON-RPC response(s) and write IPC files.
 */
function extractSignatures(json, protocol) {
  const responses = Array.isArray(json) ? json : [json];
  for (const resp of responses) {
    const sig = resp && resp.result;
    if (isSolanaSignature(sig)) {
      writeIpcFile(sig, protocol);
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
    bodyStr,
  };
}

function wrapFetch(originalFn) {
  const wrapped = async function wrappedFetch(input, init) {
    const { isRpcSend, isJupExecute, bodyStr } = classifyFetchRequest(input, init);

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
        const protocol = detectProtocolFromBody(bodyStr);
        extractSignatures(json, protocol);
      } else if (isJupExecute) {
        const sig = json && (json.signature || json.txSignature);
        if (isSolanaSignature(sig)) {
          writeIpcFile(sig, 'jupiter');
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
          const protocol = detectProtocolFromBody(body);
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
                extractSignatures(json, protocol);
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
