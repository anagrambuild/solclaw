# phoenix-sdk Guide

## Architecture Overview

The SDK has three client layers, each building on the last:

```
PhoenixClient  (high-level, stateful, auto-reconnect, receiver-based)
    |
    +-- PhoenixWSClient   (low-level WS, no reconnect)
    +-- PhoenixHttpClient (REST API)
```

Most consumers should use **`PhoenixClient`** directly.

---

## PhoenixHttpClient (`http_client.rs`)

Stateless REST client for the Phoenix perpetuals API. Methods include:

- `get_exchange()` / `get_exchange_keys()` / `get_markets()` / `get_market(symbol)`
- `get_traders(authority)` — fetch trader state via HTTP
- `get_collateral_history(...)` / `get_funding_history(...)` / `get_order_history(...)` / `get_trade_history(...)`
- `get_candles(symbol, timeframe, ...)`
- `build_isolated_limit_order_tx(...)` / `build_isolated_market_order_tx(...)` — server-side isolated order construction

Constructed via `PhoenixHttpClient::new_from_env()` or `PhoenixHttpClient::from_env(env)`. Reads `PHOENIX_API_URL` and optional `PHOENIX_API_KEY` from environment.

Also accessible from `PhoenixClient` via `client.http()`.

## PhoenixWSClient (`ws_client.rs`)

Low-level WebSocket client. Handles connection, message parsing, and fan-out to subscribers. **Does not manage reconnection**.

Subscribe methods return `(UnboundedReceiver<T>, SubscriptionHandle)`. Drop the handle to unsubscribe.

`SubscriptionKey` is public and canonical for identifying channels:

- `SubscriptionKey::market(symbol)`
- `SubscriptionKey::orderbook(symbol)`
- `SubscriptionKey::trader(&authority, trader_pda_index)`
- `SubscriptionKey::funding_rate(symbol)`
- `SubscriptionKey::candles(symbol, timeframe)`
- `SubscriptionKey::trades(symbol)`
- `SubscriptionKey::all_mids()`

Most consumers should **not** use `PhoenixWSClient` directly — use `PhoenixClient` instead.

---

## PhoenixClient (`client.rs`)

The primary interface. Wraps both WS and HTTP clients, providing:

1. **Automatic reconnection** with exponential backoff
2. **Receiver-based subscriptions** (`subscribe(...)`) instead of callbacks
3. **Lock-free live state ownership** inside one background task
4. **Dependency-aware subscription refcounting** so transitive dependencies are not dropped early
5. **Resubscription** after reconnect for all active dependency keys

### Construction

```rust
let client = PhoenixClient::new_from_env().await?;
// or
let client = PhoenixClient::from_env(env).await?;
```

On construction, the client fetches exchange metadata via HTTP and spawns a background connection loop.

### Subscribe API

The high-level entry point is:

```rust
let (mut rx, _handle) = client.subscribe(subscription).await?;
```

Drop `_handle` to unsubscribe that logical subscription.

Supported subscriptions:

- `PhoenixSubscription::Key(SubscriptionKey)`
- `PhoenixSubscription::Market { symbol, candle_timeframes, include_trades }`
- `PhoenixSubscription::TraderMargin { authority, trader_pda_index, subaccount_index, market_symbols }`

### Event Model

Receivers yield `PhoenixClientEvent`, including previous state snapshots before each update is applied:

- `MarketUpdate { prev_market, update }`
- `OrderbookUpdate { prev_market, update }`
- `TraderUpdate { prev_trader, update }`
- `MidsUpdate { prev_mids, update }`
- `FundingRateUpdate { prev_funding_rate, update }`
- `CandleUpdate { prev_candle, update }`
- `TradesUpdate { prev_trades, update }`
- `MarginUpdate { trigger, margin, metadata, prev_trader }`

`MarginUpdate` uses `MarginTrigger::{Market, Trader}` and emits recomputation results for trader-margin subscriptions.

### Lifecycle

```rust
tokio::signal::ctrl_c().await?;
client.shutdown();
client.run().await;
```

### Internal Design

The background task owns all mutable runtime state (metadata, markets, traders, mids, funding, candles, trades).

It tracks:

- logical subscriptions (user-facing subscribe handles)
- concrete `SubscriptionKey` dependencies
- dependency refcounts
- live WS `SubscriptionHandle`s and per-key receiver streams

Incoming WS streams are raced via a keyed receiver collection (`StreamMap`) in `tokio::select!`.
