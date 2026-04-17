//! Example: Send a market order using PhoenixTxBuilder.
//!
//! This example demonstrates how to use PhoenixTxBuilder with a raw Solana
//! RPC client for order placement.
//!
//! Run with:
//!   export PHOENIX_API_KEY=your_api_key  # optional
//!   cargo run -p phoenix-sdk --example send_market_order -- SOL [SL_PRICE|x]
//! [TP_PRICE|x]
//!
//! Use "x" in place of a price to skip stop-loss or take-profit.
//! Examples:
//!   send_market_order SOL              # no bracket legs
//!   send_market_order SOL 120.5 150.0  # SL at 120.5, TP at 150.0
//!   send_market_order SOL x 150.0      # no SL, TP at 150.0
//!   send_market_order SOL 120.5 x      # SL at 120.5, no TP

use std::env;

use phoenix_sdk::{
    BracketLegOrders, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, Side, TraderKey,
};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: send_market_order <SYMBOL> [SL_PRICE|x] [TP_PRICE|x]");
        eprintln!("Example: send_market_order SOL 120.5 150.0");
        std::process::exit(1);
    }
    let symbol = &args[1];

    fn parse_price(arg: &str) -> Option<f64> {
        if arg.eq_ignore_ascii_case("x") {
            return None;
        }
        Some(arg.parse::<f64>().expect("invalid price"))
    }

    let sl_price = args.get(2).and_then(|s| parse_price(s));
    let tp_price = args.get(3).and_then(|s| parse_price(s));
    let bracket = if sl_price.is_some() || tp_price.is_some() {
        Some(BracketLegOrders {
            stop_loss_price: sl_price,
            take_profit_price: tp_price,
        })
    } else {
        None
    };

    // Load keypair
    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });

    println!("Loading keypair from: {}", keypair_path);
    let keypair =
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?;
    let trader = TraderKey::new(keypair.pubkey());

    println!("Trader authority: {}", trader.authority());
    println!("Trader PDA: {}", trader.pda());

    // Fetch exchange metadata via HTTP
    println!("\nFetching exchange metadata...");
    let http = PhoenixHttpClient::new_from_env();
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);

    // Show cached market info
    if let Some(market) = metadata.get_market(symbol) {
        println!("\n=== {} Market ===", market.symbol);
        println!("  Market Key: {}", market.market_pubkey);
        println!("  Spline Collection: {}", market.spline_pubkey);
        println!("  Taker Fee: {:.4}%", market.taker_fee * 100.0);
        println!("  Maker Fee: {:.4}%", market.maker_fee * 100.0);
    }

    // Build market order instructions
    let builder = PhoenixTxBuilder::new(&metadata);
    let num_base_lots = 67;
    println!(
        "\nPlacing market order: {} {} base lots",
        symbol, num_base_lots
    );
    if let Some(ref b) = bracket {
        if let Some(sl) = b.stop_loss_price {
            println!("  Stop-loss: {}", sl);
        }
        if let Some(tp) = b.take_profit_price {
            println!("  Take-profit: {}", tp);
        }
    }

    let instructions = builder.build_market_order(
        trader.authority(),
        trader.pda(),
        symbol,
        Side::Bid,
        num_base_lots,
        bracket.as_ref(),
    )?;

    // Send transaction via raw Solana RPC client
    let rpc =
        RpcClient::new_with_commitment(RPC_ENDPOINT.to_string(), CommitmentConfig::confirmed());
    let blockhash = rpc.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    let signature = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Transaction confirmed!");
    println!("Signature: {}", signature);
    println!("Explorer: https://explorer.solana.com/tx/{}", signature);

    Ok(())
}
