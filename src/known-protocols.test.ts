import { describe, expect, it } from 'vitest';

import { normalizeProtocol } from './known-protocols.js';

describe('normalizeProtocol', () => {
  it('returns exact match for known protocols', () => {
    expect(normalizeProtocol('drift')).toBe('drift');
    expect(normalizeProtocol('jupiter')).toBe('jupiter');
    expect(normalizeProtocol('raydium')).toBe('raydium');
    expect(normalizeProtocol('system-program')).toBe('system-program');
    expect(normalizeProtocol('token-program')).toBe('token-program');
    expect(normalizeProtocol('coingecko')).toBe('coingecko');
    expect(normalizeProtocol('crossmint')).toBe('crossmint');
    expect(normalizeProtocol('glam')).toBe('glam');
    expect(normalizeProtocol('helius')).toBe('helius');
  });

  it('lowercases and trims input', () => {
    expect(normalizeProtocol('DRIFT')).toBe('drift');
    expect(normalizeProtocol('  Jupiter  ')).toBe('jupiter');
    expect(normalizeProtocol('Raydium')).toBe('raydium');
  });

  it('maps legacy aliases', () => {
    expect(normalizeProtocol('system')).toBe('system-program');
    expect(normalizeProtocol('transfer')).toBe('token-program');
    expect(normalizeProtocol('SYSTEM')).toBe('system-program');
    expect(normalizeProtocol('Transfer')).toBe('token-program');
    expect(normalizeProtocol('solana')).toBe('token-program');
    expect(normalizeProtocol('spl-transfer')).toBe('token-program');
  });

  it('extracts known protocol from prefix (drift-perp-long → drift)', () => {
    expect(normalizeProtocol('drift-perp-long')).toBe('drift');
    expect(normalizeProtocol('drift-init')).toBe('drift');
    expect(normalizeProtocol('jupiter-swap')).toBe('jupiter');
    expect(normalizeProtocol('kamino_lending')).toBe('kamino');
    expect(normalizeProtocol('raydium-amm-v4')).toBe('raydium');
  });

  it('extracts known protocol from anywhere in the string (contains match)', () => {
    expect(normalizeProtocol('perp-drift-v2')).toBe('drift');
    expect(normalizeProtocol('my-jupiter-swap')).toBe('jupiter');
    expect(normalizeProtocol('auto-orca-whirlpool')).toBe('orca');
    expect(normalizeProtocol('v2-meteora-dlmm')).toBe('meteora');
  });

  it('passes through unknown protocols lowercased', () => {
    expect(normalizeProtocol('some-new-dex')).toBe('some-new-dex');
    expect(normalizeProtocol('BANANA')).toBe('banana');
    expect(normalizeProtocol('  Unknown_Proto  ')).toBe('unknown_proto');
  });

  it('handles all known protocols from the set', () => {
    const all = [
      'breeze', 'coingecko', 'crossmint', 'dflow', 'drift', 'glam',
      'helius', 'jupiter', 'kamino', 'manifest', 'marginfi', 'metaplex',
      'meteora', 'orca', 'pumpfun', 'raydium', 'swig', 'system-program',
      'token-program',
    ];
    for (const p of all) {
      expect(normalizeProtocol(p)).toBe(p);
    }
  });
});
