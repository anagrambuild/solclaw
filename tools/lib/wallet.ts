/**
 * Shared wallet/config loader for CLI tools.
 * Reads config/solana-config.json and returns connection + keypair.
 */

import fs from 'fs';
import path from 'path';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';

export const COMMON_TOKENS: Record<string, string> = {
  // Native & wrapped
  SOL: 'So11111111111111111111111111111111111111112',
  WSOL: 'So11111111111111111111111111111111111111112',

  // Stablecoins
  USDC: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  USDT: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
  PYUSD: '2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo',
  EURC: 'HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr',
  USDS: 'USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA',

  // DeFi & infrastructure
  JUP: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN',
  RAY: '4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R',
  ORCA: 'orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE',
  DRIFT: 'DriFtupJYLTosbwoN8koMbEYSx54aFAVLddWsbksjwg7',
  KMNO: 'KMNo3nJsBXfcpJTVhZcXLW7RmTwTt4GVFE7suUBo9sS',
  MNDE: 'MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey',
  STEP: 'StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT',

  // Liquid staking
  MSOL: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So',
  JITOSOL: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn',
  JUPSOL: 'jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v',
  BSOL: 'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1',
  INF: '5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm',

  // Jupiter products
  JLP: '27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4',
  JUPUSD: 'JuprjznTrTSp2UFa3ZBUFgwdAmtZCq4MQCwysN55USD',

  // Governance & L1
  JTO: 'jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL',
  PYTH: 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3',
  W: '85VBFQZC9TZkfaptBWjvUw7YbZjy52A6mjtPGjstQAmQ',
  TNSR: 'TNSRxcUxoT9xBG3de7PiJyTDYu7kskLqcpddxnEJAS6',
  PENGU: '2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv',
  ACS: '5MAYDfq5yxtudAhtfyuMBuHZjgAbaS9tbEyEQYAhDS5y',
  NOS: 'nosXBVoaCTtYdLvKY6Csb4AC8JCdQKKAaWYtx2ZMoo7',
  CLOUD: 'CLoUDKc4Ane7HeQcPpE3YHnznRxhMimJ4MyaUqyHFzAu',

  // DePIN & compute
  RENDER: 'rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof',
  HNT: 'hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux',
  MOBILE: 'mb1eu7TzEc71KxDpsmsKoucSSuuoGLv1drys1oP2jh6',
  HONEY: '4vMsoUT2BWatFweudnQM1xedRLfJgJ7hswhcpz4xgBTy',
  GRASS: 'Grass7B4RdKfBCjTKgSqnXkqjwiGvQyFbuSCUJr3XXjs',

  // Memecoins
  BONK: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',
  WIF: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm',
  POPCAT: '7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr',
  MEW: 'MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5',
  WEN: 'WENWENvqqNya429ubCdR81ZmD69brwQaaBYY6p3LCpk',
  MYRO: 'HhJpBhRRn4g56VsyLuT8DL5Bv31HkXqsrahTTUCZeZg4',
  SAMO: '7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU',
  BOME: 'ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82',
  SLERF: '9999FVbjHioTcoJpoBiSjpxHW6xEn3witVuXKqBh2RFQ',
  FARTCOIN: '9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump',

  // Political / trending
  TRUMP: '6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN',

  // Bridged assets
  WBTC: '3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh',
  ETH: '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs',
  CBBTC: 'cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij',
};

export interface WalletConfig {
  keypair: Keypair;
  connection: Connection;
  publicKey: PublicKey;
  rpcUrl: string;
}

export function loadWallet(configPath = 'config/solana-config.json'): WalletConfig {
  // Dashboard-injected key takes priority over config file
  const injectedKey = process.env.SOLCLAW_WALLET_PRIVATE_KEY;
  if (injectedKey) {
    const secretKey = bs58.decode(injectedKey);
    const keypair = Keypair.fromSecretKey(secretKey);

    // Still read config for RPC URL preference if available
    let rpcUrl = 'https://api.breeze.baby/agent/rpc-mainnet-beta';
    try {
      const raw = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      if (raw.preferences?.rpcUrl) rpcUrl = raw.preferences.rpcUrl;
    } catch {
      // No config file — use default RPC
    }

    const connection = new Connection(rpcUrl, 'confirmed');
    return { keypair, connection, publicKey: keypair.publicKey, rpcUrl };
  }

  const raw = JSON.parse(fs.readFileSync(configPath, 'utf-8'));

  const method = raw.wallet?.signingMethod ?? (raw.wallet?.provider === 'solana-agent-kit' ? 'standard' : raw.wallet?.signingMethod);
  if (method === 'crossmint') {
    console.error('Error: This tool requires a local keypair. Your config uses Crossmint signing.');
    console.error('Use the Crossmint MCP tools instead (crossmint_get_balance, crossmint_transfer, etc.).');
    process.exit(1);
  }

  if (!raw.wallet?.privateKey) {
    console.error('Error: No private key found in config. Run: npm run setup:solana');
    process.exit(1);
  }

  const secretKey = bs58.decode(raw.wallet.privateKey);
  const keypair = Keypair.fromSecretKey(secretKey);
  const rpcUrl = raw.preferences?.rpcUrl ?? 'https://api.breeze.baby/agent/rpc-mainnet-beta';
  const connection = new Connection(rpcUrl, 'confirmed');

  return { keypair, connection, publicKey: keypair.publicKey, rpcUrl };
}

/** Resolve a token symbol or mint address to a mint address */
export function resolveMint(tokenOrMint: string): string {
  const upper = tokenOrMint.toUpperCase();
  return COMMON_TOKENS[upper] ?? tokenOrMint;
}

/** Jupiter API base — uses free lite-api unless JUPITER_API_KEY is set */
export function jupiterBase(): string {
  return process.env.JUPITER_API_KEY ? 'https://api.jup.ag' : 'https://lite-api.jup.ag';
}

const IPC_TRANSACTIONS_DIR = '/workspace/ipc/transactions';

// ── Direct API sync (best-effort, non-blocking) ──────────────────────────────
async function syncTransactionToApi(
  signature: string,
  protocol: string,
  walletAddress: string,
  mint?: string | null,
  amount?: string | null,
): Promise<void> {
  const apiUrl =
    process.env.TRANSACTION_SYNC_API_URL ||
    'https://api.breeze.baby/agent/stats-sync-up';
  const entry: Record<string, unknown> = { signature, protocol, wallet_address: walletAddress };
  if (mint)   entry.mint   = mint;
  if (amount) entry.amount = parseFloat(amount);
  try {
    await fetch(apiUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ transaction_entries: [entry] }),
      signal: AbortSignal.timeout(10_000),
    });
  } catch {
    // best-effort — IPC file is the durable fallback
  }
}

function logTransactionIpc(
  signature: string,
  protocol: string,
  walletAddress: string,
  mint?: string,
  amount?: string,
): void {
  try {
    fs.mkdirSync(IPC_TRANSACTIONS_DIR, { recursive: true });
    const filename = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}.json`;
    const filepath = path.join(IPC_TRANSACTIONS_DIR, filename);
    const data = {
      type: 'log_transaction',
      signature,
      protocol,
      wallet_address: walletAddress,
      mint: mint || null,
      amount: amount || null,
      timestamp: new Date().toISOString(),
    };
    const tempPath = `${filepath}.tmp`;
    fs.writeFileSync(tempPath, JSON.stringify(data, null, 2));
    fs.renameSync(tempPath, filepath);
  } catch (err) {
    console.error(`[logTransactionIpc] Failed to log transaction: ${err instanceof Error ? err.message : String(err)}`);
  }

  // Fire-and-forget direct API sync
  void syncTransactionToApi(signature, protocol, walletAddress, mint, amount);
}

export { logTransactionIpc };

export interface SendAndLogOpts {
  protocol: string;
  mint?: string;
  amount?: string;
  commitment?: 'confirmed' | 'finalized';
}

/**
 * Sign, send, confirm, and log a transaction — all in one call.
 * USE THIS instead of connection.sendRawTransaction() directly.
 * Logging is automatic and cannot be skipped.
 *
 * Tries VersionedTransaction first, falls back to legacy Transaction.
 *
 * @returns Transaction signature
 */
export async function signSendAndLog(
  connection: Connection,
  keypair: Keypair,
  txData: Buffer | Uint8Array | string,
  opts: SendAndLogOpts,
): Promise<string> {
  const { VersionedTransaction, Transaction } = await import('@solana/web3.js');
  const bytes = typeof txData === 'string'
    ? Buffer.from(txData, 'base64')
    : txData;

  let sig: string;
  try {
    const tx = VersionedTransaction.deserialize(bytes);
    tx.sign([keypair]);
    sig = await connection.sendRawTransaction(tx.serialize());
  } catch {
    const tx = Transaction.from(bytes);
    tx.partialSign(keypair);
    sig = await connection.sendRawTransaction(tx.serialize());
  }

  await connection.confirmTransaction(sig, opts.commitment || 'confirmed');
  logTransactionIpc(sig, opts.protocol, keypair.publicKey.toBase58(), opts.mint, opts.amount);

  return sig;
}
