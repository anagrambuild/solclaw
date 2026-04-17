//! Market command execution.

use crate::cli::market::MarketCommand;
use crate::context::AppContext;
use crate::error::VulcanError;
use crate::output::{render_success, TableRenderable};
use serde::Serialize;

// ── Result types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MarketListResult {
    pub markets: Vec<MarketSummary>,
}

#[derive(Debug, Serialize)]
pub struct MarketSummary {
    pub symbol: String,
    pub status: String,
    pub taker_fee: f64,
    pub maker_fee: f64,
    pub max_leverage: f64,
    pub isolated_only: bool,
}

impl TableRenderable for MarketListResult {
    fn render_table(&self) {
        if self.markets.is_empty() {
            println!("No markets found.");
            return;
        }
        let rows: Vec<Vec<String>> = self
            .markets
            .iter()
            .map(|m| {
                vec![
                    m.symbol.clone(),
                    m.status.clone(),
                    format!("{:.2}%", m.taker_fee * 100.0),
                    format!("{:.2}%", m.maker_fee * 100.0),
                    format!("{:.0}x", m.max_leverage),
                    if m.isolated_only {
                        "yes".into()
                    } else {
                        "no".into()
                    },
                ]
            })
            .collect();
        crate::output::table::render_table(
            &[
                "Symbol",
                "Status",
                "Taker Fee",
                "Maker Fee",
                "Max Leverage",
                "Isolated Only",
            ],
            rows,
        );
    }
}

#[derive(Debug, Serialize)]
pub struct MarketInfoResult {
    pub symbol: String,
    pub status: String,
    pub market_pubkey: String,
    pub tick_size: u64,
    pub base_lots_decimals: i8,
    pub taker_fee: f64,
    pub maker_fee: f64,
    pub funding_interval_seconds: u32,
    pub funding_period_seconds: u32,
    pub max_funding_rate_per_interval: f64,
    pub isolated_only: bool,
    pub leverage_tiers: Vec<LeverageTierInfo>,
}

#[derive(Debug, Serialize)]
pub struct LeverageTierInfo {
    pub max_leverage: f64,
    pub max_size_base_lots: u64,
}

impl TableRenderable for MarketInfoResult {
    fn render_table(&self) {
        println!("Market: {}", self.symbol);
        println!("Status: {}", self.status);
        println!("Pubkey: {}", self.market_pubkey);
        println!("Tick size: {}", self.tick_size);
        println!("Base lots decimals: {}", self.base_lots_decimals);
        println!("Taker fee: {:.4}%", self.taker_fee * 100.0);
        println!("Maker fee: {:.4}%", self.maker_fee * 100.0);
        println!(
            "Funding: every {}s / period {}s",
            self.funding_interval_seconds, self.funding_period_seconds
        );
        println!(
            "Max funding rate/interval: {:.6}%",
            self.max_funding_rate_per_interval * 100.0
        );
        println!("Isolated only: {}", self.isolated_only);
        if !self.leverage_tiers.is_empty() {
            println!("\nLeverage tiers:");
            let rows: Vec<Vec<String>> = self
                .leverage_tiers
                .iter()
                .map(|t| {
                    vec![
                        format!("{:.0}x", t.max_leverage),
                        t.max_size_base_lots.to_string(),
                    ]
                })
                .collect();
            crate::output::table::render_table(&["Max Leverage", "Max Size (base lots)"], rows);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TickerResult {
    pub symbol: String,
    pub mark_price: f64,
    pub mid_price: f64,
    pub oracle_price: f64,
    pub prev_day_price: f64,
    pub change_24h_pct: f64,
    pub volume_24h_usd: f64,
    pub open_interest: f64,
    pub funding_rate: f64,
}

impl TableRenderable for TickerResult {
    fn render_table(&self) {
        let rows = vec![vec![
            self.symbol.clone(),
            format!("${:.2}", self.mark_price),
            format!("{:+.2}%", self.change_24h_pct),
            format!("${:.0}", self.volume_24h_usd),
            format!("{:.0}", self.open_interest),
            format!("{:+.4}%", self.funding_rate * 100.0),
        ]];
        crate::output::table::render_table(
            &[
                "Symbol",
                "Mark Price",
                "24h Change",
                "24h Volume",
                "Open Interest",
                "Funding Rate",
            ],
            rows,
        );
    }
}

#[derive(Debug, Serialize)]
pub struct CandlesResult {
    pub symbol: String,
    pub interval: String,
    pub candles: Vec<CandleRow>,
}

#[derive(Debug, Serialize)]
pub struct CandleRow {
    pub time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

impl TableRenderable for CandlesResult {
    fn render_table(&self) {
        let rows: Vec<Vec<String>> = self
            .candles
            .iter()
            .map(|c| {
                vec![
                    c.time.clone(),
                    format!("{:.2}", c.open),
                    format!("{:.2}", c.high),
                    format!("{:.2}", c.low),
                    format!("{:.2}", c.close),
                    c.volume.map_or("-".into(), |v| format!("{:.0}", v)),
                ]
            })
            .collect();
        crate::output::table::render_table(
            &["Time", "Open", "High", "Low", "Close", "Volume"],
            rows,
        );
    }
}

#[derive(Debug, Serialize)]
pub struct OrderbookResult {
    pub symbol: String,
    pub mid_price: Option<f64>,
    pub spread: Option<f64>,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
}

#[derive(Debug, Serialize)]
pub struct OrderbookLevel {
    pub price: f64,
    pub quantity: f64,
}

impl TableRenderable for OrderbookResult {
    fn render_table(&self) {
        if self.bids.is_empty() && self.asks.is_empty() {
            println!("Orderbook is empty for {}.", self.symbol);
            return;
        }

        if let (Some(mid), Some(spread)) = (self.mid_price, self.spread) {
            let spread_bps = if mid > 0.0 {
                (spread / mid) * 10000.0
            } else {
                0.0
            };
            println!(
                "{} — mid: {:.4}  spread: {:.1}bps\n",
                self.symbol, mid, spread_bps
            );
        }

        println!("  Asks:");
        let ask_rows: Vec<Vec<String>> = self
            .asks
            .iter()
            .rev()
            .map(|l| vec![format!("{:.4}", l.price), format!("{:.4}", l.quantity)])
            .collect();
        crate::output::table::render_table(&["Price", "Quantity"], ask_rows);

        println!("\n  Bids:");
        let bid_rows: Vec<Vec<String>> = self
            .bids
            .iter()
            .map(|l| vec![format!("{:.4}", l.price), format!("{:.4}", l.quantity)])
            .collect();
        crate::output::table::render_table(&["Price", "Quantity"], bid_rows);
    }
}

// ── Execution ───────────────────────────────────────────────────────────

pub async fn execute_list_inner(ctx: &AppContext) -> Result<MarketListResult, VulcanError> {
    let markets = ctx
        .http_client
        .get_markets()
        .await
        .map_err(|e| VulcanError::api("MARKETS_FETCH_FAILED", e.to_string()))?;

    Ok(MarketListResult {
        markets: markets
            .iter()
            .map(|m| MarketSummary {
                symbol: m.symbol.clone(),
                status: format!("{:?}", m.market_status),
                taker_fee: m.taker_fee,
                maker_fee: m.maker_fee,
                max_leverage: m
                    .leverage_tiers
                    .first()
                    .map(|t| t.max_leverage)
                    .unwrap_or(1.0),
                isolated_only: m.isolated_only,
            })
            .collect(),
    })
}

pub async fn execute_info_inner(
    ctx: &AppContext,
    symbol: &str,
) -> Result<MarketInfoResult, VulcanError> {
    let market = ctx
        .http_client
        .get_market(symbol)
        .await
        .map_err(|e| VulcanError::api("MARKET_FETCH_FAILED", e.to_string()))?;

    Ok(MarketInfoResult {
        symbol: market.symbol.clone(),
        status: format!("{:?}", market.market_status),
        market_pubkey: market.market_pubkey.clone(),
        tick_size: market.tick_size,
        base_lots_decimals: market.base_lots_decimals,
        taker_fee: market.taker_fee,
        maker_fee: market.maker_fee,
        funding_interval_seconds: market.funding_interval_seconds,
        funding_period_seconds: market.funding_period_seconds,
        max_funding_rate_per_interval: market.max_funding_rate_per_interval,
        isolated_only: market.isolated_only,
        leverage_tiers: market
            .leverage_tiers
            .iter()
            .map(|t| LeverageTierInfo {
                max_leverage: t.max_leverage,
                max_size_base_lots: t.max_size_base_lots,
            })
            .collect(),
    })
}

pub async fn execute_ticker_inner(
    ctx: &AppContext,
    symbol: &str,
) -> Result<TickerResult, VulcanError> {
    let env = phoenix_sdk::PhoenixEnv {
        api_url: ctx.config.network.api_url.clone(),
        ws_url: {
            let api = &ctx.config.network.api_url;
            let ws = api
                .replace("https://", "wss://")
                .replace("http://", "ws://");
            if ws.ends_with("/ws") {
                ws
            } else {
                format!("{}/ws", ws.trim_end_matches('/'))
            }
        },
        api_key: ctx.config.network.api_key.clone(),
    };

    let client = phoenix_sdk::PhoenixClient::from_env(env)
        .await
        .map_err(|e| VulcanError::network("WS_CONNECT_FAILED", e.to_string()))?;

    let (mut rx, _handle) = client
        .subscribe(phoenix_sdk::PhoenixSubscription::Market {
            symbol: symbol.to_string(),
            candle_timeframes: vec![],
            include_trades: false,
        })
        .await
        .map_err(|e| VulcanError::network("WS_SUBSCRIBE_FAILED", e.to_string()))?;

    let stats = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(event) = rx.recv().await {
            if let phoenix_sdk::PhoenixClientEvent::MarketUpdate { update, .. } = event {
                return Ok(update);
            }
        }
        Err(VulcanError::network(
            "NO_MARKET_DATA",
            "No market stats received",
        ))
    })
    .await
    .map_err(|_| VulcanError::network("TIMEOUT", "Timed out waiting for market data"))??;

    let change_pct = if stats.prev_day_mark_price > 0.0 {
        ((stats.mark_price - stats.prev_day_mark_price) / stats.prev_day_mark_price) * 100.0
    } else {
        0.0
    };

    let result = TickerResult {
        symbol: stats.symbol.clone(),
        mark_price: stats.mark_price,
        mid_price: stats.mid_price,
        oracle_price: stats.oracle_price,
        prev_day_price: stats.prev_day_mark_price,
        change_24h_pct: change_pct,
        volume_24h_usd: stats.day_volume_usd,
        open_interest: stats.open_interest,
        funding_rate: stats.funding_rate,
    };

    client.shutdown();
    Ok(result)
}

pub async fn execute_orderbook_inner(
    ctx: &AppContext,
    symbol: &str,
    depth: usize,
) -> Result<OrderbookResult, VulcanError> {
    let api = &ctx.config.network.api_url;
    let ws_url = {
        let ws = api
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        if ws.ends_with("/ws") {
            ws
        } else {
            format!("{}/ws", ws.trim_end_matches('/'))
        }
    };

    let client = phoenix_sdk::PhoenixWSClient::new(&ws_url, ctx.config.network.api_key.clone())
        .map_err(|e| VulcanError::network("WS_CONNECT_FAILED", e.to_string()))?;

    let (mut rx, _handle) = client
        .subscribe_to_orderbook(symbol.to_string())
        .map_err(|e| VulcanError::network("WS_SUBSCRIBE_FAILED", e.to_string()))?;

    let update = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        match rx.recv().await {
            Some(data) => Ok(data),
            None => Err(VulcanError::network(
                "NO_ORDERBOOK_DATA",
                "No orderbook data received",
            )),
        }
    })
    .await
    .map_err(|_| VulcanError::network("TIMEOUT", "Timed out waiting for orderbook data"))??;

    let book = &update.orderbook;

    let bids: Vec<OrderbookLevel> = book
        .bids
        .iter()
        .take(depth)
        .map(|&(price, qty)| OrderbookLevel {
            price,
            quantity: qty,
        })
        .collect();

    let asks: Vec<OrderbookLevel> = book
        .asks
        .iter()
        .take(depth)
        .map(|&(price, qty)| OrderbookLevel {
            price,
            quantity: qty,
        })
        .collect();

    let spread = match (bids.first(), asks.first()) {
        (Some(b), Some(a)) => Some(a.price - b.price),
        _ => None,
    };

    Ok(OrderbookResult {
        symbol: symbol.to_string(),
        mid_price: book.mid,
        spread,
        bids,
        asks,
    })
}

pub async fn execute_candles_inner(
    ctx: &AppContext,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<CandlesResult, VulcanError> {
    let timeframe: phoenix_sdk::Timeframe = interval
        .parse()
        .map_err(|e: String| VulcanError::validation("INVALID_INTERVAL", e))?;
    let params = phoenix_sdk::CandlesQueryParams::new(symbol, timeframe);
    let candles = ctx
        .http_client
        .get_candles(params)
        .await
        .map_err(|e| VulcanError::api("CANDLES_FETCH_FAILED", e.to_string()))?;

    let candles_to_show: Vec<_> = candles.into_iter().rev().take(limit).rev().collect();

    Ok(CandlesResult {
        symbol: symbol.to_string(),
        interval: interval.to_string(),
        candles: candles_to_show
            .iter()
            .map(|c| {
                let secs = if c.time > 1_000_000_000_000 {
                    c.time / 1000
                } else {
                    c.time
                };
                let dt = chrono::DateTime::from_timestamp(secs, 0)
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| c.time.to_string());
                CandleRow {
                    time: dt,
                    open: c.open,
                    high: c.high,
                    low: c.low,
                    close: c.close,
                    volume: c.volume,
                }
            })
            .collect(),
    })
}

pub async fn execute(ctx: &AppContext, cmd: MarketCommand) -> Result<(), VulcanError> {
    match cmd {
        MarketCommand::List => {
            let result = execute_list_inner(ctx).await?;
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        MarketCommand::Info { symbol } => {
            let result = execute_info_inner(ctx, &symbol).await?;
            render_success(ctx.output_format, &result, serde_json::Value::Null);
            Ok(())
        }

        MarketCommand::Ticker { symbol } => {
            let result = execute_ticker_inner(ctx, &symbol).await?;
            render_success(
                ctx.output_format,
                &result,
                serde_json::json!({ "command": "market ticker", "symbol": symbol }),
            );

            if ctx.watch {
                crate::watch::watch_loop(
                    ctx,
                    crate::watch::WatchKind::Market(symbol.clone()),
                    || {
                        let symbol = symbol.clone();
                        async move {
                            let result = execute_ticker_inner(ctx, &symbol).await?;
                            render_success(
                                ctx.output_format,
                                &result,
                                serde_json::json!({ "command": "market ticker", "symbol": symbol }),
                            );
                            Ok(())
                        }
                    },
                )
                .await?;
            }
            Ok(())
        }

        MarketCommand::Orderbook { symbol, depth } => {
            let result = execute_orderbook_inner(ctx, &symbol, depth).await?;
            render_success(
                ctx.output_format,
                &result,
                serde_json::json!({ "command": "market orderbook", "symbol": symbol, "depth": depth }),
            );

            if ctx.watch {
                crate::watch::watch_loop(ctx, crate::watch::WatchKind::Orderbook(symbol.clone()), || {
                    let symbol = symbol.clone();
                    async move {
                        let result = execute_orderbook_inner(ctx, &symbol, depth).await?;
                        render_success(
                            ctx.output_format,
                            &result,
                            serde_json::json!({ "command": "market orderbook", "symbol": symbol, "depth": depth }),
                        );
                        Ok(())
                    }
                }).await?;
            }
            Ok(())
        }

        MarketCommand::Candles {
            symbol,
            interval,
            limit,
        } => {
            let result = execute_candles_inner(ctx, &symbol, &interval, limit).await?;
            render_success(
                ctx.output_format,
                &result,
                serde_json::json!({ "command": "market candles", "symbol": symbol, "interval": interval }),
            );
            Ok(())
        }

        MarketCommand::Trades { symbol, limit } => {
            let _ = (symbol, limit);
            Err(VulcanError::internal(
                "NOT_IMPLEMENTED",
                "market trades not yet implemented (needs WebSocket trades stream)",
            ))
        }

        MarketCommand::FundingRates { symbol, limit } => {
            let _ = (symbol, limit);
            Err(VulcanError::internal(
                "NOT_IMPLEMENTED",
                "market funding-rates not yet implemented (needs history endpoint with no auth)",
            ))
        }
    }
}
