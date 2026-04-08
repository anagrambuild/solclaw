import fs from 'fs';
import path from 'path';
import { logger } from './logger.js';

/**
 * Parse one or more env files and return values for the requested keys.
 * Does NOT load anything into process.env — callers decide what to
 * do with the values. This keeps secrets out of the process environment
 * so they don't leak to child processes.
 */
export function readEnvFile(
  keys: string[],
  files: string | string[] = '.env',
): Record<string, string> {
  const result: Record<string, string> = {};
  const wanted = new Set(keys);
  const envFiles = Array.isArray(files) ? files : [files];

  for (const file of envFiles) {
    const envFile = path.join(process.cwd(), file);
    let content: string;
    try {
      content = fs.readFileSync(envFile, 'utf-8');
    } catch (err) {
      logger.debug({ err, envFile }, 'Env file not found, using defaults');
      continue;
    }

    for (const line of content.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const eqIdx = trimmed.indexOf('=');
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      if (!wanted.has(key)) continue;
      let value = trimmed.slice(eqIdx + 1).trim();
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      }
      if (value) result[key] = value;
    }
  }

  return result;
}
