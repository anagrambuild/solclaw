//! Application context — shared state across commands.

use crate::config::VulcanConfig;
use crate::mcp::session_wallet::SessionWallet;
use crate::output::OutputFormat;
use crate::wallet::WalletStore;
use anyhow::Result;
use phoenix_sdk::{PhoenixHttpClient, PhoenixTxBuilder};
use phoenix_types::PhoenixMetadata;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Shared application context available to all commands.
pub struct AppContext {
    pub config: VulcanConfig,
    pub wallet_store: WalletStore,
    pub output_format: OutputFormat,
    pub dry_run: bool,
    pub yes: bool,
    pub verbose: bool,
    pub watch: bool,
    pub vulcan_dir: PathBuf,
    pub http_client: PhoenixHttpClient,
    /// Pre-decrypted session wallet for MCP mode (None in CLI mode).
    pub session_wallet: Option<Arc<SessionWallet>>,
    /// Lazily-initialized metadata (fetched on first use).
    metadata: OnceCell<PhoenixMetadata>,
}

impl AppContext {
    /// Build an AppContext from global CLI flags and config.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        output_format: OutputFormat,
        dry_run: bool,
        yes: bool,
        verbose: bool,
        watch: bool,
        rpc_url: Option<String>,
        api_url: Option<String>,
        api_key: Option<String>,
    ) -> Result<Self> {
        let mut config = VulcanConfig::load()?;

        // CLI flags override config
        if let Some(rpc) = rpc_url {
            config.network.rpc_url = rpc;
        }
        if let Some(api) = api_url {
            config.network.api_url = api;
        }
        if let Some(key) = api_key {
            config.network.api_key = Some(key);
        }

        let vulcan_dir = VulcanConfig::dir();
        std::fs::create_dir_all(&vulcan_dir)?;

        let wallet_store = WalletStore::new(&vulcan_dir)?;

        // Build HTTP client from config (not env vars — config takes precedence)
        let http_client = match &config.network.api_key {
            Some(key) => PhoenixHttpClient::new(&config.network.api_url, key),
            None => PhoenixHttpClient::new_public(&config.network.api_url),
        };

        Ok(Self {
            config,
            wallet_store,
            output_format,
            dry_run,
            yes,
            verbose,
            watch,
            vulcan_dir,
            http_client,
            session_wallet: None,
            metadata: OnceCell::new(),
        })
    }

    /// Get exchange metadata, fetching it lazily on first call.
    pub async fn metadata(&self) -> Result<&PhoenixMetadata, crate::error::VulcanError> {
        self.metadata
            .get_or_try_init(|| async {
                let exchange = self.http_client.get_exchange().await.map_err(|e| {
                    crate::error::VulcanError::api("EXCHANGE_FETCH_FAILED", e.to_string())
                })?;
                let view: phoenix_types::ExchangeView = exchange.into();
                Ok(PhoenixMetadata::new(view))
            })
            .await
    }

    /// Create a transaction builder from cached metadata.
    pub async fn tx_builder(&self) -> Result<PhoenixTxBuilder<'_>, crate::error::VulcanError> {
        let metadata = self.metadata().await?;
        Ok(PhoenixTxBuilder::new(metadata))
    }
}
