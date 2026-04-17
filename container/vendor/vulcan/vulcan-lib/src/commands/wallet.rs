//! Wallet command execution.

use crate::cli::wallet::{ImportFormat, WalletCommand};
use crate::context::AppContext;
use crate::error::VulcanError;
use crate::output::{render_success, TableRenderable};
use crate::wallet::Wallet;
use crate::wallet::WalletFile;
use serde::Serialize;
use solana_pubkey::Pubkey;

/// Derive the associated token account address (no external dependency needed).
fn spl_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let spl_token_program =
        Pubkey::try_from("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::try_from("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();
    let seeds = &[wallet.as_ref(), spl_token_program.as_ref(), mint.as_ref()];
    Pubkey::find_program_address(seeds, &ata_program).0
}

#[derive(Debug, Serialize)]
pub struct WalletInfo {
    pub name: String,
    pub public_key: String,
    pub is_default: bool,
}

impl TableRenderable for WalletInfo {
    fn render_table(&self) {
        crate::output::table::render_table(
            &["Name", "Public Key", "Default"],
            vec![vec![
                self.name.clone(),
                self.public_key.clone(),
                if self.is_default {
                    "yes".into()
                } else {
                    "no".into()
                },
            ]],
        );
    }
}

#[derive(Debug, Serialize)]
pub struct WalletList {
    pub wallets: Vec<WalletInfo>,
}

impl TableRenderable for WalletList {
    fn render_table(&self) {
        if self.wallets.is_empty() {
            println!("No wallets found. Create one with: vulcan wallet create --name <NAME>");
            return;
        }
        let rows: Vec<Vec<String>> = self
            .wallets
            .iter()
            .map(|w| {
                vec![
                    w.name.clone(),
                    w.public_key.clone(),
                    if w.is_default {
                        "yes".into()
                    } else {
                        "no".into()
                    },
                ]
            })
            .collect();
        crate::output::table::render_table(&["Name", "Public Key", "Default"], rows);
    }
}

#[derive(Debug, Serialize)]
pub struct WalletCreated {
    pub name: String,
    pub public_key: String,
}

impl TableRenderable for WalletCreated {
    fn render_table(&self) {
        println!("Wallet '{}' created successfully.", self.name);
        println!("Public key: {}", self.public_key);
    }
}

#[derive(Debug, Serialize)]
pub struct WalletRemoved {
    pub name: String,
}

impl TableRenderable for WalletRemoved {
    fn render_table(&self) {
        println!("Wallet '{}' removed.", self.name);
    }
}

#[derive(Debug, Serialize)]
pub struct WalletExport {
    pub name: String,
    pub public_key: String,
}

impl TableRenderable for WalletExport {
    fn render_table(&self) {
        println!("{}", self.public_key);
    }
}

#[derive(Debug, Serialize)]
pub struct WalletBalance {
    pub name: String,
    pub address: String,
    pub sol: f64,
    pub usdc: f64,
}

impl TableRenderable for WalletBalance {
    fn render_table(&self) {
        println!("Wallet '{}'", self.name);
        println!("  Address: {}", self.address);
        println!("  SOL:     {:.9} SOL", self.sol);
        println!("  USDC:    {:.6} USDC", self.usdc);
    }
}

#[derive(Debug, Serialize)]
pub struct DefaultSet {
    pub name: String,
}

impl TableRenderable for DefaultSet {
    fn render_table(&self) {
        println!("Default wallet set to '{}'.", self.name);
    }
}

pub async fn execute(ctx: &AppContext, cmd: WalletCommand) -> Result<(), VulcanError> {
    match cmd {
        WalletCommand::Create { name } => {
            if ctx.wallet_store.exists(&name) {
                return Err(VulcanError::validation(
                    "WALLET_EXISTS",
                    format!("Wallet '{}' already exists", name),
                ));
            }

            let password = crate::commands::trade::prompt_password()?;

            let wallet = Wallet::generate()
                .map_err(|e| VulcanError::internal("KEYGEN_FAILED", e.to_string()))?;

            let encrypted = wallet
                .encrypt(&password)
                .map_err(|e| VulcanError::internal("ENCRYPT_FAILED", e.to_string()))?;

            let wallet_file = WalletFile {
                name: name.clone(),
                public_key: wallet.public_key.clone(),
                encrypted,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            ctx.wallet_store.save(&wallet_file).map_err(|e| {
                VulcanError::new(
                    crate::error::ErrorCategory::Io,
                    "SAVE_FAILED",
                    e.to_string(),
                )
            })?;

            let result = WalletCreated {
                name,
                public_key: wallet.public_key.clone(),
            };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::Import {
            name,
            format,
            source,
        } => {
            if ctx.wallet_store.exists(&name) {
                return Err(VulcanError::validation(
                    "WALLET_EXISTS",
                    format!("Wallet '{}' already exists", name),
                ));
            }

            let wallet = match format {
                ImportFormat::Base58 => Wallet::from_base58(&source),
                ImportFormat::Bytes => {
                    let bytes: Vec<u8> = serde_json::from_str(&source).map_err(|e| {
                        VulcanError::validation(
                            "INVALID_BYTES",
                            format!("Invalid byte array: {}", e),
                        )
                    })?;
                    Wallet::from_bytes(&bytes)
                }
                ImportFormat::File => Wallet::from_file(std::path::Path::new(&source)),
            }
            .map_err(|e| VulcanError::validation("IMPORT_FAILED", e.to_string()))?;

            let password = crate::commands::trade::prompt_password()?;

            let encrypted = wallet
                .encrypt(&password)
                .map_err(|e| VulcanError::internal("ENCRYPT_FAILED", e.to_string()))?;

            let wallet_file = WalletFile {
                name: name.clone(),
                public_key: wallet.public_key.clone(),
                encrypted,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            ctx.wallet_store.save(&wallet_file).map_err(|e| {
                VulcanError::new(
                    crate::error::ErrorCategory::Io,
                    "SAVE_FAILED",
                    e.to_string(),
                )
            })?;

            let result = WalletCreated {
                name,
                public_key: wallet.public_key.clone(),
            };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::List => {
            let names = ctx.wallet_store.list().map_err(|e| {
                VulcanError::new(
                    crate::error::ErrorCategory::Io,
                    "LIST_FAILED",
                    e.to_string(),
                )
            })?;

            let default_name = ctx.wallet_store.default_wallet().ok().flatten();

            let wallets: Vec<WalletInfo> = names
                .into_iter()
                .map(|name| {
                    let public_key = ctx
                        .wallet_store
                        .load(&name)
                        .map(|f| f.public_key)
                        .unwrap_or_else(|_| "???".into());
                    let is_default = default_name.as_deref() == Some(&name);
                    WalletInfo {
                        name,
                        public_key,
                        is_default,
                    }
                })
                .collect();

            let result = WalletList { wallets };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::Show { name } => {
            let wallet_file = ctx
                .wallet_store
                .load(&name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;
            let default_name = ctx.wallet_store.default_wallet().ok().flatten();
            let is_default = default_name.as_deref() == Some(name.as_str());

            let result = WalletInfo {
                name,
                public_key: wallet_file.public_key,
                is_default,
            };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::SetDefault { name } => {
            ctx.wallet_store
                .set_default(&name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

            let result = DefaultSet { name };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::Remove { name } => {
            if !ctx.yes {
                // TODO: interactive confirmation prompt
                eprintln!("Use --yes to confirm removal of wallet '{}'", name);
                return Err(VulcanError::validation(
                    "CONFIRMATION_REQUIRED",
                    "Pass --yes to confirm wallet removal",
                ));
            }

            ctx.wallet_store
                .remove(&name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

            let result = WalletRemoved { name };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::Export { name } => {
            let wallet_file = ctx
                .wallet_store
                .load(&name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

            let result = WalletExport {
                name,
                public_key: wallet_file.public_key,
            };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        WalletCommand::Balance { name } => {
            let wallet_name = match name {
                Some(n) => n,
                None => ctx
                    .wallet_store
                    .default_wallet()
                    .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
                    .ok_or_else(|| {
                        VulcanError::config(
                            "NO_DEFAULT_WALLET",
                            "No default wallet set. Use 'vulcan wallet set-default <NAME>'",
                        )
                    })?,
            };

            let wallet_file = ctx
                .wallet_store
                .load(&wallet_name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

            let pubkey = solana_pubkey::Pubkey::try_from(wallet_file.public_key.as_str())
                .map_err(|e| VulcanError::validation("INVALID_PUBKEY", e.to_string()))?;

            let rpc_client =
                solana_rpc_client::rpc_client::RpcClient::new(ctx.config.network.rpc_url.clone());

            // SOL balance
            let sol_lamports = rpc_client
                .get_balance(&pubkey)
                .map_err(|e| VulcanError::network("RPC_BALANCE_FAILED", e.to_string()))?;
            let sol = sol_lamports as f64 / 1_000_000_000.0;

            // USDC balance — derive the associated token account
            let usdc_mint =
                solana_pubkey::Pubkey::try_from("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
                    .unwrap();
            let ata = spl_associated_token_address(&pubkey, &usdc_mint);

            let usdc = match rpc_client.get_token_account_balance(&ata) {
                Ok(balance) => balance.ui_amount.unwrap_or(0.0),
                Err(_) => 0.0, // No token account = 0 USDC
            };

            let result = WalletBalance {
                name: wallet_name,
                address: wallet_file.public_key,
                sol,
                usdc,
            };
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }
    }
}

// ── Inner functions for MCP ────────────────────────────────────────────

pub fn execute_list_inner(ctx: &AppContext) -> Result<WalletList, VulcanError> {
    let names = ctx.wallet_store.list().map_err(|e| {
        VulcanError::new(
            crate::error::ErrorCategory::Io,
            "LIST_FAILED",
            e.to_string(),
        )
    })?;

    let default_name = ctx.wallet_store.default_wallet().ok().flatten();

    let wallets: Vec<WalletInfo> = names
        .into_iter()
        .map(|name| {
            let public_key = ctx
                .wallet_store
                .load(&name)
                .map(|f| f.public_key)
                .unwrap_or_else(|_| "???".into());
            let is_default = default_name.as_deref() == Some(&name);
            WalletInfo {
                name,
                public_key,
                is_default,
            }
        })
        .collect();

    Ok(WalletList { wallets })
}

pub fn execute_balance_inner(
    ctx: &AppContext,
    name: Option<&str>,
) -> Result<WalletBalance, VulcanError> {
    let wallet_name = match name {
        Some(n) => n.to_string(),
        None => ctx
            .wallet_store
            .default_wallet()
            .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
            .ok_or_else(|| {
                VulcanError::config(
                    "NO_DEFAULT_WALLET",
                    "No default wallet set. Use 'vulcan wallet set-default <NAME>'",
                )
            })?,
    };

    let wallet_file = ctx
        .wallet_store
        .load(&wallet_name)
        .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

    let pubkey = Pubkey::try_from(wallet_file.public_key.as_str())
        .map_err(|e| VulcanError::validation("INVALID_PUBKEY", e.to_string()))?;

    let rpc_client =
        solana_rpc_client::rpc_client::RpcClient::new(ctx.config.network.rpc_url.clone());

    let sol_lamports = rpc_client
        .get_balance(&pubkey)
        .map_err(|e| VulcanError::network("RPC_BALANCE_FAILED", e.to_string()))?;
    let sol = sol_lamports as f64 / 1_000_000_000.0;

    let usdc_mint = Pubkey::try_from("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
    let ata = spl_associated_token_address(&pubkey, &usdc_mint);

    let usdc = match rpc_client.get_token_account_balance(&ata) {
        Ok(balance) => balance.ui_amount.unwrap_or(0.0),
        Err(_) => 0.0,
    };

    Ok(WalletBalance {
        name: wallet_name,
        address: wallet_file.public_key,
        sol,
        usdc,
    })
}
