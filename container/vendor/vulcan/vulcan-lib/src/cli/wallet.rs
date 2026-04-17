//! Wallet subcommand definitions.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum WalletCommand {
    /// Generate a new Solana keypair, encrypt, and store
    Create {
        /// Name for the new wallet
        #[arg(long)]
        name: String,
    },

    /// Import a wallet from base58 private key, byte array, or Solana CLI JSON file
    Import {
        /// Name for the imported wallet
        #[arg(long)]
        name: String,

        /// Import format
        #[arg(long, value_enum, default_value = "base58")]
        format: ImportFormat,

        /// Source: base58 string, byte array, or file path (depending on --format)
        source: String,
    },

    /// List all stored wallets
    List,

    /// Show wallet details (pubkey, default status)
    Show {
        /// Wallet name
        name: String,
    },

    /// Set a wallet as the default for all commands
    SetDefault {
        /// Wallet name
        name: String,
    },

    /// Remove a wallet from local storage
    Remove {
        /// Wallet name
        name: String,
    },

    /// Export wallet public key (never exports private key)
    Export {
        /// Wallet name
        name: String,
    },

    /// Show SOL and USDC balances for a wallet
    Balance {
        /// Wallet name (defaults to default wallet)
        name: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ImportFormat {
    /// Base58 encoded private key
    Base58,
    /// Byte array (JSON)
    Bytes,
    /// Solana CLI JSON file
    File,
}
