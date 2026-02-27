/**
 * Solana Configuration Management
 * Handles loading and validation of Solana config
 */

import fs from 'fs/promises';
import path from 'path';

export interface SolanaConfig {
  wallet: {
    signingMethod: 'standard' | 'crossmint';
    privateKey?: string;
    publicKey: string;
    crossmintApiKey?: string;
    crossmintEnvironment?: string;
  };
  preferences: {
    rpcUrl: string;
    defaultSlippage: number;
  };
  setupComplete: boolean;
  setupDate: string;
}

/**
 * Load Solana configuration from file
 */
export async function loadSolanaConfig(
  configPath: string = 'config/solana-config.json'
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

  return config;
}

/**
 * Check if Solana is configured
 */
export async function isSolanaConfigured(
  configPath: string = 'config/solana-config.json'
): Promise<boolean> {
  try {
    const fullPath = path.resolve(configPath);
    await fs.access(fullPath);
    const config = await loadSolanaConfig(configPath);

    if (config.wallet.signingMethod === 'crossmint') {
      return config.setupComplete && !!config.wallet.crossmintApiKey;
    }

    // Standard signing
    return config.setupComplete && !!config.wallet.privateKey;
  } catch {
    return false;
  }
}
