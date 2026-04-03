export const SOLCLAW_DIR = '.solclaw';
export const STATE_FILE = 'state.yaml';
export const BASE_DIR = '.solclaw/base';
export const BACKUP_DIR = '.solclaw/backup';
export const LOCK_FILE = '.solclaw/lock';
export const CUSTOM_DIR = '.solclaw/custom';
export const SKILLS_SCHEMA_VERSION = '0.1.0';

// Top-level paths to include in base snapshot and upstream extraction.
// Add new entries here when new root-level directories/files need tracking.
export const BASE_INCLUDES = [
  'src/',
  'package.json',
  '.env.example',
  'container/',
];
