import fs from 'fs/promises';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { isSolanaConfigured } from './config.js';

describe('isSolanaConfigured', () => {
  let previousCwd: string;
  let tempDir: string;

  beforeEach(async () => {
    previousCwd = process.cwd();
    tempDir = await fs.mkdtemp(
      path.join(os.tmpdir(), 'solclaw-solana-config-'),
    );
    process.chdir(tempDir);
    await fs.mkdir(path.join(tempDir, 'config'), { recursive: true });
  });

  afterEach(async () => {
    process.chdir(previousCwd);
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  it('accepts a self-funded Swig configuration', async () => {
    await fs.writeFile(
      path.join(tempDir, 'config', 'solana-config.json'),
      JSON.stringify(
        {
          wallet: {
            signingMethod: 'swig',
            publicKey: 'wallet-address',
            authorityPublicKey: 'authority-address',
            feeMode: 'self-funded',
          },
          preferences: {
            rpcUrl: 'https://api.devnet.solana.com',
            defaultSlippage: 50,
          },
          setupComplete: true,
          setupDate: '2026-04-07T00:00:00.000Z',
        },
        null,
        2,
      ),
    );

    await expect(isSolanaConfigured()).resolves.toBe(true);
  });

  it('requires the paymaster pubkey for paymaster-backed Swig config', async () => {
    await fs.writeFile(
      path.join(tempDir, 'config', 'solana-config.json'),
      JSON.stringify(
        {
          wallet: {
            signingMethod: 'swig',
            publicKey: 'wallet-address',
            authorityPublicKey: 'authority-address',
            feeMode: 'paymaster',
          },
          preferences: {
            rpcUrl: 'https://api.devnet.solana.com',
            defaultSlippage: 50,
          },
          setupComplete: true,
          setupDate: '2026-04-07T00:00:00.000Z',
        },
        null,
        2,
      ),
    );

    await expect(isSolanaConfigured()).resolves.toBe(false);
  });
});
