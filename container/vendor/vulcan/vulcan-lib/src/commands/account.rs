//! Account command execution.

use crate::cli::account::AccountCommand;
use crate::context::AppContext;
use crate::error::VulcanError;
use crate::output::{render_success, TableRenderable};
use serde::Serialize;
use solana_pubkey::Pubkey;
use std::str::FromStr;

// ── Result types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RegisterResult {
    pub authority: String,
    pub trader_pda: String,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
}

impl TableRenderable for RegisterResult {
    fn render_table(&self) {
        if self.dry_run {
            println!("[DRY RUN] Would register trader account:");
        } else {
            println!("Trader account registered:");
        }
        println!("  Authority: {}", self.authority);
        println!("  Trader PDA: {}", self.trader_pda);
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AccountInfoResult {
    pub authority: String,
    pub trader_key: String,
    pub pda_index: u8,
    pub subaccount_index: u8,
    pub state: String,
    pub collateral_balance: String,
    pub portfolio_value: String,
    pub risk_state: String,
    pub risk_tier: String,
    pub num_positions: usize,
    pub num_open_orders: usize,
    pub max_positions: u64,
}

impl TableRenderable for AccountInfoResult {
    fn render_table(&self) {
        println!("Account Info:");
        println!("  Authority: {}", self.authority);
        println!("  Trader key: {}", self.trader_key);
        println!("  PDA index: {}", self.pda_index);
        println!("  Subaccount index: {}", self.subaccount_index);
        println!("  State: {}", self.state);
        println!("  Collateral: {}", self.collateral_balance);
        println!("  Portfolio value: {}", self.portfolio_value);
        println!("  Risk state: {}", self.risk_state);
        println!("  Risk tier: {}", self.risk_tier);
        println!("  Positions: {}/{}", self.num_positions, self.max_positions);
        println!("  Open orders: {}", self.num_open_orders);
    }
}

#[derive(Debug, Serialize)]
pub struct SubaccountListResult {
    pub authority: String,
    pub subaccounts: Vec<SubaccountInfo>,
}

#[derive(Debug, Serialize)]
pub struct SubaccountInfo {
    pub trader_key: String,
    pub pda_index: u8,
    pub subaccount_index: u8,
    pub state: String,
    pub collateral_balance: String,
    pub num_positions: usize,
    pub margin_type: String,
}

impl TableRenderable for SubaccountListResult {
    fn render_table(&self) {
        if self.subaccounts.is_empty() {
            println!("No subaccounts found.");
            return;
        }
        let rows: Vec<Vec<String>> = self
            .subaccounts
            .iter()
            .map(|s| {
                vec![
                    format!("{}", s.subaccount_index),
                    s.margin_type.clone(),
                    s.state.clone(),
                    s.collateral_balance.clone(),
                    s.num_positions.to_string(),
                ]
            })
            .collect();
        crate::output::table::render_table(
            &["Subaccount", "Type", "State", "Collateral", "Positions"],
            rows,
        );
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn resolve_authority(ctx: &AppContext) -> Result<(String, Pubkey), VulcanError> {
    let wallet_name = ctx
        .wallet_store
        .default_wallet()
        .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
        .ok_or_else(|| VulcanError::config("NO_DEFAULT_WALLET", "No default wallet set"))?;

    let wallet_file = ctx
        .wallet_store
        .load(&wallet_name)
        .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

    let authority = Pubkey::from_str(&wallet_file.public_key)
        .map_err(|e| VulcanError::validation("INVALID_PUBKEY", e.to_string()))?;

    Ok((wallet_name, authority))
}

// ── Execution ───────────────────────────────────────────────────────────

pub async fn execute(ctx: &AppContext, cmd: AccountCommand) -> Result<(), VulcanError> {
    match cmd {
        AccountCommand::Register { invite_code } => {
            let (wallet_name, authority) = resolve_authority(ctx)?;

            // Step 1: Register via HTTP API (activate invite code)
            let _reg_result = ctx
                .http_client
                .register_trader(&authority, &invite_code)
                .await
                .map_err(|e| VulcanError::api("REGISTER_API_FAILED", e.to_string()))?;

            // Step 2: Check if trader already exists on-chain
            let already_registered = ctx
                .http_client
                .get_traders(&authority)
                .await
                .map(|traders| traders.iter().any(|t| t.trader_subaccount_index == 0))
                .unwrap_or(false);

            let sig = if already_registered {
                eprintln!("Trader account already registered, skipping on-chain transaction.");
                None
            } else {
                // Step 3: Build and submit on-chain registration transaction
                let builder = ctx.tx_builder().await?;
                let ixs = builder
                    .build_register_trader(authority, 0, 0)
                    .map_err(|e| VulcanError::api("BUILD_REGISTER_FAILED", e.to_string()))?;

                let wallet = if let Some(sw) = &ctx.session_wallet {
                    sw.to_wallet()?
                } else {
                    let wallet_file = ctx
                        .wallet_store
                        .load(&wallet_name)
                        .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;
                    let password = crate::commands::trade::prompt_password()?;
                    crate::wallet::Wallet::decrypt(&wallet_file.encrypted, &password)
                        .map_err(|e| VulcanError::auth("DECRYPT_FAILED", e.to_string()))?
                };

                crate::commands::trade::send_or_dry_run(ctx, ixs, &wallet).await?
            };

            let trader_key = phoenix_sdk::types::TraderKey::new(authority);
            let result = RegisterResult {
                authority: authority.to_string(),
                trader_pda: trader_key.pda().to_string(),
                dry_run: ctx.dry_run,
                tx_signature: sig,
            };

            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        AccountCommand::Info => {
            let (_, authority) = resolve_authority(ctx)?;

            let traders = ctx
                .http_client
                .get_traders(&authority)
                .await
                .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

            let trader = traders
                .iter()
                .find(|t| t.trader_subaccount_index == 0)
                .ok_or_else(|| {
                    VulcanError::api(
                        "NO_TRADER_ACCOUNT",
                        "No registered trader account found. Use 'vulcan account register' first.",
                    )
                })?;

            let total_orders: usize = trader.limit_orders.values().map(|v| v.len()).sum();

            let result = AccountInfoResult {
                authority: trader.authority.clone(),
                trader_key: trader.trader_key.clone(),
                pda_index: trader.trader_pda_index,
                subaccount_index: trader.trader_subaccount_index,
                state: format!("{:?}", trader.state),
                collateral_balance: trader.collateral_balance.ui.clone(),
                portfolio_value: trader.portfolio_value.ui.clone(),
                risk_state: format!("{:?}", trader.risk_state),
                risk_tier: format!("{:?}", trader.risk_tier),
                num_positions: trader.positions.len(),
                num_open_orders: total_orders,
                max_positions: trader.max_positions,
            };

            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        AccountCommand::Subaccounts => {
            let (_, authority) = resolve_authority(ctx)?;

            let traders = ctx
                .http_client
                .get_traders(&authority)
                .await
                .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

            let subaccounts: Vec<SubaccountInfo> = traders
                .iter()
                .map(|t| SubaccountInfo {
                    trader_key: t.trader_key.clone(),
                    pda_index: t.trader_pda_index,
                    subaccount_index: t.trader_subaccount_index,
                    state: format!("{:?}", t.state),
                    collateral_balance: t.collateral_balance.ui.clone(),
                    num_positions: t.positions.len(),
                    margin_type: if t.trader_subaccount_index == 0 {
                        "Cross".to_string()
                    } else {
                        "Isolated".to_string()
                    },
                })
                .collect();

            let result = SubaccountListResult {
                authority: authority.to_string(),
                subaccounts,
            };

            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        AccountCommand::CreateSubaccount {
            pda_index,
            subaccount_index,
        } => {
            if subaccount_index == 0 {
                return Err(VulcanError::validation(
                    "INVALID_SUBACCOUNT",
                    "Subaccount index 0 is reserved for cross-margin. Use 1+ for isolated.",
                ));
            }

            let (wallet_name, authority) = resolve_authority(ctx)?;
            let builder = ctx.tx_builder().await?;

            let ixs = builder
                .build_register_trader(authority, pda_index, subaccount_index)
                .map_err(|e| VulcanError::api("BUILD_REGISTER_FAILED", e.to_string()))?;

            let wallet = if let Some(sw) = &ctx.session_wallet {
                sw.to_wallet()?
            } else {
                let wallet_file = ctx
                    .wallet_store
                    .load(&wallet_name)
                    .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;
                let password = crate::commands::trade::prompt_password()?;
                crate::wallet::Wallet::decrypt(&wallet_file.encrypted, &password)
                    .map_err(|e| VulcanError::auth("DECRYPT_FAILED", e.to_string()))?
            };

            let sig = crate::commands::trade::send_or_dry_run(ctx, ixs, &wallet).await?;

            let trader_key =
                phoenix_sdk::types::TraderKey::new_with_idx(authority, pda_index, subaccount_index);
            let result = RegisterResult {
                authority: authority.to_string(),
                trader_pda: trader_key.pda().to_string(),
                dry_run: ctx.dry_run,
                tx_signature: sig,
            };

            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }
    }
}

// ── Inner functions for MCP ────────────────────────────────────────────

pub async fn execute_info_inner(ctx: &AppContext) -> Result<AccountInfoResult, VulcanError> {
    let (_, authority) = resolve_authority(ctx)?;

    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let trader = traders
        .iter()
        .find(|t| t.trader_subaccount_index == 0)
        .ok_or_else(|| {
            VulcanError::api(
                "NO_TRADER_ACCOUNT",
                "No registered trader account found. Use 'vulcan account register' first.",
            )
        })?;

    let total_orders: usize = trader.limit_orders.values().map(|v| v.len()).sum();

    Ok(AccountInfoResult {
        authority: trader.authority.clone(),
        trader_key: trader.trader_key.clone(),
        pda_index: trader.trader_pda_index,
        subaccount_index: trader.trader_subaccount_index,
        state: format!("{:?}", trader.state),
        collateral_balance: trader.collateral_balance.ui.clone(),
        portfolio_value: trader.portfolio_value.ui.clone(),
        risk_state: format!("{:?}", trader.risk_state),
        risk_tier: format!("{:?}", trader.risk_tier),
        num_positions: trader.positions.len(),
        num_open_orders: total_orders,
        max_positions: trader.max_positions,
    })
}

pub async fn execute_register_inner(
    ctx: &AppContext,
    invite_code: &str,
) -> Result<RegisterResult, VulcanError> {
    let (wallet_name, authority) = resolve_authority(ctx)?;

    // Step 1: Register via HTTP API
    let _reg_result = ctx
        .http_client
        .register_trader(&authority, invite_code)
        .await
        .map_err(|e| VulcanError::api("REGISTER_API_FAILED", e.to_string()))?;

    // Step 2: Check if already registered
    let already_registered = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map(|traders| traders.iter().any(|t| t.trader_subaccount_index == 0))
        .unwrap_or(false);

    let sig = if already_registered {
        None
    } else {
        let builder = ctx.tx_builder().await?;
        let ixs = builder
            .build_register_trader(authority, 0, 0)
            .map_err(|e| VulcanError::api("BUILD_REGISTER_FAILED", e.to_string()))?;

        let wallet = if let Some(sw) = &ctx.session_wallet {
            sw.to_wallet()?
        } else {
            let wallet_file = ctx
                .wallet_store
                .load(&wallet_name)
                .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;
            let password = crate::commands::trade::prompt_password()?;
            crate::wallet::Wallet::decrypt(&wallet_file.encrypted, &password)
                .map_err(|e| VulcanError::auth("DECRYPT_FAILED", e.to_string()))?
        };

        crate::commands::trade::send_or_dry_run(ctx, ixs, &wallet).await?
    };

    let trader_key = phoenix_sdk::types::TraderKey::new(authority);
    Ok(RegisterResult {
        authority: authority.to_string(),
        trader_pda: trader_key.pda().to_string(),
        dry_run: ctx.dry_run,
        tx_signature: sig,
    })
}
