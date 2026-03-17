/**
 * Canonical protocol names and normalizer.
 * Keeps protocol values in the DB clean and consistent.
 *
 * Agents sometimes log creative names like "drift-perp-long" or "jupiter-swap".
 * This module maps them back to the canonical base protocol.
 */

export const KNOWN_PROTOCOLS = new Set([
  'breeze',
  'coingecko',
  'crossmint',
  'dflow',
  'drift',
  'glam',
  'helius',
  'jupiter',
  'kamino',
  'manifest',
  'marginfi',
  'metaplex',
  'meteora',
  'orca',
  'pumpfun',
  'raydium',
  'swig',
  'system-program',
  'token-program',
] as const);

export type KnownProtocol = typeof KNOWN_PROTOCOLS extends Set<infer T> ? T : never;

/**
 * Normalize a raw protocol string to its canonical form.
 *
 * 1. Lowercases and trims
 * 2. Checks for exact match against known protocols
 * 3. Checks if the string starts with a known protocol name (handles "drift-perp-long" → "drift")
 * 4. Falls back to "unknown" for truly unrecognized values
 */
// Legacy names → canonical names
const ALIASES: Record<string, string> = {
  'system': 'system-program',
  'transfer': 'token-program',
  'solana': 'token-program',
  'spl-transfer': 'token-program',
};

export function normalizeProtocol(raw: string): string {
  const cleaned = raw.toLowerCase().trim();

  if (ALIASES[cleaned]) return ALIASES[cleaned];

  if (KNOWN_PROTOCOLS.has(cleaned as KnownProtocol)) {
    return cleaned;
  }

  // Check if it contains a known protocol name (e.g. "drift-perp-long" → "drift", "perp-drift-v2" → "drift")
  for (const protocol of KNOWN_PROTOCOLS) {
    if (cleaned.includes(protocol)) {
      return protocol;
    }
  }

  return cleaned;
}
