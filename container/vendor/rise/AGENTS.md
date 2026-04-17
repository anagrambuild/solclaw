# phoenix-sdk

SDK for the Phoenix perpetuals exchange on Solana.

## Directory Structure

```
rust/
├── Cargo.toml              # Workspace manifest
├── cli/                    # phoenix-sdk-cli crate (smoke-test CLI)
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs         # Clap-based CLI for HTTP + WebSocket smoke testing
│   └── scripts/
│       └── smoke_http_client.sh
├── ix/                     # phoenix-ix crate (instruction builders)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Crate root, re-exports instruction builders
│       ├── constants.rs    # Program IDs, discriminants, PDA derivation
│       ├── types.rs        # AccountMeta, Instruction, Side, OrderFlags, IsolatedCollateralFlow, etc.
│       ├── error.rs        # PhoenixIxError enum
│       ├── limit_order.rs  # Limit order instruction builder (cross and isolated)
│       ├── market_order.rs # Market order instruction builder (cross and isolated)
│       ├── order_packet.rs # OrderPacket serialization for on-chain instruction data
│       ├── cancel_orders.rs # Cancel orders instruction builder
│       ├── deposit_funds.rs # Deposit funds instruction builder
│       ├── withdraw_funds.rs # Withdraw funds instruction builder
│       ├── register_trader.rs # Register trader (subaccount) instruction builder
│       ├── stop_loss.rs    # Stop-loss order instruction builder
│       ├── transfer_collateral.rs # Cross-to-isolated and child-to-parent collateral transfers
│       ├── sync_parent_to_child.rs # Sync parent trader state to isolated child subaccount
│       ├── ember_deposit.rs # Ember USDC->Phoenix token deposit
│       ├── ember_withdraw.rs # Ember Phoenix token->USDC withdraw
│       ├── spl_approve.rs  # SPL Token approve instruction builder
│       └── create_ata.rs   # Idempotent ATA creation instruction
├── math/                   # phoenix-math-utils crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Crate root, re-exports math utilities
│       ├── direction.rs    # Direction and stop-loss order types
│       ├── errors.rs       # Application-level error types
│       ├── fixed.rs        # I80F48 fixed-point arithmetic wrapper
│       ├── funding.rs      # Funding rate calculations
│       ├── leverage_tiers.rs # Position-size-dependent margin requirements
│       ├── limit_order_state.rs # Limit order margin state aggregation
│       ├── margin.rs       # Per-market margin computation
│       ├── margin_calc.rs  # Core margin calculation formulas
│       ├── market_math.rs  # MarketCalculator for price/lot conversions
│       ├── perp_metadata.rs # Simplified perpetual asset metadata
│       ├── portfolio.rs    # Portfolio-level aggregation across markets
│       ├── price.rs        # Price quantization and tick conversions
│       ├── risk.rs         # Risk assessment types and margin state
│       ├── trader_position.rs # Trader position in a perp market
│       └── quantities/     # Type-safe quantity system (BaseLots, QuoteLots, Ticks, etc.)
├── sdk/                    # phoenix-sdk crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs          # Crate root, re-exports main types
│   │   ├── client.rs       # PhoenixClient unified client with reconnection and callbacks
│   │   ├── env.rs          # Environment configuration with defaults
│   │   ├── http_client.rs  # HTTP client for REST API (markets, traders, candles)
│   │   ├── tx_builder.rs   # Transaction builder for orders and deposits
│   │   └── ws_client.rs    # WebSocket client with auto-reconnect
│   ├── examples/
│   │   ├── phoenix_client.rs
│   │   ├── subscribe_trader_state.rs
│   │   ├── subscribe_market_stats.rs
│   │   ├── subscribe_l2_book.rs
│   │   ├── subscribe_candles.rs
│   │   ├── subscribe_trades.rs
│   │   ├── send_market_order.rs
│   │   ├── send_limit_order.rs
│   │   ├── isolated_market_order_client.rs  # Isolated margin market order (client-side construction)
│   │   ├── isolated_market_order_server.rs  # Isolated margin market order (server-side HTTP endpoint)
│   │   ├── isolated_limit_order.rs          # Isolated margin limit order
│   │   ├── cancel_order.rs
│   │   ├── deposit_funds.rs
│   │   ├── register_trader.rs
│   │   ├── compute_trader_margin.rs
│   │   ├── http_client.rs
│   │   ├── market_maker.rs
│   │   └── ws_debug_cli.rs
│   └── tests/
│       └── trader_state_tests.rs
└── types/                  # phoenix-types crate
    ├── Cargo.toml
    └── src/
        ├── lib.rs          # Crate root, re-exports all types
        ├── candles.rs      # Candle types (Timeframe, ApiCandle, CandleData)
        ├── client.rs       # Client-side types for higher-level SDK clients
        ├── conversions.rs  # Conversion utilities for building margin calc types
        ├── core.rs         # Core primitives (Decimal, Price, Side, PaginatedResponse)
        ├── exchange.rs     # Exchange keys and configuration
        ├── http_error.rs   # HTTP error types
        ├── js_safe_ints.rs # Big integers serialized as strings for JS compatibility
        ├── l2book.rs       # L2 orderbook state container
        ├── market.rs       # Market config, status, orderbook, statistics
        ├── market_state.rs # Combined market state (statistics + orderbook)
        ├── market_stats.rs # Market statistics state container
        ├── metadata.rs     # Exchange metadata caching
        ├── subscription_key.rs # Subscription key for message routing
        ├── trader.rs       # WebSocket protocol types (snapshots, deltas, capabilities)
        ├── trader_http.rs  # HTTP API types (TraderView, order/collateral/funding history)
        ├── trader_key.rs   # TraderKey identification and PDA derivation
        ├── trader_state.rs # Trader state container with snapshot/delta handling
        ├── ix.rs           # Server-side instruction request types (PlaceIsolatedLimitOrderRequest, PlaceIsolatedMarketOrderRequest)
        ├── trades.rs       # Trade event records
        ├── ws.rs           # WebSocket protocol types (subscriptions, client/server messages)
        └── ws_error.rs     # WebSocket error types
```

## Build Commands

All commands run from `rust/` directory:

```bash
cargo build              # Build both crates
cargo test               # Run all tests

# Examples optionally use environment variables:
# PHOENIX_API_URL=https://perp-api.phoenix.trade (optional; for HTTP/RPC and WS derivation)
# PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws (optional; overrides derived URL)
# PHOENIX_API_KEY=your_api_key (optional; sent as x-api-key when set)

cargo run -p phoenix-sdk --example subscribe_trader_state
cargo run -p phoenix-sdk --example subscribe_l2_book -- SOL
cargo run -p phoenix-sdk --example subscribe_candles -- SOL-PERP 1m
```

## Crates

### phoenix-ix

Solana instruction builders for Phoenix perpetuals exchange. Supports both cross-margin and isolated margin orders:
- **constants** - Program IDs (Phoenix, Ember, SPL Token), instruction discriminants, PDA derivation functions
- **limit_order** - `LimitOrderParams` / `IsolatedLimitOrderParams` builders and `create_place_limit_order_ix` function
- **market_order** - `MarketOrderParams` / `IsolatedMarketOrderParams` builders and `create_place_market_order_ix` function
- **order_packet** - `OrderPacket` serialization for on-chain instruction data
- **cancel_orders** - `CancelOrdersByIdParams` builder and `create_cancel_orders_by_id_ix` function
- **deposit_funds** - `DepositFundsParams` builder and `create_deposit_funds_ix` function for depositing Phoenix tokens into the protocol
- **withdraw_funds** - `WithdrawFundsParams` builder and `create_withdraw_funds_ix` function for withdrawing Phoenix tokens from the protocol
- **register_trader** - `RegisterTraderParams` builder and `create_register_trader_ix` function for registering trader subaccounts
- **stop_loss** - `StopLossParams` builder and `create_place_stop_loss_ix` function
- **transfer_collateral** - `TransferCollateralParams` / `TransferCollateralChildToParentParams` for moving collateral between cross-margin and isolated subaccounts
- **sync_parent_to_child** - `SyncParentToChildParams` and `create_sync_parent_to_child_ix` for syncing parent state to isolated child subaccounts
- **ember_deposit** - `EmberDepositParams` builder and `create_ember_deposit_ix` function for converting USDC to Phoenix tokens
- **ember_withdraw** - `EmberWithdrawParams` builder and `create_ember_withdraw_ix` function for converting Phoenix tokens to USDC
- **spl_approve** - `SplApproveParams` builder and `create_spl_approve_ix` function for SPL Token approve delegation
- **create_ata** - `create_associated_token_account_idempotent_ix` for creating ATAs

### phoenix-math-utils

Type-safe math utilities for the Phoenix perpetuals exchange:
- **fixed** - `I80F48` fixed-point arithmetic wrapper around the `fixed` crate
- **funding** - Funding rate calculations and conversions
- **market_math** - `MarketCalculator` for converting between prices, ticks, base lots, and quote lots
- **price** - Price quantization and tick conversion utilities
- **quantities** - Type-safe newtype wrappers (`BaseLots`, `QuoteLots`, `Ticks`, etc.) preventing arithmetic errors at compile time
- **direction** - Direction and stop-loss order types for price comparisons
- **errors** - Application-level error types
- **leverage_tiers** - Leverage tiers for position-size-dependent margin requirements
- **limit_order_state** - Limit order margin state aggregation for margin calculations
- **margin** - Core margin types and per-market margin computation
- **margin_calc** - Core margin calculation formulas for perpetual futures positions
- **perp_metadata** - Simplified perpetual asset metadata for margin calculations
- **portfolio** - Portfolio-level types and aggregation across multiple markets
- **risk** - Risk assessment types and margin state
- **trader_position** - `TraderPosition` representing a trader's position in a perp market

### phoenix-types

Minimal serde types matching the Phoenix API wire formats. No runtime dependencies beyond serde.
- **core** - Fundamental primitives (`Decimal`, `Price`, `Side`, `PaginatedResponse`)
- **trader** - WebSocket protocol types for real-time state synchronization (snapshots, deltas, capabilities)
- **trader_http** - HTTP API types for views and history (`TraderView`, `OrderHistoryItem`, `CollateralEvent`, `FundingHistoryEvent`)
- **market** - Market configuration, status enums, orderbook, and statistics
- **exchange** - Exchange keys and authority configuration
- **candles** - Candlestick (OHLCV) data types
- **trades** - Trade event records for WebSocket and HTTP
- **ws** - WebSocket protocol (subscriptions, client/server message envelopes)
- **client** - Client-side types for higher-level SDK clients (`PhoenixSubscription`, `PhoenixClientEvent`, `MarginTrigger`)
- **conversions** - Conversion utilities for building margin calculation types from HTTP/WebSocket data
- **ix** - Server-side instruction request types (`PlaceIsolatedLimitOrderRequest`, `PlaceIsolatedMarketOrderRequest`, `TpSlOrderConfig`)
- **http_error** - HTTP error types for the Phoenix SDK
- **js_safe_ints** - Safe big integers that serialize as strings for JSON/JavaScript compatibility
- **l2book** - L2 orderbook state container for Phoenix markets
- **market_state** - Combined market state container (statistics + orderbook)
- **market_stats** - Market statistics state container
- **metadata** - Exchange metadata caching for the SDK
- **subscription_key** - Subscription key for routing messages to the correct subscriber
- **trader_key** - `TraderKey` identification and PDA derivation
- **trader_state** - Trader state container with snapshot and delta handling
- **ws_error** - WebSocket error types (`PhoenixWsError`)

### phoenix-sdk

- **client** - `PhoenixClient` unified client wrapping WS and HTTP clients with automatic reconnection, lock-free single-owner runtime state, receiver-based `subscribe(...)` API (`PhoenixSubscription`), dependency-aware unsubscribe, and composite subscriptions (including market bundles and trader margin updates)
- **env** - `PhoenixEnv` environment configuration loading with defaults for API URL, WebSocket URL, and API key
- **http_client** - `PhoenixHttpClient` for REST API calls (exchange config, markets, traders, candles, collateral history, funding history); also provides `build_isolated_limit_order_tx` and `build_isolated_market_order_tx` for server-side isolated order construction
- **tx_builder** - `PhoenixTxBuilder` builds Solana instructions from `PhoenixMetadata`; provides `build_market_order`, `build_limit_order`, `build_cancel_orders`, `build_deposit_funds`, `build_withdraw_funds` for cross-margin, and `build_isolated_market_order`, `build_isolated_limit_order` for isolated margin (with subaccount registration, collateral transfer, and sync)
- **ws_client** - `PhoenixWSClient` handles WebSocket connection, auto-reconnect with exponential backoff, and message routing to subscribers; `SubscriptionHandle` returned from subscribe methods enables unsubscription by dropping

### phoenix-sdk-cli

Clap-based smoke-test CLI for exercising the HTTP and WebSocket clients. Supports all HTTP endpoints and WebSocket subscriptions via subcommands.

## Additional Agent Docs

- [`rust/sdk/AGENTS.md`](./rust/sdk/AGENTS.md) — Architecture guide for `PhoenixClient`, `PhoenixWSClient`, and `PhoenixHttpClient`. Covers the three-layer client hierarchy, callback-based subscription patterns, cached state getters, lifecycle management, and internal design (command channels, `SubscriptionHandles`, `AggChannels`).
