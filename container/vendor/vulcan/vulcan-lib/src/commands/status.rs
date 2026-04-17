//! Status command — diagnostic health check for agents and users.

use crate::context::AppContext;
use crate::error::VulcanError;
use crate::output::{render_success, TableRenderable};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub config: ConfigStatus,
    pub wallet: WalletStatus,
    pub rpc: ConnectivityStatus,
    pub api: ApiStatus,
    pub trader: TraderStatus,
}

#[derive(Debug, Serialize)]
pub struct ConfigStatus {
    pub ok: bool,
    pub path: String,
    pub rpc_url: String,
    pub api_url: String,
}

#[derive(Debug, Serialize)]
pub struct WalletStatus {
    pub ok: bool,
    pub name: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectivityStatus {
    pub ok: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiStatus {
    pub ok: bool,
    pub markets: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TraderStatus {
    pub ok: bool,
    pub registered: bool,
    pub trader_key: Option<String>,
    pub collateral: Option<String>,
    pub error: Option<String>,
}

impl TableRenderable for StatusReport {
    fn render_table(&self) {
        let check = |ok: bool| if ok { "OK" } else { "FAIL" };

        println!("Vulcan Status");
        println!("─────────────────────────────────────────");

        println!(
            "  Config:  [{}]  {}",
            check(self.config.ok),
            self.config.path
        );
        println!("           RPC: {}", self.config.rpc_url);
        println!("           API: {}", self.config.api_url);

        print!("  Wallet:  [{}]", check(self.wallet.ok));
        if let Some(name) = &self.wallet.name {
            print!("  {}", name);
        }
        if let Some(pk) = &self.wallet.public_key {
            print!(" ({})", pk);
        }
        if !self.wallet.ok {
            print!("  No default wallet configured");
        }
        println!();

        print!("  RPC:     [{}]", check(self.rpc.ok));
        if let Some(v) = &self.rpc.version {
            print!("  Solana {}", v);
        }
        if let Some(e) = &self.rpc.error {
            print!("  {}", e);
        }
        println!();

        print!("  API:     [{}]", check(self.api.ok));
        if let Some(m) = self.api.markets {
            print!("  {} markets available", m);
        }
        if let Some(e) = &self.api.error {
            print!("  {}", e);
        }
        println!();

        print!("  Trader:  [{}]", check(self.trader.ok));
        if self.trader.registered {
            if let Some(k) = &self.trader.trader_key {
                print!("  {}", k);
            }
            if let Some(c) = &self.trader.collateral {
                print!(" (${} USDC)", c);
            }
        } else if self.trader.ok {
            print!("  Not registered");
        }
        if let Some(e) = &self.trader.error {
            print!("  {}", e);
        }
        println!();
    }
}

pub async fn execute(ctx: &AppContext) -> Result<(), VulcanError> {
    let report = execute_inner(ctx).await?;
    render_success(ctx.output_format, &report, serde_json::Value::Null);
    Ok(())
}

pub async fn execute_inner(ctx: &AppContext) -> Result<StatusReport, VulcanError> {
    // Config check — always passes if we got this far
    let config = ConfigStatus {
        ok: true,
        path: crate::config::VulcanConfig::dir()
            .join("config.toml")
            .to_string_lossy()
            .to_string(),
        rpc_url: ctx.config.network.rpc_url.clone(),
        api_url: ctx.config.network.api_url.clone(),
    };

    // Wallet check
    let wallet = match ctx.wallet_store.default_wallet() {
        Ok(Some(name)) => match ctx.wallet_store.load(&name) {
            Ok(wf) => WalletStatus {
                ok: true,
                name: Some(name),
                public_key: Some(wf.public_key.clone()),
            },
            Err(_) => WalletStatus {
                ok: false,
                name: Some(name),
                public_key: None,
            },
        },
        _ => WalletStatus {
            ok: false,
            name: None,
            public_key: None,
        },
    };

    // RPC connectivity check
    let rpc = {
        let rpc_client =
            solana_rpc_client::rpc_client::RpcClient::new(ctx.config.network.rpc_url.clone());
        match rpc_client.get_version() {
            Ok(v) => ConnectivityStatus {
                ok: true,
                version: Some(v.solana_core),
                error: None,
            },
            Err(e) => ConnectivityStatus {
                ok: false,
                version: None,
                error: Some(e.to_string()),
            },
        }
    };

    // API connectivity check
    let api = match ctx.http_client.get_markets().await {
        Ok(markets) => ApiStatus {
            ok: true,
            markets: Some(markets.len()),
            error: None,
        },
        Err(e) => ApiStatus {
            ok: false,
            markets: None,
            error: Some(e.to_string()),
        },
    };

    // Trader registration check
    let trader = if let Some(pk) = &wallet.public_key {
        match solana_pubkey::Pubkey::try_from(pk.as_str()) {
            Ok(authority) => match ctx.http_client.get_traders(&authority).await {
                Ok(traders) => {
                    let cross = traders.iter().find(|t| t.trader_subaccount_index == 0);
                    match cross {
                        Some(t) => TraderStatus {
                            ok: true,
                            registered: true,
                            trader_key: Some(t.trader_key.clone()),
                            collateral: Some(t.collateral_balance.ui.clone()),
                            error: None,
                        },
                        None => TraderStatus {
                            ok: true,
                            registered: false,
                            trader_key: None,
                            collateral: None,
                            error: None,
                        },
                    }
                }
                Err(e) => TraderStatus {
                    ok: false,
                    registered: false,
                    trader_key: None,
                    collateral: None,
                    error: Some(e.to_string()),
                },
            },
            Err(_) => TraderStatus {
                ok: false,
                registered: false,
                trader_key: None,
                collateral: None,
                error: Some("Invalid public key".to_string()),
            },
        }
    } else {
        TraderStatus {
            ok: false,
            registered: false,
            trader_key: None,
            collateral: None,
            error: Some("No wallet configured".to_string()),
        }
    };

    Ok(StatusReport {
        config,
        wallet,
        rpc,
        api,
        trader,
    })
}
