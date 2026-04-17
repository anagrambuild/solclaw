//! Interactive setup wizard — wallet creation, config, and connectivity check.

use crate::config::VulcanConfig;
use crate::error::VulcanError;
use crate::wallet::{Wallet, WalletFile, WalletStore};
use std::io::{self, BufRead, Write};
use std::path::Path;

// ── Prompt helpers ─────────────────────────────────────────────────────

fn prompt(msg: &str) -> Result<String, VulcanError> {
    print!("{msg}");
    io::stdout()
        .flush()
        .map_err(|e| VulcanError::io("FLUSH_FAILED", e.to_string()))?;
    let mut input = String::new();
    io::stdin()
        .lock()
        .read_line(&mut input)
        .map_err(|e| VulcanError::io("READ_FAILED", e.to_string()))?;
    Ok(input.trim().to_string())
}

fn prompt_yn(msg: &str, default: bool) -> Result<bool, VulcanError> {
    let hint = if default { "Y/n" } else { "y/N" };
    let input = prompt(&format!("{msg} [{hint}] "))?;
    Ok(match input.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    })
}

fn prompt_password(msg: &str) -> Result<String, VulcanError> {
    // Use rpassword if available, otherwise fall back to regular prompt
    print!("{msg}");
    io::stdout()
        .flush()
        .map_err(|e| VulcanError::io("FLUSH_FAILED", e.to_string()))?;
    rpassword::read_password().map_err(|e| VulcanError::io("PASSWORD_READ_FAILED", e.to_string()))
}

fn step_header(n: u8, total: u8, label: &str) {
    println!("  [{n}/{total}] {label}");
    println!("  {}", "─".repeat(label.len() + 6));
}

// ── Banner ─────────────────────────────────────────────────────────────

fn print_banner() {
    let b = "\x1b[38;2;255;100;0m"; // Phoenix orange
    let bold = "\x1b[1m";
    let dim = "\x1b[2m";
    let r = "\x1b[0m";

    println!();
    println!("  {b}{bold}██╗   ██╗██╗   ██╗██╗      ██████╗ █████╗ ███╗   ██╗{r}");
    println!("  {b}{bold}██║   ██║██║   ██║██║     ██╔════╝██╔══██╗████╗  ██║{r}");
    println!("  {b}{bold}██║   ██║██║   ██║██║     ██║     ███████║██╔██╗ ██║{r}");
    println!("  {b}{bold}╚██╗ ██╔╝██║   ██║██║     ██║     ██╔══██║██║╚██╗██║{r}");
    println!("  {b}{bold} ╚████╔╝ ╚██████╔╝███████╗╚██████╗██║  ██║██║ ╚████║{r}");
    println!("  {b}{bold}  ╚═══╝   ╚═════╝ ╚══════╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═══╝{r}");
    println!();
    println!("  {b}╭──────────────────────────────────────────────────────╮{r}");
    println!(
        "  {b}│{r}  {bold}Phoenix Perpetuals DEX{r} {dim}— AI-native CLI for Solana{r}  {b}│{r}"
    );
    println!("  {b}╰──────────────────────────────────────────────────────╯{r}");
    println!();
}

// ── Execution ──────────────────────────────────────────────────────────

pub fn execute(wallet_store: &WalletStore) -> Result<(), VulcanError> {
    print_banner();

    let total = 4;

    // ── Step 1: Wallet ─────────────────────────────────────────────────
    step_header(1, total, "Wallet");

    let wallets = wallet_store
        .list()
        .map_err(|e| VulcanError::io("WALLET_LIST_FAILED", e.to_string()))?;

    if !wallets.is_empty() {
        let default = wallet_store
            .default_wallet()
            .map_err(|e| VulcanError::io("WALLET_DEFAULT_FAILED", e.to_string()))?;
        let default_label = default.as_deref().unwrap_or("none");
        println!(
            "  ✓ {} wallet(s) found (default: {})",
            wallets.len(),
            default_label
        );
        for name in &wallets {
            let marker = if Some(name.as_str()) == default.as_deref() {
                "→"
            } else {
                " "
            };
            println!("    {marker} {name}");
        }
        println!();

        if !prompt_yn("  Add another wallet?", false)? {
            println!();
            return finish_setup(wallet_store);
        }
        println!();
    }

    let (wallet_name, address) = setup_wallet(wallet_store)?;

    // Set as default if it's the only wallet or user wants it
    let wallets_after = wallet_store
        .list()
        .map_err(|e| VulcanError::io("WALLET_LIST_FAILED", e.to_string()))?;
    if wallets_after.len() == 1 {
        wallet_store
            .set_default(&wallet_name)
            .map_err(|e| VulcanError::io("SET_DEFAULT_FAILED", e.to_string()))?;
        println!("  ✓ Set as default wallet");
    } else if prompt_yn("  Set as default wallet?", true)? {
        wallet_store
            .set_default(&wallet_name)
            .map_err(|e| VulcanError::io("SET_DEFAULT_FAILED", e.to_string()))?;
        println!("  ✓ Set as default wallet");
    }

    println!("    Address: {address}");
    println!();

    finish_setup(wallet_store)
}

fn setup_wallet(wallet_store: &WalletStore) -> Result<(String, String), VulcanError> {
    let name = prompt("  Wallet name: ")?;
    if name.is_empty() {
        return Err(VulcanError::validation(
            "EMPTY_NAME",
            "Wallet name cannot be empty",
        ));
    }
    if wallet_store.exists(&name) {
        return Err(VulcanError::validation(
            "WALLET_EXISTS",
            format!("Wallet '{}' already exists", name),
        ));
    }

    println!();
    println!("  How would you like to set up your wallet?");
    println!("    1) Generate a new keypair");
    println!("    2) Import from base58 private key");
    println!("    3) Import from Solana keypair file (JSON)");
    println!();

    let choice = prompt("  Choice [1]: ")?;
    let choice = if choice.is_empty() {
        "1".to_string()
    } else {
        choice
    };

    let wallet = match choice.as_str() {
        "1" => {
            println!();
            println!("  Generating new keypair...");
            Wallet::generate().map_err(|e| VulcanError::internal("KEYGEN_FAILED", e.to_string()))?
        }
        "2" => {
            let key = prompt("  Enter base58 private key: ")?;
            Wallet::from_base58(&key)
                .map_err(|e| VulcanError::validation("INVALID_KEY", e.to_string()))?
        }
        "3" => {
            let path_str = prompt("  Path to keypair file: ")?;
            let path = Path::new(&path_str);
            if !path.exists() {
                return Err(VulcanError::validation(
                    "FILE_NOT_FOUND",
                    format!("File not found: {}", path_str),
                ));
            }
            Wallet::from_file(path)
                .map_err(|e| VulcanError::validation("INVALID_FILE", e.to_string()))?
        }
        _ => {
            return Err(VulcanError::validation(
                "INVALID_CHOICE",
                "Please enter 1, 2, or 3",
            ));
        }
    };

    let address = wallet.public_key.clone();

    // Encrypt with password
    println!();
    let password = prompt_password("  Set encryption password: ")?;
    if password.is_empty() {
        return Err(VulcanError::validation(
            "EMPTY_PASSWORD",
            "Password cannot be empty",
        ));
    }
    let password_confirm = prompt_password("  Confirm password: ")?;
    if password != password_confirm {
        return Err(VulcanError::validation(
            "PASSWORD_MISMATCH",
            "Passwords do not match",
        ));
    }

    let encrypted = wallet
        .encrypt(&password)
        .map_err(|e| VulcanError::internal("ENCRYPT_FAILED", e.to_string()))?;

    let wallet_file = WalletFile {
        name: name.clone(),
        public_key: address.clone(),
        encrypted,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    wallet_store
        .save(&wallet_file)
        .map_err(|e| VulcanError::io("WALLET_SAVE_FAILED", e.to_string()))?;

    println!();
    if choice == "1" {
        println!("  ✓ Wallet created");
        println!();
        println!("  ⚠ Back up your wallet file. If lost, your funds cannot be recovered.");
        println!("    File: {}", wallet_store.wallet_path(&name).display());
    } else {
        println!("  ✓ Wallet imported");
    }

    Ok((name, address))
}

fn finish_setup(wallet_store: &WalletStore) -> Result<(), VulcanError> {
    let total = 4;

    // ── Step 2: Config ─────────────────────────────────────────────────
    step_header(2, total, "Configuration");

    let config_path = VulcanConfig::path();
    if config_path.exists() {
        println!("  ✓ Config file found");
        println!("    Path: {}", config_path.display());
        println!();

        if prompt_yn("  Reconfigure?", false)? {
            println!();
            configure()?;
        }
    } else {
        println!("  ○ No config file found — using defaults");
        println!();

        if prompt_yn("  Customize configuration?", false)? {
            println!();
            configure()?;
        } else {
            // Save defaults
            let config = VulcanConfig::default();
            config
                .save()
                .map_err(|e| VulcanError::io("CONFIG_SAVE_FAILED", e.to_string()))?;
            println!("  ✓ Default config saved to {}", config_path.display());
        }
    }

    println!();

    // ── Step 3: Verify Connection ──────────────────────────────────────
    step_header(3, total, "Verify Connection");

    let config = VulcanConfig::load()
        .map_err(|e| VulcanError::config("CONFIG_LOAD_FAILED", e.to_string()))?;
    println!("  ○ API: {}", config.network.api_url);
    println!("  ○ RPC: {}", config.network.rpc_url);
    println!(
        "  ○ API Key: {}",
        if config.network.api_key.is_some() {
            "configured"
        } else {
            "none (public access)"
        }
    );
    println!();
    println!("  Run `vulcan market list` to verify API connectivity.");

    println!();

    // ── Step 4: Next Steps ─────────────────────────────────────────────
    step_header(4, total, "Next Steps");

    let default_wallet = wallet_store.default_wallet().ok().flatten();

    if default_wallet.is_some() {
        println!("  ○ Register your trader account:");
        println!("    vulcan account register");
        println!();
        println!("  ○ Deposit collateral:");
        println!("    vulcan margin deposit <AMOUNT>");
    } else {
        println!("  ○ Create or import a wallet:");
        println!("    vulcan wallet create <NAME>");
        println!("    vulcan wallet import <NAME> --base58 <KEY>");
    }

    println!();
    println!("  ────────────────────────────────────────");
    println!("  ✓ Setup complete! You're ready to go.");
    println!();
    println!("  Quick start:");
    println!("    vulcan market list              Browse markets");
    println!("    vulcan market ticker SOL          Live price");
    println!("    vulcan trade market-buy SOL 1 --dry-run");
    println!();

    Ok(())
}

fn configure() -> Result<(), VulcanError> {
    let mut config = VulcanConfig::load()
        .map_err(|e| VulcanError::config("CONFIG_LOAD_FAILED", e.to_string()))?;

    let rpc = prompt(&format!("  Solana RPC URL [{}]: ", config.network.rpc_url))?;
    if !rpc.is_empty() {
        config.network.rpc_url = rpc;
    }

    let api = prompt(&format!("  Phoenix API URL [{}]: ", config.network.api_url))?;
    if !api.is_empty() {
        config.network.api_url = api;
    }

    let key_hint = config
        .network
        .api_key
        .as_deref()
        .map(|k| {
            if k.len() > 8 {
                format!("{}...", &k[..8])
            } else {
                k.to_string()
            }
        })
        .unwrap_or_else(|| "none".to_string());
    let api_key = prompt(&format!("  API Key [{}]: ", key_hint))?;
    if !api_key.is_empty() {
        config.network.api_key = Some(api_key);
    }

    let slippage = prompt(&format!(
        "  Default slippage (bps) [{}]: ",
        config.trading.default_slippage_bps
    ))?;
    if !slippage.is_empty() {
        config.trading.default_slippage_bps = slippage.parse().map_err(|_| {
            VulcanError::validation("INVALID_SLIPPAGE", "Slippage must be a number")
        })?;
    }

    config
        .save()
        .map_err(|e| VulcanError::io("CONFIG_SAVE_FAILED", e.to_string()))?;

    println!("  ✓ Config saved to {}", VulcanConfig::path().display());

    Ok(())
}
