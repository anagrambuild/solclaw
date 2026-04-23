import fs from 'fs/promises';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { readEnvFile } from './env.js';

describe('readEnvFile', () => {
  let previousCwd: string;
  let tempDir: string;

  beforeEach(async () => {
    previousCwd = process.cwd();
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'solclaw-env-test-'));
    process.chdir(tempDir);
  });

  afterEach(async () => {
    process.chdir(previousCwd);
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  it('merges keys from multiple env files with later files winning', async () => {
    await fs.writeFile(
      path.join(tempDir, '.env'),
      'AXIOM_TOKEN=base-token\nOVERRIDE=from-env\n',
    );
    await fs.writeFile(
      path.join(tempDir, '.env.solana'),
      'SWIG_AUTHORITY_PRIVATE_KEY=secret\nOVERRIDE=from-solana\n',
    );

    const result = readEnvFile(
      ['AXIOM_TOKEN', 'SWIG_AUTHORITY_PRIVATE_KEY', 'OVERRIDE'],
      ['.env', '.env.solana'],
    );

    expect(result).toEqual({
      AXIOM_TOKEN: 'base-token',
      SWIG_AUTHORITY_PRIVATE_KEY: 'secret',
      OVERRIDE: 'from-solana',
    });
  });
});
