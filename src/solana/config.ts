/**
 * Solana Configuration Management
 * Handles loading and validation of Solana config
 */

import fs from 'fs/promises';
import path from 'path';

export interface SolanaConfig {
  wallet: {
    signingMethod: 'standard' | 'crossmint' | 'swig';
    privateKey?: string;
    publicKey: string;
    crossmintApiKey?: string;
    crossmintEnvironment?: string;
    authorityPublicKey?: string;
    swigWalletAddress?: string;
    swigAccountAddress?: string;
    feeMode?: 'paymaster' | 'gas-sponsor' | 'self-funded';
    swigPaymasterPubkey?: string;
    swigPaymasterNetwork?: 'mainnet' | 'devnet';
    gasSponsorUrl?: string;
  };
  preferences: {
    rpcUrl: string;
    defaultSlippage: number;
  };
  setupComplete: boolean;
  setupDate: string;
}

/**
 * Load Solana configuration.
 * If SOLCLAW_WALLET_PRIVATE_KEY is set (dashboard injection), it overrides
 * the config file's wallet keys while preserving other preferences.
 */
export async function loadSolanaConfig(
  configPath: string = 'config/solana-config.json',
): Promise<SolanaConfig> {
  const fullPath = path.resolve(configPath);
  const configData = await fs.readFile(fullPath, 'utf-8');
  const raw = JSON.parse(configData);

  // Backwards compat: treat old provider: 'solana-agent-kit' as signingMethod: 'standard'
  if (raw.wallet && !raw.wallet.signingMethod) {
    if (raw.wallet.provider === 'solana-agent-kit' || raw.wallet.privateKey) {
      raw.wallet.signingMethod = 'standard';
    }
  }

  const config = raw as SolanaConfig;

  if (!config.wallet) {
    throw new Error('Invalid config: missing wallet configuration');
  }

  if (!config.setupComplete) {
    throw new Error('Setup incomplete. Run: npm run setup');
  }

  // Dashboard-injected key overrides config file wallet for direct keypair mode.
  // Swig uses a separate authority key and should not be coerced into standard mode.
  const injectedKey = process.env.SOLCLAW_WALLET_PRIVATE_KEY;
  if (injectedKey && config.wallet.signingMethod !== 'swig') {
    const { Keypair } = await import('@solana/web3.js');
    const bs58 = (await import('bs58')).default;
    const secretKey = bs58.decode(injectedKey);
    const kp = Keypair.fromSecretKey(secretKey);
    config.wallet.privateKey = injectedKey;
    config.wallet.publicKey = kp.publicKey.toBase58();
    config.wallet.signingMethod = 'standard';
  }

  return config;
}

/**
 * Check if Solana is configured
 */
export async function isSolanaConfigured(
  configPath: string = 'config/solana-config.json',
): Promise<boolean> {
  try {
    const fullPath = path.resolve(configPath);
    await fs.access(fullPath);
    const config = await loadSolanaConfig(configPath);

    if (config.wallet.signingMethod === 'crossmint') {
      return config.setupComplete && !!config.wallet.crossmintApiKey;
    }

    if (config.wallet.signingMethod === 'swig') {
      const authorityPublicKey =
        config.wallet.authorityPublicKey || config.wallet.publicKey;
      if (!authorityPublicKey) return false;

      if (config.wallet.feeMode === 'paymaster') {
        return config.setupComplete && !!config.wallet.swigPaymasterPubkey;
      }

      if (config.wallet.feeMode === 'gas-sponsor') {
        return config.setupComplete && !!config.wallet.gasSponsorUrl;
      }

      return config.setupComplete;
    }

    // Standard signing
    return config.setupComplete && !!config.wallet.privateKey;
  } catch {
    return false;
  }
}

function validateRpcUrl(rpcUrl: string): string {
  const normalized = rpcUrl.trim();
  if (!normalized) {
    throw new Error('RPC URL cannot be empty');
  }

  let parsed: URL;
  try {
    parsed = new URL(normalized);
  } catch {
    throw new Error('Invalid RPC URL format');
  }

  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error('RPC URL must use http:// or https://');
  }

  return normalized;
}

async function updateEnvSolanaRpcUrl(rpcUrl: string): Promise<void> {
  const envPath = path.resolve('.env.solana');

  let content = '';
  try {
    content = await fs.readFile(envPath, 'utf-8');
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code;
    if (code !== 'ENOENT') throw err;
  }

  const lines = content ? content.replace(/\r\n/g, '\n').split('\n') : [];
  const nextLines: string[] = [];
  let replaced = false;

  for (const line of lines) {
    if (line.startsWith('SOLANA_RPC_URL=')) {
      if (!replaced) {
        nextLines.push(`SOLANA_RPC_URL=${rpcUrl}`);
        replaced = true;
      }
      continue;
    }
    nextLines.push(line);
  }

  if (!replaced) {
    if (nextLines.length > 0 && nextLines[nextLines.length - 1] !== '') {
      nextLines.push('');
    }
    nextLines.push(`SOLANA_RPC_URL=${rpcUrl}`);
  }

  const nextContent = nextLines.join('\n');
  await fs.writeFile(
    envPath,
    nextContent.endsWith('\n') ? nextContent : `${nextContent}\n`,
  );
}

/**
 * Update RPC URL in Solana config and mirror it to .env.solana.
 */
export async function updateSolanaRpcUrl(
  rpcUrl: string,
  configPath: string = 'config/solana-config.json',
): Promise<{ previousRpcUrl?: string; rpcUrl: string }> {
  const normalizedRpcUrl = validateRpcUrl(rpcUrl);
  const fullPath = path.resolve(configPath);
  const configData = await fs.readFile(fullPath, 'utf-8');
  const raw = JSON.parse(configData) as SolanaConfig;

  if (!raw.preferences || typeof raw.preferences !== 'object') {
    raw.preferences = {
      rpcUrl: normalizedRpcUrl,
      defaultSlippage: 50,
    };
  }

  const previousRpcUrl = raw.preferences.rpcUrl;
  raw.preferences.rpcUrl = normalizedRpcUrl;

  await fs.writeFile(fullPath, `${JSON.stringify(raw, null, 2)}\n`);
  await updateEnvSolanaRpcUrl(normalizedRpcUrl);

  return {
    previousRpcUrl,
    rpcUrl: normalizedRpcUrl,
  };
}
