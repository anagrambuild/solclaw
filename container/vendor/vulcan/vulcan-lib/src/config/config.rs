//! Config file parsing and defaults.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VulcanConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub trading: TradingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WalletConfig {
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    #[serde(default = "default_slippage")]
    pub default_slippage_bps: u32,
    #[serde(default = "default_true")]
    pub confirm_trades: bool,
}

fn default_rpc_url() -> String {
    "https://api.mainnet-beta.solana.com".to_string()
}

fn default_api_url() -> String {
    "https://perp-api.phoenix.trade".to_string()
}

fn default_format() -> String {
    "table".to_string()
}

fn default_true() -> bool {
    true
}

fn default_slippage() -> u32 {
    50
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            rpc_url: default_rpc_url(),
            api_url: default_api_url(),
            api_key: None,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            color: default_true(),
        }
    }
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            default_slippage_bps: default_slippage(),
            confirm_trades: default_true(),
        }
    }
}

impl VulcanConfig {
    /// Path to the Vulcan config directory.
    pub fn dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".vulcan")
    }

    /// Path to the config file.
    pub fn path() -> PathBuf {
        Self::dir().join("config.toml")
    }

    /// Load config from disk, or return defaults if not found.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::path(), content)?;
        Ok(())
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
