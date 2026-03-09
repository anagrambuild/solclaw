/**
 * Solana setup step
 * Auto-generates a local keypair and configures Solana wallet
 */

import { input, confirm, password } from '@inquirer/prompts';
import fs from 'fs/promises';
import path from 'path';
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';
import chalk from 'chalk';
import * as QRCode from 'qrcode';
import { emitStatus } from './status.js';

/**
 * Parse CLI args into a key-value map.
 * Supports: --network mainnet --slippage 50 --rpc-url <url>
 *           --dflow-key <key> --jupiter-key <key> --breeze-key <key> --helius-key <key>
 */
function parseArgs(args: string[]): Record<string, string> {
  const result: Record<string, string> = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith('--') && i + 1 < args.length && !args[i + 1].startsWith('--')) {
      result[args[i].slice(2)] = args[i + 1];
      i++;
    }
  }
  return result;
}

/**
 * Generate and display a QR code in the terminal for a Solana address
 */
export async function displayWalletQR(address: string): Promise<void> {
  const solanaPayUrl = `solana:${address}`;
  const qrText = await QRCode.toString(solanaPayUrl, { type: 'terminal', small: true });
  console.log(qrText);
  console.log(chalk.cyan(`  ${address}\n`));
}

export async function run(args: string[]): Promise<void> {
  console.log(chalk.cyan.bold('\n🦀 Solana Configuration\n'));
  console.log('Auto-generating wallet keypair.\n');

  emitStatus('SOLANA_SETUP', { STATUS: 'starting' });

  const cliArgs = parseArgs(args);
  const nonInteractive = !!cliArgs.network;

  try {
    // Step 1: Auto-generate keypair
    console.log(chalk.yellow('Step 1: Generating Wallet'));

    const keypair = Keypair.generate();
    const privateKey = bs58.encode(keypair.secretKey);
    const publicKey = keypair.publicKey.toBase58();

    console.log(chalk.green('\n✓ New keypair generated!'));
    console.log(chalk.yellow('\n⚠️  SAVE THIS PRIVATE KEY - You will not see it again!\n'));
    console.log(chalk.white(`Public Key:  ${chalk.cyan(publicKey)}`));
    console.log(chalk.white(`Private Key: ${chalk.cyan(privateKey)}\n`));

    // Step 2: RPC Configuration
    console.log(chalk.yellow('Step 2: RPC Configuration'));

    let rpcUrl: string;

    if (cliArgs.network === 'mainnet' || (!cliArgs.network && nonInteractive)) {
      rpcUrl = 'https://api.breeze.baby/agent/rpc-mainnet-beta';
      console.log(chalk.cyan('Using Mainnet'));
    } else if (cliArgs.network === 'devnet') {
      rpcUrl = 'https://api.devnet.solana.com';
      console.log(chalk.cyan('Using Devnet'));
      console.log(chalk.gray('Get free SOL: solana airdrop 1 ' + publicKey + ' --url devnet'));
    } else if (cliArgs.network === 'testnet') {
      rpcUrl = 'https://api.testnet.solana.com';
      console.log(chalk.cyan('Using Testnet'));
    } else if (cliArgs['rpc-url']) {
      rpcUrl = cliArgs['rpc-url'];
    } else {
      // Default to mainnet
      rpcUrl = 'https://api.breeze.baby/agent/rpc-mainnet-beta';
      console.log(chalk.cyan('Using Mainnet (default)'));
    }

    const defaultSlippage = cliArgs.slippage || '50';

    // Step 3: Optional Protocol API Keys
    console.log(chalk.yellow('\nStep 3: Optional Protocol API Keys'));
    console.log(chalk.gray('These are optional. The agent works without them but some protocols offer better rates or features with an API key.\n'));

    const protocolKeys: Record<string, string> = {};

    if (cliArgs['dflow-key']) {
      protocolKeys.DFLOW_API_KEY = cliArgs['dflow-key'];
    } else if (!nonInteractive) {
      const wantsDflow = await confirm({ message: 'Do you have a DFlow API key?', default: false });
      if (wantsDflow) {
        const key = await password({ message: 'DFlow API key:' });
        if (key) protocolKeys.DFLOW_API_KEY = key;
      }
    }

    if (cliArgs['jupiter-key']) {
      protocolKeys.JUPITER_API_KEY = cliArgs['jupiter-key'];
    } else if (!nonInteractive) {
      const wantsJupiter = await confirm({ message: 'Do you have a Jupiter API key?', default: false });
      if (wantsJupiter) {
        const key = await password({ message: 'Jupiter API key:' });
        if (key) protocolKeys.JUPITER_API_KEY = key;
      }
    }

    if (cliArgs['breeze-key']) {
      protocolKeys.BREEZE_API_KEY = cliArgs['breeze-key'];
    } else if (!nonInteractive) {
      const wantsBreeze = await confirm({ message: 'Do you have a Breeze API key?', default: false });
      if (wantsBreeze) {
        const key = await password({ message: 'Breeze API key:' });
        if (key) protocolKeys.BREEZE_API_KEY = key;
      }
    }

    if (cliArgs['helius-key']) {
      protocolKeys.HELIUS_API_KEY = cliArgs['helius-key'];
    } else if (!nonInteractive) {
      const wantsHelius = await confirm({ message: 'Do you have a Helius API key?', default: false });
      if (wantsHelius) {
        const key = await password({ message: 'Helius API key:' });
        if (key) protocolKeys.HELIUS_API_KEY = key;
      }
    }

    // Build config
    const config: Record<string, any> = {
      wallet: {
        signingMethod: 'standard',
        publicKey,
        privateKey,
      },
      preferences: {
        rpcUrl,
        defaultSlippage: parseInt(defaultSlippage),
      },
      setupComplete: true,
      setupDate: new Date().toISOString(),
    };

    // Save config
    const configPath = path.join(process.cwd(), 'config', 'solana-config.json');
    await fs.mkdir(path.dirname(configPath), { recursive: true });
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));

    // Create .env file
    const envLines = [
      '# SolClaw Solana Configuration',
      '# Generated during setup',
      '',
      `SOLANA_RPC_URL=${rpcUrl}`,
      `SOLANA_SIGNING_METHOD=standard`,
      `SOLANA_PRIVATE_KEY=${privateKey}`,
      '',
    ];

    const envPath = path.join(process.cwd(), '.env.solana');
    await fs.writeFile(envPath, envLines.join('\n'));

    // Append protocol API keys to .env (read by container-runner)
    if (Object.keys(protocolKeys).length > 0) {
      const mainEnvPath = path.join(process.cwd(), '.env');
      let existing = '';
      try {
        existing = await fs.readFile(mainEnvPath, 'utf-8');
      } catch {
        // .env doesn't exist yet, that's fine
      }

      const newLines: string[] = [];
      if (existing && !existing.endsWith('\n')) newLines.push('');
      newLines.push('# Protocol API Keys (added by Solana setup)');
      for (const [key, value] of Object.entries(protocolKeys)) {
        if (existing.includes(`${key}=`)) {
          existing = existing
            .split('\n')
            .filter((line) => !line.startsWith(`${key}=`))
            .join('\n');
        }
        newLines.push(`${key}=${value}`);
      }
      newLines.push('');

      await fs.writeFile(mainEnvPath, existing + newLines.join('\n'));
    }

    // Summary with QR code
    console.log(chalk.green.bold('\n✅ Solana Configuration Complete!\n'));
    console.log(chalk.white('Configuration saved to:'));
    console.log(chalk.cyan(`  ${configPath}`));
    console.log(chalk.cyan(`  ${envPath}\n`));

    console.log(chalk.white(`Network: ${chalk.cyan(rpcUrl)}\n`));

    console.log(chalk.white('Your agent can now:'));
    console.log(chalk.cyan('  • Check wallet balances'));
    console.log(chalk.cyan('  • Get token prices via Jupiter'));
    console.log(chalk.cyan('  • Swap tokens via Jupiter Ultra'));
    console.log(chalk.cyan('  • Transfer SOL and SPL tokens'));
    console.log(chalk.cyan('  • Access DeFi protocols via skills\n'));

    console.log(chalk.white.bold('Wallet Address (scan to fund):'));
    await displayWalletQR(publicKey);

    emitStatus('SOLANA_SETUP', {
      STATUS: 'complete',
      PUBLIC_KEY: publicKey,
      RPC_URL: rpcUrl,
      SIGNING_METHOD: 'standard',
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
