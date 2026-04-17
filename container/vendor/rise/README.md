# Rise - the Phoenix Perps SDK

SDK for the Phoenix perpetuals exchange on Solana. Fetch real-time market and trader data via websocket and HTTP and place orders via RPC.

## Features

- **Real-time WebSocket subscriptions** - L2 orderbook, market stats, candles, trader state, fills
- **Unified PhoenixClient** - Auto-reconnect, dependency-aware subscriptions, and receiver-based events
- **Local state management** - Automatic snapshot/delta reconciliation with sequence ordering
- **Order execution** - Market orders, limit orders, and cancellations via Solana RPC (cross-margin and isolated margin)
- **Type-safe** - Strongly typed message routing and state containers

## Architecture

```
rust/
├── sdk/      phoenix-sdk        High-level client, WebSocket/HTTP
├── types/    phoenix-types      Wire format + shared client model types
├── math/     phoenix-math-utils Margin calculations, risk, fixed-point math
├── ix/       phoenix-ix         Solana instruction builders for order placement
└── cli/      phoenix-sdk-cli    Smoke-test CLI for HTTP + WebSocket
```

## Quick Start

### Subscribe to L2 Orderbook

```rust
use phoenix_sdk::{PhoenixWSClient, L2Book};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uses optional PHOENIX_WS_URL / PHOENIX_API_URL and PHOENIX_API_KEY env vars
    let client = PhoenixWSClient::new_from_env()?;
    let (mut rx, _handle) = client.subscribe_to_orderbook("SOL".into())?;
    let mut book = L2Book::new("SOL".into());

    while let Some(msg) = rx.recv().await {
        book.apply_update(&msg);
        println!("Best bid: {:?}, Best ask: {:?}", book.best_bid(), book.best_ask());
    }
    Ok(())
}
```

### Subscribe to Trader State

```rust
use phoenix_sdk::{PhoenixWSClient, Trader, TraderKey};
use solana_pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uses optional PHOENIX_WS_URL / PHOENIX_API_URL and PHOENIX_API_KEY env vars
    let client = PhoenixWSClient::new_from_env()?;

    let authority = Pubkey::from_str("YOUR_AUTHORITY_PUBKEY")?;
    let key = TraderKey::from_authority(authority);
    let mut trader = Trader::new(key.clone());
    let (mut rx, _handle) = client.subscribe_to_trader_state(&authority)?;

    while let Some(msg) = rx.recv().await {
        trader.apply_update(&msg);
        println!("Collateral: {}", trader.total_collateral());
        for pos in trader.all_positions() {
            println!("  {} {} lots @ {}", pos.symbol, pos.base_position_lots, pos.entry_price_usd);
        }
    }
    Ok(())
}
```

### Unified PhoenixClient Subscription

```rust
use phoenix_sdk::{PhoenixClient, PhoenixSubscription, PhoenixClientEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PhoenixClient::new_from_env().await?;
    let (mut rx, _handle) = client
        .subscribe(PhoenixSubscription::market("SOL")).await?;

    while let Some(event) = rx.recv().await {
        if let PhoenixClientEvent::MarketUpdate { symbol, update, .. } = event {
            println!("{} mark={}", symbol, update.mark_price);
        }
    }

    Ok(())
}
```

### Place Orders

```rust
use phoenix_sdk::{PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey, Side};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = read_keypair_file("~/.config/solana/id.json")?;
    let trader = TraderKey::new(keypair.pubkey());

    // Fetch exchange metadata (uses optional PHOENIX_API_URL/PHOENIX_API_KEY env vars)
    let http = PhoenixHttpClient::new_from_env();
    let metadata = PhoenixMetadata::new(http.get_exchange().await?.into());
    let builder = PhoenixTxBuilder::new(&metadata);

    // Build market order instructions
    let ixs = builder.build_market_order(
        trader.authority(), trader.pda(), "SOL", Side::Bid, 100,
    )?;

    // Send via Solana RPC
    let rpc = RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com".into(),
        CommitmentConfig::confirmed(),
    );
    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &ixs, Some(&keypair.pubkey()), &[&keypair], blockhash,
    );
    let sig = rpc.send_and_confirm_transaction(&tx).await?;

    Ok(())
}
```

### Trader registration

Register a trader with the SDK, through the UI, or with the following curl command:

`curl -sS -X POST 'https://perp-api.phoenix.trade/v1/invite/activate' -H "Content-Type: application/json" -d '{"authority":"'"$PUBKEY"'","code": "'"$CODE"'"}'`

## Crates

### phoenix-sdk

Main SDK with WebSocket client and state containers:

| Type | Description |
|------|-------------|
| `PhoenixWSClient` | WebSocket connection with subscription management |
| `Trader` | Tracks positions, orders, splines, and collateral across subaccounts |
| `L2Book` | Orderbook state with bid/ask accessors, spread, and liquidity metrics |
| `MarketStats` | Mark price, oracle price, funding rates, volume |
| `Market` | Combined L2Book + MarketStats container |
| `PhoenixTxBuilder` | Transaction builder for orders (cross and isolated margin), deposits, withdrawals |
| `PhoenixHttpClient` | REST API for exchange configuration |

### phoenix-types

Serde types matching the Phoenix WebSocket wire format, plus shared generic
client model types used by `phoenix-sdk` (`PhoenixSubscription`,
`PhoenixClientEvent`, `MarginTrigger`, and related command/handle types).

- `ServerMessage` - Enum of all server message types
- `TraderStateServerMessage` - Snapshots and deltas for trader state
- `L2BookUpdate`, `MarketStatsUpdate`, `CandleData`, `FillsMessage`
- `PhoenixSubscription`, `PhoenixClientEvent`, `MarginTrigger`
- Subscription request types

### phoenix-ix

Solana instruction builders for the Phoenix program (cross-margin and isolated margin):

- `create_place_limit_order_ix` - Build limit order instructions
- `create_place_market_order_ix` - Build market order instructions
- `create_cancel_orders_by_id_ix` - Build cancel instructions
- `create_place_stop_loss_ix` - Build stop-loss order instructions
- `create_register_trader_ix` - Register trader subaccounts
- `create_transfer_collateral_ix` - Transfer collateral between cross and isolated subaccounts
- `create_sync_parent_to_child_ix` - Sync parent state to isolated child subaccounts

## Examples

Run from the `rust/` directory:

```bash
# Optional environment variables
export PHOENIX_API_URL=https://public-api.phoenix.trade
export PHOENIX_WS_URL=wss://public-api.phoenix.trade/ws
export PHOENIX_API_KEY=your_api_key

# Trader state updates
cargo run -p phoenix-sdk --example subscribe_trader_state

# L2 orderbook
cargo run -p phoenix-sdk --example subscribe_l2_book -- SOL

# Market stats
cargo run -p phoenix-sdk --example subscribe_market_stats -- SOL

# Candles (1s, 5s, 1m, 5m, 15m, 30m, 1h, 4h, 1d)
cargo run -p phoenix-sdk --example subscribe_candles -- SOL 1m

# Trade events
cargo run -p phoenix-sdk --example subscribe_trades -- SOL

# Compute trader margin
cargo run -p phoenix-sdk --example compute_trader_margin

# HTTP client usage
cargo run -p phoenix-sdk --example http_client

# Isolated margin market order (client-side)
cargo run -p phoenix-sdk --example isolated_market_order_client

# Isolated margin market order (server-side)
cargo run -p phoenix-sdk --example isolated_market_order_server

# Isolated margin limit order
cargo run -p phoenix-sdk --example isolated_limit_order

# Register trader subaccount
cargo run -p phoenix-sdk --example register_trader

# Market maker example
cargo run -p phoenix-sdk --example market_maker

# WebSocket debug CLI
cargo run -p phoenix-sdk --example ws_debug_cli
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `PHOENIX_API_URL` | Optional Phoenix REST API URL (defaults to `https://public-api.phoenix.trade`) |
| `PHOENIX_WS_URL` | Optional Phoenix WebSocket URL (defaults to `wss://public-api.phoenix.trade/ws`) |
| `PHOENIX_API_KEY` | Optional Phoenix API key (sent as `x-api-key` when set) |

## Build

```bash
cd rust
cargo build
cargo test
```

Requires Rust 1.86.0+.
