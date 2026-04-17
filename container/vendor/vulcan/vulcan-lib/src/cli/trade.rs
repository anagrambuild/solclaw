//! Trade subcommand definitions.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum TradeCommand {
    /// Place a market buy order
    MarketBuy {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Order size in base lots
        size: f64,
        /// Take-profit price
        #[arg(long)]
        tp: Option<f64>,
        /// Stop-loss price
        #[arg(long)]
        sl: Option<f64>,
        /// Use isolated margin
        #[arg(long)]
        isolated: bool,
        /// USDC collateral for isolated subaccount
        #[arg(long, requires = "isolated")]
        collateral: Option<f64>,
        /// Reduce-only order
        #[arg(long)]
        reduce_only: bool,
    },

    /// Place a market sell order
    MarketSell {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Order size in base lots
        size: f64,
        /// Take-profit price
        #[arg(long)]
        tp: Option<f64>,
        /// Stop-loss price
        #[arg(long)]
        sl: Option<f64>,
        /// Use isolated margin
        #[arg(long)]
        isolated: bool,
        /// USDC collateral for isolated subaccount
        #[arg(long, requires = "isolated")]
        collateral: Option<f64>,
        /// Reduce-only order
        #[arg(long)]
        reduce_only: bool,
    },

    /// Place a limit buy order
    LimitBuy {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Order size in base lots
        size: f64,
        /// Limit price
        price: f64,
        /// Take-profit price
        #[arg(long)]
        tp: Option<f64>,
        /// Stop-loss price
        #[arg(long)]
        sl: Option<f64>,
        /// Use isolated margin
        #[arg(long)]
        isolated: bool,
        /// USDC collateral for isolated subaccount
        #[arg(long, requires = "isolated")]
        collateral: Option<f64>,
        /// Reduce-only order
        #[arg(long)]
        reduce_only: bool,
    },

    /// Place a limit sell order
    LimitSell {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Order size in base lots
        size: f64,
        /// Limit price
        price: f64,
        /// Take-profit price
        #[arg(long)]
        tp: Option<f64>,
        /// Stop-loss price
        #[arg(long)]
        sl: Option<f64>,
        /// Use isolated margin
        #[arg(long)]
        isolated: bool,
        /// USDC collateral for isolated subaccount
        #[arg(long, requires = "isolated")]
        collateral: Option<f64>,
        /// Reduce-only order
        #[arg(long)]
        reduce_only: bool,
    },

    /// Cancel specific orders by ID
    Cancel {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Order IDs to cancel
        #[arg(required = true)]
        order_ids: Vec<String>,
    },

    /// Cancel all open orders for a market
    CancelAll {
        /// Market symbol (e.g., SOL)
        symbol: String,
    },

    /// List open orders (optionally filter by market)
    Orders {
        /// Market symbol (e.g., SOL). Omit to list all.
        symbol: Option<String>,
    },

    /// Set take-profit and/or stop-loss on an existing position
    SetTpsl {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Take-profit price
        #[arg(long)]
        tp: Option<f64>,
        /// Stop-loss price
        #[arg(long)]
        sl: Option<f64>,
    },

    /// Cancel take-profit and/or stop-loss on an existing position
    CancelTpsl {
        /// Market symbol (e.g., SOL)
        symbol: String,
        /// Cancel take-profit
        #[arg(long)]
        tp: bool,
        /// Cancel stop-loss
        #[arg(long)]
        sl: bool,
    },
}
