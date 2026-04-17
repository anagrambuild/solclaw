//! History subcommand definitions.

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    /// Past trade/fill history
    Trades {
        /// Filter by market symbol
        #[arg(long)]
        symbol: Option<String>,
        /// Max results to return
        #[arg(long, default_value = "20")]
        limit: i64,
    },

    /// Past order history
    Orders {
        /// Filter by market symbol
        #[arg(long)]
        symbol: Option<String>,
        /// Max results to return
        #[arg(long, default_value = "20")]
        limit: i64,
    },

    /// Deposit/withdrawal history
    Collateral {
        /// Max results to return
        #[arg(long, default_value = "20")]
        limit: i64,
    },

    /// Funding payment history
    Funding {
        /// Filter by market symbol
        #[arg(long)]
        symbol: Option<String>,
        /// Max results to return
        #[arg(long, default_value = "20")]
        limit: i64,
    },

    /// PnL over time
    Pnl {
        /// Resolution: hourly or daily
        #[arg(long, default_value = "hourly")]
        resolution: String,
        /// Max results to return
        #[arg(long, default_value = "24")]
        limit: i64,
    },
}
