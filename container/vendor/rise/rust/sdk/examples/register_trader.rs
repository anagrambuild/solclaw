//! Example: Register a trader by activating an invite code.
//!
//! Run with:
//!   cargo run -p phoenix-sdk --example register_trader -- <AUTHORITY_PUBKEY>
//! <INVITE_CODE>

use std::str::FromStr;

use phoenix_sdk::PhoenixHttpClient;
use solana_pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: register_trader <AUTHORITY_PUBKEY> <INVITE_CODE>");
        std::process::exit(1);
    }

    let authority = Pubkey::from_str(&args[1])?;
    let code = &args[2];

    let client = PhoenixHttpClient::new_from_env();
    let response = client.register_trader(&authority, code).await?;
    println!("{}", response);

    Ok(())
}
