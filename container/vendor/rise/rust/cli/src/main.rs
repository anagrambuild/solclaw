use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use phoenix_sdk::{
    CandlesQueryParams, CollateralHistoryQueryParams, FundingHistoryQueryParams,
    OrderHistoryQueryParams, PhoenixEnv, PhoenixHttpClient, PhoenixWSClient, ServerMessage,
    Timeframe, TradeHistoryQueryParams,
};
use serde::Serialize;
use solana_pubkey::Pubkey;
use tokio::time::timeout;
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "phoenix-sdk-cli")]
#[command(about = "Tiny smoke-test CLI for Phoenix SDK HTTP + WebSocket clients")]
struct Cli {
    /// Base API URL (e.g. https://public-api.phoenix.trade)
    #[arg(long, global = true)]
    api_url: Option<String>,

    /// Explicit WebSocket URL (defaults to derived from api_url)
    #[arg(long, global = true)]
    ws_url: Option<String>,

    /// API key (optional)
    #[arg(long, global = true)]
    api_key: Option<String>,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,

    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    Http {
        #[command(subcommand)]
        command: HttpCommand,
    },
    Ws {
        #[command(subcommand)]
        command: WsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum HttpCommand {
    ExchangeKeys,
    Markets,
    Market {
        #[arg(long)]
        symbol: String,
    },
    Exchange,
    Traders {
        #[arg(long)]
        authority: String,
    },
    CollateralHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long)]
        next_cursor: Option<String>,
        #[arg(long)]
        prev_cursor: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
    },
    FundingHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
    OrderHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long)]
        trader_pda_index: Option<u8>,
        #[arg(long)]
        market_symbol: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        privy_id: Option<String>,
    },
    Candles {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<u32>,
    },
    TradeHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long)]
        market_symbol: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum WsCommand {
    AllMids {
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    FundingRate {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    Orderbook {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    Market {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    Trades {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    Candles {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
    TraderState {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        trader_pda_index: u8,
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let env = build_env(&cli)?;

    match cli.command {
        RootCommand::Http { command } => run_http(command, &env, cli.pretty).await,
        RootCommand::Ws { command } => run_ws(command, &env, cli.pretty).await,
    }
}

fn build_env(cli: &Cli) -> Result<PhoenixEnv, Box<dyn Error>> {
    let mut env = PhoenixEnv::load();

    if let Some(api_url) = &cli.api_url {
        env.api_url = api_url.clone();
    }

    if let Some(api_key) = &cli.api_key {
        env.api_key = Some(api_key.clone());
    }

    env.ws_url = if let Some(ws_url) = &cli.ws_url {
        ws_url.clone()
    } else {
        derive_ws_url(&env.api_url)?
    };

    Ok(env)
}

fn derive_ws_url(api_url: &str) -> Result<String, Box<dyn Error>> {
    let mut url = Url::parse(api_url)?;
    let scheme = url.scheme();

    if let Some(rest) = scheme.strip_prefix("http") {
        let ws_scheme = format!("ws{}", rest);
        let _ = url.set_scheme(&ws_scheme);
    } else if scheme != "ws" && scheme != "wss" {
        return Err(format!("unsupported URL scheme for api_url: {scheme}").into());
    }

    let mut segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();

    if segments.last().copied() != Some("ws") {
        segments.push("ws");
    }

    url.set_path(&format!("/{}", segments.join("/")));
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.to_string())
}

async fn run_http(cmd: HttpCommand, env: &PhoenixEnv, pretty: bool) -> Result<(), Box<dyn Error>> {
    let client = PhoenixHttpClient::from_env(env.clone());

    match cmd {
        HttpCommand::ExchangeKeys => {
            let response = client.get_exchange_keys().await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::Markets => {
            let response = client.get_markets().await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::Market { symbol } => {
            let response = client.get_market(&symbol.to_ascii_uppercase()).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::Exchange => {
            let response = client.get_exchange().await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::Traders { authority } => {
            let authority = parse_pubkey(&authority)?;
            let response = client.get_traders(&authority).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::CollateralHistory {
            authority,
            pda_index,
            limit,
            next_cursor,
            prev_cursor,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = CollateralHistoryQueryParams::new(limit).with_pda_index(pda_index);
            if let Some(value) = next_cursor {
                params = params.with_next_cursor(value);
            }
            if let Some(value) = prev_cursor {
                params = params.with_prev_cursor(value);
            }
            if let Some(value) = cursor {
                params.request.cursor = Some(value);
            }
            let response = client.get_collateral_history(&authority, params).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::FundingHistory {
            authority,
            pda_index,
            symbol,
            start_time,
            end_time,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = FundingHistoryQueryParams::new().with_pda_index(pda_index);
            if let Some(value) = symbol {
                params = params.with_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            let response = client.get_funding_history(&authority, params).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::OrderHistory {
            authority,
            limit,
            trader_pda_index,
            market_symbol,
            cursor,
            privy_id,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = OrderHistoryQueryParams::new(limit);
            if let Some(value) = trader_pda_index {
                params = params.with_pda_index(value);
            }
            if let Some(value) = market_symbol {
                params = params.with_market_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            if let Some(value) = privy_id {
                params = params.with_privy_id(value);
            }
            let response = client.get_order_history(&authority, params).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::Candles {
            symbol,
            timeframe,
            start_time,
            end_time,
            limit,
        } => {
            let timeframe = Timeframe::from_str(&timeframe)
                .map_err(|e| format!("invalid timeframe '{timeframe}': {e}"))?;
            let mut params = CandlesQueryParams::new(symbol.to_ascii_uppercase(), timeframe);
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            let response = client.get_candles(params).await?;
            print_json(&response, pretty)?;
        }
        HttpCommand::TradeHistory {
            authority,
            pda_index,
            market_symbol,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = TradeHistoryQueryParams::new().with_pda_index(pda_index);
            if let Some(value) = market_symbol {
                params = params.with_market_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            let response = client.get_trade_history(&authority, params).await?;
            print_json(&response, pretty)?;
        }
    }

    Ok(())
}

async fn run_ws(cmd: WsCommand, env: &PhoenixEnv, pretty: bool) -> Result<(), Box<dyn Error>> {
    let client = PhoenixWSClient::from_env_with_connection_status(env.clone())?;

    match cmd {
        WsCommand::AllMids { timeout_secs } => {
            let (mut rx, _handle) = client.subscribe_to_all_mids()?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "allMids").await?;
            print_json(&ServerMessage::AllMids(message), pretty)?;
        }
        WsCommand::FundingRate {
            symbol,
            timeout_secs,
        } => {
            let (mut rx, _handle) =
                client.subscribe_to_funding_rate(symbol.to_ascii_uppercase())?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "fundingRate").await?;
            print_json(&ServerMessage::FundingRate(message), pretty)?;
        }
        WsCommand::Orderbook {
            symbol,
            timeout_secs,
        } => {
            let (mut rx, _handle) = client.subscribe_to_orderbook(symbol.to_ascii_uppercase())?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "orderbook").await?;
            print_json(&ServerMessage::Orderbook(message), pretty)?;
        }
        WsCommand::Market {
            symbol,
            timeout_secs,
        } => {
            let (mut rx, _handle) = client.subscribe_to_market(symbol.to_ascii_uppercase())?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "market").await?;
            print_json(&ServerMessage::Market(message), pretty)?;
        }
        WsCommand::Trades {
            symbol,
            timeout_secs,
        } => {
            let (mut rx, _handle) = client.subscribe_to_trades(symbol.to_ascii_uppercase())?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "trades").await?;
            print_json(&ServerMessage::Trades(message), pretty)?;
        }
        WsCommand::Candles {
            symbol,
            timeframe,
            timeout_secs,
        } => {
            let timeframe = Timeframe::from_str(&timeframe)
                .map_err(|e| format!("invalid timeframe '{timeframe}': {e}"))?;
            let (mut rx, _handle) =
                client.subscribe_to_candles(symbol.to_ascii_uppercase(), timeframe)?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "candles").await?;
            print_json(&ServerMessage::Candles(message), pretty)?;
        }
        WsCommand::TraderState {
            authority,
            trader_pda_index,
            timeout_secs,
        } => {
            let authority = parse_pubkey(&authority)?;
            let (mut rx, _handle) =
                client.subscribe_to_trader_state_with_pda(&authority, trader_pda_index)?;
            let message = recv_or_timeout(&mut rx, timeout_secs, "traderState").await?;
            print_json(&ServerMessage::TraderState(message), pretty)?;
        }
    }

    Ok(())
}

async fn recv_or_timeout<T>(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<T>,
    timeout_secs: u64,
    label: &str,
) -> Result<T, Box<dyn Error>> {
    match timeout(Duration::from_secs(timeout_secs), rx.recv()).await {
        Ok(Some(message)) => Ok(message),
        Ok(None) => Err(format!("{label} channel closed before first message").into()),
        Err(_) => Err(format!("timed out waiting for first {label} message").into()),
    }
}

fn parse_pubkey(input: &str) -> Result<Pubkey, Box<dyn Error>> {
    Pubkey::from_str(input).map_err(|e| format!("invalid pubkey '{input}': {e}").into())
}

fn print_json<T: Serialize>(value: &T, pretty: bool) -> Result<(), Box<dyn Error>> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}
