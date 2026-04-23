/**
 * Solana setup step
 * Configures either a standard local keypair or a Swig smart-wallet authority.
 */

import { input, confirm, password, select } from '@inquirer/prompts';
import fs from 'fs/promises';
import path from 'path';
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';
import chalk from 'chalk';
import * as QRCode from 'qrcode';
import { emitStatus } from './status.js';

const DEFAULT_MAINNET_RPC = 'https://api.breeze.baby/agent/rpc-mainnet-beta';
const DEFAULT_DEVNET_RPC = 'https://api.devnet.solana.com';
const DEFAULT_TESTNET_RPC = 'https://api.testnet.solana.com';

type SigningMethod = 'standard' | 'swig';
type SwigFeeMode = 'paymaster' | 'gas-sponsor' | 'self-funded';

interface SolanaSetupConfig {
  wallet?: {
    signingMethod?: string;
    privateKey?: string;
    publicKey?: string;
    authorityPublicKey?: string;
    swigWalletAddress?: string;
    swigAccountAddress?: string;
    feeMode?: SwigFeeMode;
    swigPaymasterPubkey?: string;
    swigPaymasterNetwork?: 'mainnet' | 'devnet';
    gasSponsorUrl?: string;
  };
  preferences?: {
    rpcUrl?: string;
    defaultSlippage?: number;
  };
  setupComplete?: boolean;
  setupDate?: string;
}

interface StandardWalletSetup {
  signingMethod: 'standard';
  publicKey: string;
  privateKey: string;
  walletIsInjected: boolean;
  walletSource?: string;
}

interface SwigWalletSetup {
  signingMethod: 'swig';
  publicKey: string;
  authorityPublicKey: string;
  authorityPrivateKey: string;
  walletAddress?: string;
  swigAccountAddress?: string;
  feeMode: SwigFeeMode;
  swigPaymasterPubkey?: string;
  swigPaymasterNetwork?: 'mainnet' | 'devnet';
  swigPaymasterApiKey?: string;
  gasSponsorUrl?: string;
  walletIsInjected: boolean;
  walletSource?: string;
}

type WalletSetup = StandardWalletSetup | SwigWalletSetup;

/**
 * Parse CLI args into a key-value map.
 * Supports: --network mainnet --slippage 50 --rpc-url <url>
 *           --signing-method swig --swig-fee-mode paymaster
 *           --dflow-key <key> --jupiter-key <key> --breeze-key <key> --helius-key <key>
 */
function parseArgs(args: string[]): Record<string, string> {
  const result: Record<string, string> = {};
  for (let i = 0; i < args.length; i++) {
    if (
      args[i].startsWith('--') &&
      i + 1 < args.length &&
      !args[i + 1].startsWith('--')
    ) {
      result[args[i].slice(2)] = args[i + 1];
      i++;
    }
  }
  return result;
}

async function readJsonIfExists<T>(filePath: string): Promise<T | null> {
  try {
    const raw = await fs.readFile(filePath, 'utf-8');
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

async function readSimpleEnvFile(
  filePath: string,
): Promise<Record<string, string>> {
  const result: Record<string, string> = {};

  try {
    const content = await fs.readFile(filePath, 'utf-8');
    for (const line of content.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const eqIdx = trimmed.indexOf('=');
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      let value = trimmed.slice(eqIdx + 1).trim();
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      }
      if (key && value) result[key] = value;
    }
  } catch {
    // Missing env file is normal during first-time setup.
  }

  return result;
}

function derivePublicKey(privateKey: string): string {
  const secretKey = bs58.decode(privateKey);
  return Keypair.fromSecretKey(secretKey).publicKey.toBase58();
}

function resolvePaymasterNetwork(
  raw: string | undefined,
): 'mainnet' | 'devnet' | undefined {
  if (!raw) return undefined;
  if (raw === 'mainnet' || raw === 'devnet') return raw;
  throw new Error(
    `Invalid Swig paymaster network "${raw}". Use "mainnet" or "devnet".`,
  );
}

/**
 * Generate and display a QR code in the terminal for a Solana address
 */
export async function displayWalletQR(address: string): Promise<void> {
  const solanaPayUrl = `solana:${address}`;
  const qrText = await QRCode.toString(solanaPayUrl, {
    type: 'terminal',
    small: true,
  });
  console.log(qrText);
  console.log(chalk.cyan(`  ${address}\n`));
}

async function resolveSigningMethod(
  cliArgs: Record<string, string>,
  nonInteractive: boolean,
): Promise<SigningMethod> {
  const cliMethod = cliArgs['signing-method'];
  if (cliMethod) {
    if (cliMethod === 'standard' || cliMethod === 'swig') return cliMethod;
    throw new Error(
      `Invalid signing method "${cliMethod}". Use "standard" or "swig".`,
    );
  }

  if (nonInteractive) return 'standard';

  return select<SigningMethod>({
    message: 'Select signing method:',
    choices: [
      {
        name: 'Standard local keypair',
        value: 'standard',
        description: 'Direct signing with a local Solana keypair',
      },
      {
        name: 'Swig smart wallet',
        value: 'swig',
        description:
          'Use a Swig smart wallet with MCP tools for wallet creation and transactions',
      },
    ],
    default: 'standard',
  });
}

async function resolveStandardWallet(
  existingConfig: SolanaSetupConfig | null,
): Promise<StandardWalletSetup> {
  let publicKey: string;
  let privateKey: string;
  let walletIsInjected = false;
  let walletSource: string | undefined;

  const injectedKey = process.env.SOLCLAW_WALLET_PRIVATE_KEY;
  if (injectedKey) {
    try {
      publicKey = derivePublicKey(injectedKey);
      privateKey = injectedKey;
      walletIsInjected = true;
      walletSource = 'Dashboard-assigned';
      console.log(chalk.green('\n✓ Using dashboard-assigned wallet\n'));
    } catch {
      console.log(
        chalk.yellow(
          '⚠ Invalid SOLCLAW_WALLET_PRIVATE_KEY, generating new keypair instead\n',
        ),
      );
      const kp = Keypair.generate();
      privateKey = bs58.encode(kp.secretKey);
      publicKey = kp.publicKey.toBase58();
    }
  } else if (
    existingConfig?.wallet?.signingMethod === 'standard' &&
    existingConfig.wallet.privateKey &&
    existingConfig.wallet.publicKey
  ) {
    publicKey = existingConfig.wallet.publicKey;
    privateKey = existingConfig.wallet.privateKey;
    walletIsInjected = true;
    walletSource = 'Existing config';
    console.log(chalk.green('\n✓ Existing wallet found, reusing\n'));
  } else {
    console.log('Auto-generating wallet keypair.\n');
    const kp = Keypair.generate();
    privateKey = bs58.encode(kp.secretKey);
    publicKey = kp.publicKey.toBase58();
  }

  console.log(chalk.white.bold('  Public Key'));
  console.log(chalk.cyan(`  ${publicKey}\n`));
  console.log(chalk.white.bold('  Private Key'));
  console.log(chalk.yellow(`  ${privateKey}\n`));

  if (!walletIsInjected) {
    console.log(chalk.green('\n✓ New keypair generated!\n'));
    console.log(
      chalk.red.bold(
        '╔══════════════════════════════════════════════════════════════════╗',
      ),
    );
    console.log(
      chalk.red.bold(
        '║                                                                  ║',
      ),
    );
    console.log(
      chalk.red.bold(
        '║   SAVE YOUR PRIVATE KEY NOW — YOU WILL NOT SEE IT AGAIN          ║',
      ),
    );
    console.log(
      chalk.red.bold(
        '║                                                                  ║',
      ),
    );
    console.log(
      chalk.red.bold(
        '╚══════════════════════════════════════════════════════════════════╝',
      ),
    );
    console.log('');
    console.log(
      chalk.gray(
        '  Copy your private key and store it somewhere safe (password',
      ),
    );
    console.log(
      chalk.gray(
        '  manager, encrypted note, etc). This is the ONLY time it will',
      ),
    );
    console.log(
      chalk.gray('  be displayed. If you lose it, the wallet and any funds in'),
    );
    console.log(chalk.gray('  it are gone forever.'));
    console.log('');
  }

  return {
    signingMethod: 'standard',
    publicKey,
    privateKey,
    walletIsInjected,
    walletSource,
  };
}

async function resolveSwigWallet(
  cliArgs: Record<string, string>,
  nonInteractive: boolean,
  existingConfig: SolanaSetupConfig | null,
  existingSolanaEnv: Record<string, string>,
): Promise<SwigWalletSetup> {
  const existingWallet =
    existingConfig?.wallet?.signingMethod === 'swig'
      ? existingConfig.wallet
      : undefined;

  let authorityPrivateKey =
    cliArgs['swig-authority-private-key'] ||
    process.env.SWIG_AUTHORITY_PRIVATE_KEY ||
    existingSolanaEnv.SWIG_AUTHORITY_PRIVATE_KEY;
  let walletIsInjected = false;
  let walletSource: string | undefined;

  if (authorityPrivateKey) {
    walletIsInjected = true;
    walletSource = process.env.SWIG_AUTHORITY_PRIVATE_KEY
      ? 'Environment'
      : existingSolanaEnv.SWIG_AUTHORITY_PRIVATE_KEY
        ? 'Existing config'
        : 'CLI argument';
    console.log(chalk.green('\n✓ Reusing existing Swig authority key\n'));
  } else if (!nonInteractive) {
    const authorityMode = await select<'generate' | 'paste'>({
      message: 'How should SolClaw authenticate to Swig?',
      choices: [
        {
          name: 'Generate a new authority keypair',
          value: 'generate',
          description: 'Recommended when setting up a new Swig wallet',
        },
        {
          name: 'Paste an existing authority private key',
          value: 'paste',
          description: 'Reuse an authority you already manage',
        },
      ],
      default: 'generate',
    });

    if (authorityMode === 'paste') {
      authorityPrivateKey = await password({
        message: 'Paste Swig authority private key (base58):',
        validate: (value) => {
          try {
            const decoded = bs58.decode(value);
            return decoded.length === 64
              ? true
              : 'Invalid key length (expected 64 bytes)';
          } catch {
            return 'Invalid base58 format';
          }
        },
      });
    }
  }

  if (!authorityPrivateKey) {
    const kp = Keypair.generate();
    authorityPrivateKey = bs58.encode(kp.secretKey);
    console.log(chalk.green('\n✓ Generated a new Swig authority keypair\n'));
  }

  const authorityPublicKey = derivePublicKey(authorityPrivateKey);
  console.log(chalk.white.bold('  Authority Public Key'));
  console.log(chalk.cyan(`  ${authorityPublicKey}\n`));

  let walletAddress =
    cliArgs['swig-wallet-address'] || existingWallet?.swigWalletAddress;
  let swigAccountAddress =
    cliArgs['swig-account-address'] || existingWallet?.swigAccountAddress;

  if (!walletAddress && !nonInteractive) {
    const hasExistingWallet = await confirm({
      message: 'Do you already have a Swig wallet to reuse?',
      default: false,
    });

    if (hasExistingWallet) {
      walletAddress = await input({
        message: 'Swig wallet address:',
        validate: (value) =>
          value.trim() ? true : 'Wallet address is required',
      });
      swigAccountAddress = await input({
        message: 'Swig account address:',
        validate: (value) =>
          value.trim() ? true : 'Swig account address is required',
      });
    }
  }

  const feeMode =
    (cliArgs['swig-fee-mode'] as SwigFeeMode | undefined) ||
    existingWallet?.feeMode ||
    (nonInteractive
      ? 'self-funded'
      : await select<SwigFeeMode>({
          message: 'How should Swig transaction fees be handled?',
          choices: [
            {
              name: 'Swig Paymaster',
              value: 'paymaster',
              description: 'Recommended for production gasless transactions',
            },
            {
              name: 'Custom gas sponsor',
              value: 'gas-sponsor',
              description: 'Use your own sponsorship server',
            },
            {
              name: 'Self-funded authority',
              value: 'self-funded',
              description: 'Authority keypair pays fees directly',
            },
          ],
          default: existingWallet?.feeMode || 'self-funded',
        }));

  if (
    feeMode !== 'paymaster' &&
    feeMode !== 'gas-sponsor' &&
    feeMode !== 'self-funded'
  ) {
    throw new Error(
      `Invalid Swig fee mode "${feeMode}". Use paymaster, gas-sponsor, or self-funded.`,
    );
  }

  let swigPaymasterPubkey = existingWallet?.swigPaymasterPubkey;
  let swigPaymasterNetwork = existingWallet?.swigPaymasterNetwork;
  let gasSponsorUrl = existingWallet?.gasSponsorUrl;
  let swigPaymasterApiKey = process.env.SWIG_PAYMASTER_API_KEY;

  if (feeMode === 'paymaster') {
    swigPaymasterPubkey =
      cliArgs['swig-paymaster-pubkey'] || swigPaymasterPubkey;
    swigPaymasterNetwork =
      resolvePaymasterNetwork(cliArgs['swig-paymaster-network']) ||
      swigPaymasterNetwork;
    swigPaymasterApiKey =
      cliArgs['swig-paymaster-api-key'] ||
      swigPaymasterApiKey ||
      existingSolanaEnv.SWIG_PAYMASTER_API_KEY;

    if (!swigPaymasterPubkey) {
      if (nonInteractive) {
        throw new Error(
          'Swig paymaster mode requires --swig-paymaster-pubkey.',
        );
      }
      swigPaymasterPubkey = await input({
        message: 'Swig paymaster public key:',
        validate: (value) =>
          value.trim() ? true : 'Swig paymaster public key is required',
      });
    }

    if (!swigPaymasterNetwork) {
      if (nonInteractive) {
        swigPaymasterNetwork = 'mainnet';
      } else {
        swigPaymasterNetwork = await select<'mainnet' | 'devnet'>({
          message: 'Swig paymaster network:',
          choices: [
            { name: 'Mainnet', value: 'mainnet' },
            { name: 'Devnet', value: 'devnet' },
          ],
          default: 'mainnet',
        });
      }
    }

    if (!swigPaymasterApiKey) {
      if (nonInteractive) {
        throw new Error(
          'Swig paymaster mode requires --swig-paymaster-api-key or SWIG_PAYMASTER_API_KEY.',
        );
      }
      swigPaymasterApiKey = await password({
        message: 'Swig paymaster API key:',
        validate: (value) =>
          value.trim() ? true : 'Swig paymaster API key is required',
      });
    }
  }

  if (feeMode === 'gas-sponsor') {
    gasSponsorUrl = cliArgs['gas-sponsor-url'] || gasSponsorUrl;

    if (!gasSponsorUrl) {
      if (nonInteractive) {
        throw new Error('Swig gas sponsor mode requires --gas-sponsor-url.');
      }
      gasSponsorUrl = await input({
        message: 'Custom gas sponsor URL:',
        validate: (value) =>
          value.trim() ? true : 'Gas sponsor URL is required',
      });
    }
  }

  const publicKey = walletAddress || authorityPublicKey;

  return {
    signingMethod: 'swig',
    publicKey,
    authorityPublicKey,
    authorityPrivateKey,
    walletAddress: walletAddress || undefined,
    swigAccountAddress: swigAccountAddress || undefined,
    feeMode,
    swigPaymasterPubkey:
      feeMode === 'paymaster' ? swigPaymasterPubkey : undefined,
    swigPaymasterNetwork:
      feeMode === 'paymaster' ? swigPaymasterNetwork : undefined,
    swigPaymasterApiKey:
      feeMode === 'paymaster' ? swigPaymasterApiKey : undefined,
    gasSponsorUrl: feeMode === 'gas-sponsor' ? gasSponsorUrl : undefined,
    walletIsInjected,
    walletSource,
  };
}

function resolveRpcUrl(
  cliArgs: Record<string, string>,
  nonInteractive: boolean,
  walletSetup: WalletSetup,
  existingConfig: SolanaSetupConfig | null,
): string {
  if (cliArgs['rpc-url']) {
    return cliArgs['rpc-url'];
  }

  if (cliArgs.network === 'mainnet' || (!cliArgs.network && nonInteractive)) {
    console.log(chalk.cyan('Using Mainnet'));
    return DEFAULT_MAINNET_RPC;
  }

  if (cliArgs.network === 'devnet') {
    console.log(chalk.cyan('Using Devnet'));
    const fundingAddress =
      walletSetup.signingMethod === 'swig'
        ? walletSetup.authorityPublicKey
        : walletSetup.publicKey;
    console.log(
      chalk.gray(
        `Get free SOL: solana airdrop 1 ${fundingAddress} --url devnet`,
      ),
    );
    return DEFAULT_DEVNET_RPC;
  }

  if (cliArgs.network === 'testnet') {
    console.log(chalk.cyan('Using Testnet'));
    return DEFAULT_TESTNET_RPC;
  }

  if (existingConfig?.preferences?.rpcUrl) {
    console.log(chalk.cyan('Using existing RPC configuration'));
    return existingConfig.preferences.rpcUrl;
  }

  console.log(chalk.cyan('Using Mainnet (default)'));
  return DEFAULT_MAINNET_RPC;
}

async function writeProtocolApiKeys(
  protocolKeys: Record<string, string>,
): Promise<void> {
  if (Object.keys(protocolKeys).length === 0) return;

  const mainEnvPath = path.join(process.cwd(), '.env');
  let existing = '';
  try {
    existing = await fs.readFile(mainEnvPath, 'utf-8');
  } catch {
    // .env doesn't exist yet, that's fine.
  }

  const filteredLines = existing
    .split('\n')
    .filter((line) =>
      line.trim()
        ? !Object.keys(protocolKeys).some((key) => line.startsWith(`${key}=`))
        : true,
    );

  const nextLines = filteredLines.filter(
    (_, index, arr) => !(index === arr.length - 1 && arr[index] === ''),
  );

  if (nextLines.length > 0) nextLines.push('');
  nextLines.push('# Protocol API Keys (added by Solana setup)');
  for (const [key, value] of Object.entries(protocolKeys)) {
    nextLines.push(`${key}=${value}`);
  }
  nextLines.push('');

  await fs.writeFile(mainEnvPath, nextLines.join('\n'));
}

function buildSolanaEnvLines(
  rpcUrl: string,
  walletSetup: WalletSetup,
): string[] {
  const lines = [
    '# SolClaw Solana Configuration',
    '# Generated during setup',
    '',
    `SOLANA_RPC_URL=${rpcUrl}`,
    `SOLANA_SIGNING_METHOD=${walletSetup.signingMethod}`,
  ];

  if (walletSetup.signingMethod === 'standard') {
    lines.push(`SOLANA_PRIVATE_KEY=${walletSetup.privateKey}`);
  } else {
    lines.push(`SWIG_AUTHORITY_PRIVATE_KEY=${walletSetup.authorityPrivateKey}`);
    if (walletSetup.feeMode === 'paymaster') {
      lines.push(
        `SWIG_PAYMASTER_PUBKEY=${walletSetup.swigPaymasterPubkey!}`,
        `SWIG_PAYMASTER_NETWORK=${walletSetup.swigPaymasterNetwork!}`,
      );
      if (walletSetup.swigPaymasterApiKey) {
        lines.push(`SWIG_PAYMASTER_API_KEY=${walletSetup.swigPaymasterApiKey}`);
      }
    }
    if (walletSetup.feeMode === 'gas-sponsor' && walletSetup.gasSponsorUrl) {
      lines.push(`GAS_SPONSOR_URL=${walletSetup.gasSponsorUrl}`);
    }
  }

  lines.push('');
  return lines;
}

export async function run(args: string[]): Promise<void> {
  console.log(chalk.cyan.bold('\n🦀 Solana Configuration\n'));

  emitStatus('SOLANA_SETUP', { STATUS: 'starting' });

  const cliArgs = parseArgs(args);
  const nonInteractive = !!cliArgs.network;
  const configPath = path.join(process.cwd(), 'config', 'solana-config.json');
  const envPath = path.join(process.cwd(), '.env.solana');
  const existingConfig = await readJsonIfExists<SolanaSetupConfig>(configPath);
  const existingSolanaEnv = await readSimpleEnvFile(envPath);

  try {
    console.log(chalk.yellow('Step 1: Wallet'));

    const signingMethod = await resolveSigningMethod(cliArgs, nonInteractive);
    const walletSetup =
      signingMethod === 'swig'
        ? await resolveSwigWallet(
            cliArgs,
            nonInteractive,
            existingConfig,
            existingSolanaEnv,
          )
        : await resolveStandardWallet(existingConfig);

    console.log(chalk.yellow('Step 2: RPC Configuration'));
    const rpcUrl = resolveRpcUrl(
      cliArgs,
      nonInteractive,
      walletSetup,
      existingConfig,
    );
    const defaultSlippage =
      cliArgs.slippage ||
      String(existingConfig?.preferences?.defaultSlippage || 50);

    console.log(chalk.yellow('\nStep 3: Optional Protocol API Keys'));
    console.log(
      chalk.gray(
        'These are optional. The agent works without them but some protocols offer better rates or features with an API key.\n',
      ),
    );

    const protocolKeys: Record<string, string> = {};

    if (cliArgs['dflow-key']) {
      protocolKeys.DFLOW_API_KEY = cliArgs['dflow-key'];
    } else if (!nonInteractive) {
      const wantsDflow = await confirm({
        message: 'Do you have a DFlow API key?',
        default: false,
      });
      if (wantsDflow) {
        const key = await password({ message: 'DFlow API key:' });
        if (key) protocolKeys.DFLOW_API_KEY = key;
      }
    }

    if (cliArgs['jupiter-key']) {
      protocolKeys.JUPITER_API_KEY = cliArgs['jupiter-key'];
    } else if (!nonInteractive) {
      const wantsJupiter = await confirm({
        message: 'Do you have a Jupiter API key?',
        default: false,
      });
      if (wantsJupiter) {
        const key = await password({ message: 'Jupiter API key:' });
        if (key) protocolKeys.JUPITER_API_KEY = key;
      }
    }

    if (cliArgs['breeze-key']) {
      protocolKeys.BREEZE_API_KEY = cliArgs['breeze-key'];
    } else if (!nonInteractive) {
      const wantsBreeze = await confirm({
        message: 'Do you have a Breeze API key?',
        default: false,
      });
      if (wantsBreeze) {
        const key = await password({ message: 'Breeze API key:' });
        if (key) protocolKeys.BREEZE_API_KEY = key;
      }
    }

    if (cliArgs['helius-key']) {
      protocolKeys.HELIUS_API_KEY = cliArgs['helius-key'];
    } else if (!nonInteractive) {
      const wantsHelius = await confirm({
        message: 'Do you have a Helius API key?',
        default: false,
      });
      if (wantsHelius) {
        const key = await password({ message: 'Helius API key:' });
        if (key) protocolKeys.HELIUS_API_KEY = key;
      }
    }

    const config: SolanaSetupConfig = {
      wallet:
        walletSetup.signingMethod === 'swig'
          ? {
              signingMethod: 'swig',
              publicKey: walletSetup.publicKey,
              authorityPublicKey: walletSetup.authorityPublicKey,
              swigWalletAddress: walletSetup.walletAddress,
              swigAccountAddress: walletSetup.swigAccountAddress,
              feeMode: walletSetup.feeMode,
              swigPaymasterPubkey: walletSetup.swigPaymasterPubkey,
              swigPaymasterNetwork: walletSetup.swigPaymasterNetwork,
              gasSponsorUrl: walletSetup.gasSponsorUrl,
            }
          : {
              signingMethod: 'standard',
              publicKey: walletSetup.publicKey,
              privateKey: walletSetup.privateKey,
            },
      preferences: {
        rpcUrl,
        defaultSlippage: parseInt(defaultSlippage, 10),
      },
      setupComplete: true,
      setupDate: new Date().toISOString(),
    };

    await fs.mkdir(path.dirname(configPath), { recursive: true });
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    await fs.writeFile(
      envPath,
      buildSolanaEnvLines(rpcUrl, walletSetup).join('\n'),
    );
    await writeProtocolApiKeys(protocolKeys);

    console.log(chalk.green.bold('\n✅ Solana Configuration Complete!\n'));

    console.log(chalk.white.bold('  Wallet Summary'));
    console.log(
      chalk.white(`  Address:         ${chalk.cyan(walletSetup.publicKey)}`),
    );
    console.log(
      chalk.white(
        `  Signing Method:  ${chalk.cyan(
          walletSetup.signingMethod === 'swig'
            ? 'Swig smart wallet'
            : 'Standard (local keypair)',
        )}`,
      ),
    );
    console.log(chalk.white(`  Network:         ${chalk.cyan(rpcUrl)}`));
    if (walletSetup.walletIsInjected && walletSetup.walletSource) {
      console.log(
        chalk.white(
          `  Source:          ${chalk.cyan(walletSetup.walletSource)}`,
        ),
      );
    }
    if (walletSetup.signingMethod === 'swig') {
      console.log(
        chalk.white(
          `  Authority:       ${chalk.cyan(walletSetup.authorityPublicKey)}`,
        ),
      );
      console.log(
        chalk.white(
          `  Swig Wallet:     ${chalk.cyan(
            walletSetup.walletAddress || 'Not created yet',
          )}`,
        ),
      );
      console.log(
        chalk.white(
          `  Swig Account:    ${chalk.cyan(
            walletSetup.swigAccountAddress || 'Not created yet',
          )}`,
        ),
      );
      console.log(
        chalk.white(`  Fee Mode:        ${chalk.cyan(walletSetup.feeMode)}`),
      );
      if (walletSetup.feeMode === 'paymaster') {
        console.log(
          chalk.white(
            `  Paymaster Key:   ${chalk.cyan(walletSetup.swigPaymasterPubkey!)}`,
          ),
        );
        console.log(
          chalk.white(
            `  Paymaster Net:   ${chalk.cyan(walletSetup.swigPaymasterNetwork!)}`,
          ),
        );
      }
      if (walletSetup.feeMode === 'gas-sponsor' && walletSetup.gasSponsorUrl) {
        console.log(
          chalk.white(
            `  Gas Sponsor:     ${chalk.cyan(walletSetup.gasSponsorUrl)}`,
          ),
        );
      }
    }
    console.log('');

    console.log(chalk.white.bold('  Config Files'));
    console.log(chalk.gray(`  ${configPath}`));
    console.log(chalk.gray(`  ${envPath}`));
    console.log('');

    console.log(chalk.white.bold('  Capabilities'));
    if (walletSetup.signingMethod === 'swig') {
      console.log(chalk.cyan('  • Create and fetch Swig wallets via MCP'));
      console.log(chalk.cyan('  • Add, update, and remove Swig authorities'));
      console.log(chalk.cyan('  • Transfer SOL from the Swig wallet'));
      console.log(chalk.cyan('  • Execute custom transactions through Swig'));
      console.log(
        chalk.cyan(
          '  • Reuse the configured RPC and fee strategy automatically',
        ),
      );
    } else {
      console.log(chalk.cyan('  • Check wallet balances'));
      console.log(chalk.cyan('  • Get token prices via Jupiter'));
      console.log(chalk.cyan('  • Swap tokens via Jupiter Ultra'));
      console.log(chalk.cyan('  • Transfer SOL and SPL tokens'));
      console.log(chalk.cyan('  • Access DeFi protocols via skills'));
    }
    console.log('');

    if (walletSetup.signingMethod === 'swig') {
      if (walletSetup.feeMode === 'self-funded') {
        console.log(
          chalk.white.bold(
            '  Fund Your Swig Authority (scan QR or send SOL to address):',
          ),
        );
        await displayWalletQR(walletSetup.authorityPublicKey);
        console.log(
          chalk.yellow(
            '  Send a small amount of SOL to the authority before creating or using the Swig wallet.\n',
          ),
        );
      } else if (walletSetup.walletAddress) {
        console.log(chalk.white.bold('  Current Swig Wallet Address:'));
        await displayWalletQR(walletSetup.walletAddress);
      } else {
        console.log(
          chalk.yellow(
            '  No Swig wallet address is stored yet. Ask SolClaw to create one with the Swig MCP tools.\n',
          ),
        );
      }
    } else {
      console.log(
        chalk.white.bold(
          '  Fund Your Wallet (scan QR or send SOL to address):',
        ),
      );
      await displayWalletQR(walletSetup.publicKey);
      console.log(
        chalk.yellow('  Send SOL to this address to start trading.\n'),
      );
    }

    emitStatus('SOLANA_SETUP', {
      STATUS: 'complete',
      PUBLIC_KEY: walletSetup.publicKey,
      RPC_URL: rpcUrl,
      SIGNING_METHOD: walletSetup.signingMethod,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(chalk.red('\n❌ Solana setup failed:'), message);
    emitStatus('SOLANA_SETUP', {
      STATUS: 'failed',
      ERROR: message,
    });
    throw error;
  }
}
