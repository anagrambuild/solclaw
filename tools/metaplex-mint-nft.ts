#!/usr/bin/env npx tsx
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import {
  mplCore,
  create,
  fetchAsset,
} from '@metaplex-foundation/mpl-core';
import {
  generateSigner,
  keypairIdentity,
  publicKey as toPublicKey,
} from '@metaplex-foundation/umi';
import { loadWallet } from './lib/wallet.js';
import * as fs from 'fs';

async function mintNFT() {
  try {
    // Load wallet
    const { keypair: solanaKeypair, rpcUrl } = loadWallet();

    // Create Umi instance
    const umi = createUmi(rpcUrl).use(mplCore());

    // Convert Solana keypair to Umi format
    const umiKeypair = umi.eddsa.createKeypairFromSecretKey(solanaKeypair.secretKey);
    const umiSigner = {
      ...umiKeypair,
      publicKey: toPublicKey(solanaKeypair.publicKey.toBase58())
    };

    umi.use(keypairIdentity(umiSigner));

    // Simple metadata (no upload needed for quick test)
    const metadata = {
      name: 'My First NFT',
      description: 'A simple NFT minted with Metaplex Core',
      image: 'https://arweave.net/placeholder.png',
      attributes: [
        { trait_type: 'Type', value: 'Core NFT' },
        { trait_type: 'Minted', value: new Date().toISOString() }
      ]
    };

    // For now, we'll use a simple JSON string as URI
    // In production, you'd upload this to Arweave/IPFS
    const metadataUri = `data:application/json,${encodeURIComponent(JSON.stringify(metadata))}`;

    // Generate asset signer
    const asset = generateSigner(umi);

    console.error('Minting NFT...');
    console.error(`Asset address: ${asset.publicKey}`);

    // Create the NFT
    const result = await create(umi, {
      asset,
      name: 'My First NFT',
      uri: metadataUri,
    }).sendAndConfirm(umi);

    console.error('NFT minted successfully!');

    // Fetch the created asset
    const fetchedAsset = await fetchAsset(umi, asset.publicKey);

    // Output JSON result
    const output = {
      success: true,
      mint: asset.publicKey.toString(),
      name: fetchedAsset.name,
      uri: fetchedAsset.uri,
      owner: fetchedAsset.owner.toString(),
      signature: result.signature.toString(),
      explorer: `https://solscan.io/token/${asset.publicKey}`,
    };

    console.log(JSON.stringify(output, null, 2));
  } catch (error: any) {
    console.error('Error minting NFT:', error.message);
    console.log(JSON.stringify({
      success: false,
      error: error.message,
      details: error.toString(),
    }, null, 2));
    process.exit(1);
  }
}

mintNFT();
