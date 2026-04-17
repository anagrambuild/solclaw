//! Trade command execution.

use crate::cli::trade::TradeCommand;
use crate::context::AppContext;
use crate::error::VulcanError;
use crate::output::{render_success, TableRenderable};
use phoenix_math_utils::{SignedQuoteLots, WrapperNum};
use phoenix_sdk::types::trader_state::LimitOrder as SdkLimitOrder;
use phoenix_sdk::types::{Position as SdkPosition, SubaccountState, Trader, TraderKey, TraderView};
use phoenix_sdk::IsolatedCollateralFlow;
use phoenix_sdk::Side;
use serde::Serialize;
use solana_pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;

// ── Result types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OrderResult {
    pub action: String,
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub price: Option<f64>,
    pub tp: Option<f64>,
    pub sl: Option<f64>,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
    pub num_instructions: usize,
}

impl TableRenderable for OrderResult {
    fn render_table(&self) {
        if self.dry_run {
            println!("[DRY RUN] Would place {} order:", self.action);
        } else {
            println!("Order placed:");
        }
        println!("  Symbol: {}", self.symbol);
        println!("  Side: {}", self.side);
        println!("  Size: {} base lots", self.size);
        if let Some(p) = self.price {
            println!("  Price: ${:.2}", p);
        }
        if let Some(tp) = self.tp {
            println!("  Take profit: ${:.2}", tp);
        }
        if let Some(sl) = self.sl {
            println!("  Stop loss: ${:.2}", sl);
        }
        println!("  Instructions: {}", self.num_instructions);
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MultiLimitOrderEntry {
    pub side: String,
    pub price: f64,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct MultiLimitOrderResult {
    pub action: String,
    pub symbol: String,
    pub bids: Vec<MultiLimitOrderEntry>,
    pub asks: Vec<MultiLimitOrderEntry>,
    pub slide: bool,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
    pub num_instructions: usize,
}

impl TableRenderable for MultiLimitOrderResult {
    fn render_table(&self) {
        if self.dry_run {
            println!("[DRY RUN] Would place multi-limit order:");
        } else {
            println!("Multi-limit order placed:");
        }
        println!("  Symbol: {}", self.symbol);
        println!("  Bids: {}", self.bids.len());
        for b in &self.bids {
            println!("    ${:.4} × {} lots", b.price, b.size);
        }
        println!("  Asks: {}", self.asks.len());
        for a in &self.asks {
            println!("    ${:.4} × {} lots", a.price, a.size);
        }
        println!("  Slide: {}", self.slide);
        println!("  Instructions: {}", self.num_instructions);
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CancelResult {
    pub symbol: String,
    pub cancelled_ids: Vec<String>,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
    pub num_instructions: usize,
}

impl TableRenderable for CancelResult {
    fn render_table(&self) {
        if self.dry_run {
            println!(
                "[DRY RUN] Would cancel {} orders on {}",
                self.cancelled_ids.len(),
                self.symbol
            );
        } else {
            println!(
                "Cancelled {} orders on {}",
                self.cancelled_ids.len(),
                self.symbol
            );
        }
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrderInfo {
    pub symbol: String,
    pub side: String,
    pub order_id: String,
    pub price: String,
    pub size_remaining: String,
    pub initial_size: String,
    pub reduce_only: bool,
    pub is_stop_loss: bool,
}

#[derive(Debug, Serialize)]
pub struct OrdersResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub orders: Vec<OrderInfo>,
}

impl TableRenderable for OrdersResult {
    fn render_table(&self) {
        if self.orders.is_empty() {
            match &self.symbol {
                Some(s) => println!("No open orders for {}.", s),
                None => println!("No open orders."),
            }
            return;
        }
        let show_symbol = self.symbol.is_none();
        let mut headers = vec!["Order ID", "Side", "Price", "Remaining", "Initial", "Flags"];
        if show_symbol {
            headers.insert(0, "Symbol");
        }
        let rows: Vec<Vec<String>> = self
            .orders
            .iter()
            .map(|o| {
                let mut flags = Vec::new();
                if o.reduce_only {
                    flags.push("RO");
                }
                if o.is_stop_loss {
                    flags.push("SL");
                }
                let mut row = Vec::new();
                if show_symbol {
                    row.push(o.symbol.clone());
                }
                row.extend([
                    o.order_id.clone(),
                    o.side.clone(),
                    o.price.clone(),
                    o.size_remaining.clone(),
                    o.initial_size.clone(),
                    flags.join(","),
                ]);
                row
            })
            .collect();
        crate::output::table::render_table(&headers, rows);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Get wallet password from VULCAN_WALLET_PASSWORD env var, or prompt via stderr.
pub fn prompt_password() -> Result<String, VulcanError> {
    if let Ok(pw) = std::env::var("VULCAN_WALLET_PASSWORD") {
        return Ok(pw);
    }
    eprint!("Wallet password: ");
    rpassword::read_password().map_err(|e| VulcanError::io("PASSWORD_READ_FAILED", e.to_string()))
}

/// Resolve the wallet and trader PDA for trading commands.
/// If a session wallet is available (MCP mode), use it directly.
pub fn resolve_wallet_and_pda(
    ctx: &AppContext,
    wallet_override: Option<&str>,
) -> Result<(crate::wallet::Wallet, Pubkey, Pubkey), VulcanError> {
    // MCP session wallet path — no password prompt needed
    if let Some(sw) = &ctx.session_wallet {
        let wallet = sw.to_wallet()?;
        return Ok((wallet, sw.authority, sw.trader_pda));
    }

    let wallet_name = match wallet_override {
        Some(name) => name.to_string(),
        None => ctx
            .wallet_store
            .default_wallet()
            .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
            .ok_or_else(|| {
                VulcanError::config(
                    "NO_DEFAULT_WALLET",
                    "No default wallet set. Use 'vulcan wallet set-default <NAME>'",
                )
            })?,
    };

    let wallet_file = ctx
        .wallet_store
        .load(&wallet_name)
        .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

    let password = prompt_password()?;
    let wallet = crate::wallet::Wallet::decrypt(&wallet_file.encrypted, &password)
        .map_err(|e| VulcanError::auth("DECRYPT_FAILED", e.to_string()))?;

    let authority = Pubkey::from_str(&wallet_file.public_key)
        .map_err(|e| VulcanError::validation("INVALID_PUBKEY", e.to_string()))?;

    // Default trader PDA: pda_index=0, subaccount_index=0 (cross-margin)
    let trader_key = phoenix_sdk::types::TraderKey::new(authority);
    let trader_pda = trader_key.pda();

    Ok((wallet, authority, trader_pda))
}

/// Resolve the default wallet's authority pubkey (no decryption needed).
fn resolve_authority(ctx: &AppContext) -> Result<Pubkey, VulcanError> {
    let wallet_name = ctx
        .wallet_store
        .default_wallet()
        .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
        .ok_or_else(|| VulcanError::config("NO_DEFAULT_WALLET", "No default wallet set"))?;

    let wallet_file = ctx
        .wallet_store
        .load(&wallet_name)
        .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

    Pubkey::from_str(&wallet_file.public_key)
        .map_err(|e| VulcanError::validation("INVALID_PUBKEY", e.to_string()))
}

/// Convert HTTP API TraderViews into the SDK Trader struct needed for isolated orders.
pub fn trader_from_views(authority: Pubkey, pda_index: u8, views: &[TraderView]) -> Trader {
    let key = TraderKey::new_with_idx(authority, pda_index, 0);
    let mut trader = Trader::new(key);

    for view in views {
        let collateral_f64: f64 = view.collateral_balance.value as f64
            / 10f64.powi(view.collateral_balance.decimals as i32);
        let collateral_quote_lots = (collateral_f64 * 1_000_000.0) as i64;

        let mut subaccount = SubaccountState {
            subaccount_index: view.trader_subaccount_index,
            collateral: SignedQuoteLots::new(collateral_quote_lots),
            ..Default::default()
        };

        // Convert positions
        for pos_view in &view.positions {
            let base_lots: i64 = pos_view.position_size.value;
            let entry_ticks: i64 = pos_view.entry_price.value;
            let entry_usd = pos_view
                .entry_price
                .ui
                .parse()
                .unwrap_or(phoenix_sdk::Decimal::ZERO);

            let position = SdkPosition {
                symbol: pos_view.symbol.clone(),
                base_position_lots: base_lots,
                entry_price_ticks: entry_ticks,
                entry_price_usd: entry_usd,
                virtual_quote_position_lots: 0,
                unsettled_funding_quote_lots: 0,
                accumulated_funding_quote_lots: 0,
            };
            subaccount
                .positions
                .insert(pos_view.symbol.clone(), position);
        }

        // Convert limit orders
        for (symbol, orders) in &view.limit_orders {
            for order in orders {
                let osn: u64 = order.order_sequence_number.parse().unwrap_or(0);
                let sdk_order = SdkLimitOrder {
                    symbol: symbol.clone(),
                    order_sequence_number: osn,
                    side: format!("{:?}", order.side),
                    order_type: String::new(),
                    price_ticks: order.price.value,
                    price_usd: order.price.ui.parse().unwrap_or(phoenix_sdk::Decimal::ZERO),
                    size_remaining_lots: order.trade_size_remaining.value.unsigned_abs(),
                    initial_size_lots: order.initial_trade_size.value.unsigned_abs(),
                    reduce_only: order.is_reduce_only,
                    is_stop_loss: order.is_stop_loss,
                    status: "Open".to_string(),
                };
                subaccount.orders.insert((symbol.clone(), osn), sdk_order);
            }
        }

        trader
            .subaccounts
            .insert(view.trader_subaccount_index, subaccount);
    }

    trader
}

/// Build, optionally sign, and submit a transaction.
pub async fn send_or_dry_run(
    ctx: &AppContext,
    ixs: Vec<solana_sdk::instruction::Instruction>,
    wallet: &crate::wallet::Wallet,
) -> Result<Option<String>, VulcanError> {
    if ctx.dry_run {
        return Ok(None);
    }

    let keypair = wallet
        .to_solana_keypair()
        .map_err(|e| VulcanError::auth("KEYPAIR_ERROR", e.to_string()))?;

    let rpc_client =
        solana_rpc_client::rpc_client::RpcClient::new(ctx.config.network.rpc_url.clone());

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| VulcanError::network("BLOCKHASH_FAILED", e.to_string()))?;

    // Prepend a compute budget instruction to avoid CU exhaustion on complex
    // transactions (e.g. opening a new position with many existing positions).
    let mut all_ixs = Vec::with_capacity(ixs.len() + 1);
    all_ixs.push(
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(400_000),
    );
    all_ixs.extend(ixs);

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &all_ixs,
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash,
    );

    let sig = rpc_client
        .send_and_confirm_transaction(&tx)
        .map_err(|e| VulcanError::tx_failed("TX_SEND_FAILED", e.to_string()))?;

    Ok(Some(sig.to_string()))
}

// ── Execution ───────────────────────────────────────────────────────────

pub async fn execute(ctx: &AppContext, cmd: TradeCommand) -> Result<(), VulcanError> {
    match cmd {
        TradeCommand::MarketBuy {
            symbol,
            size,
            tp,
            sl,
            isolated,
            collateral,
            reduce_only,
        } => {
            execute_market_order(
                ctx,
                &symbol,
                size,
                Side::Bid,
                tp,
                sl,
                isolated,
                collateral,
                reduce_only,
            )
            .await
        }
        TradeCommand::MarketSell {
            symbol,
            size,
            tp,
            sl,
            isolated,
            collateral,
            reduce_only,
        } => {
            execute_market_order(
                ctx,
                &symbol,
                size,
                Side::Ask,
                tp,
                sl,
                isolated,
                collateral,
                reduce_only,
            )
            .await
        }
        TradeCommand::LimitBuy {
            symbol,
            size,
            price,
            tp,
            sl,
            isolated,
            collateral,
            reduce_only,
        } => {
            execute_limit_order(
                ctx,
                &symbol,
                size,
                price,
                Side::Bid,
                tp,
                sl,
                isolated,
                collateral,
                reduce_only,
            )
            .await
        }
        TradeCommand::LimitSell {
            symbol,
            size,
            price,
            tp,
            sl,
            isolated,
            collateral,
            reduce_only,
        } => {
            execute_limit_order(
                ctx,
                &symbol,
                size,
                price,
                Side::Ask,
                tp,
                sl,
                isolated,
                collateral,
                reduce_only,
            )
            .await
        }
        TradeCommand::Cancel { symbol, order_ids } => execute_cancel(ctx, &symbol, order_ids).await,
        TradeCommand::CancelAll { symbol } => execute_cancel_all(ctx, &symbol).await,
        TradeCommand::Orders { symbol } => execute_orders(ctx, symbol.as_deref()).await,
        TradeCommand::SetTpsl { symbol, tp, sl } => execute_set_tpsl(ctx, &symbol, tp, sl).await,
        TradeCommand::CancelTpsl { symbol, tp, sl } => {
            execute_cancel_tpsl(ctx, &symbol, tp, sl).await
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_market_order_inner(
    ctx: &AppContext,
    symbol: &str,
    size: f64,
    side: Side,
    tp: Option<f64>,
    sl: Option<f64>,
    isolated: bool,
    collateral: Option<f64>,
    _reduce_only: bool,
) -> Result<OrderResult, VulcanError> {
    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let builder = ctx.tx_builder().await?;
    let num_base_lots = size as u64;

    let bracket = if tp.is_some() || sl.is_some() {
        Some(phoenix_sdk::BracketLegOrders {
            take_profit_price: tp,
            stop_loss_price: sl,
        })
    } else {
        None
    };

    let ixs = if isolated {
        // Fetch full trader state for isolated order building
        let traders = ctx
            .http_client
            .get_traders(&authority)
            .await
            .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

        let trader = trader_from_views(authority, 0, &traders);

        let collateral_flow = collateral.map(|c| IsolatedCollateralFlow::TransferFromCrossMargin {
            collateral: (c * 1_000_000.0) as u64,
        });

        builder
            .build_isolated_market_order(
                &trader,
                symbol,
                side,
                num_base_lots,
                collateral_flow,
                true, // allow_cross_and_isolated
                bracket.as_ref(),
            )
            .map_err(|e| VulcanError::api("BUILD_ORDER_FAILED", e.to_string()))?
    } else {
        // Check for isolated-only markets
        let metadata = ctx.metadata().await?;
        if metadata.is_isolated_only(symbol) {
            return Err(VulcanError::validation(
                "ISOLATED_ONLY_MARKET",
                format!(
                    "{} is isolated-only. Use --isolated --collateral <AMOUNT>.",
                    symbol
                ),
            ));
        }

        builder
            .build_market_order(
                authority,
                trader_pda,
                symbol,
                side,
                num_base_lots,
                bracket.as_ref(),
            )
            .map_err(|e| VulcanError::api("BUILD_ORDER_FAILED", e.to_string()))?
    };

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    let side_str = match side {
        Side::Bid => "buy",
        Side::Ask => "sell",
    };

    Ok(OrderResult {
        action: format!("market-{}", side_str),
        symbol: symbol.to_string(),
        side: side_str.to_string(),
        size,
        price: None,
        tp,
        sl,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_market_order(
    ctx: &AppContext,
    symbol: &str,
    size: f64,
    side: Side,
    tp: Option<f64>,
    sl: Option<f64>,
    isolated: bool,
    collateral: Option<f64>,
    reduce_only: bool,
) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm trade, or --dry-run to simulate",
        ));
    }

    let result = execute_market_order_inner(
        ctx,
        symbol,
        size,
        side,
        tp,
        sl,
        isolated,
        collateral,
        reduce_only,
    )
    .await?;

    let side_str = &result.side;
    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({
            "command": format!("trade market-{}", side_str),
            "symbol": symbol,
            "dry_run": ctx.dry_run,
        }),
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_limit_order_inner(
    ctx: &AppContext,
    symbol: &str,
    size: f64,
    price: f64,
    side: Side,
    tp: Option<f64>,
    sl: Option<f64>,
    isolated: bool,
    collateral: Option<f64>,
    _reduce_only: bool,
) -> Result<OrderResult, VulcanError> {
    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let builder = ctx.tx_builder().await?;
    let num_base_lots = size as u64;

    let bracket = if tp.is_some() || sl.is_some() {
        Some(phoenix_sdk::BracketLegOrders {
            take_profit_price: tp,
            stop_loss_price: sl,
        })
    } else {
        None
    };

    let ixs = if isolated {
        let traders = ctx
            .http_client
            .get_traders(&authority)
            .await
            .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

        let trader = trader_from_views(authority, 0, &traders);

        let collateral_flow = collateral.map(|c| IsolatedCollateralFlow::TransferFromCrossMargin {
            collateral: (c * 1_000_000.0) as u64,
        });

        builder
            .build_isolated_limit_order(
                &trader,
                symbol,
                side,
                price,
                num_base_lots,
                collateral_flow,
                true, // allow_cross_and_isolated
            )
            .map_err(|e| VulcanError::api("BUILD_ORDER_FAILED", e.to_string()))?
    } else {
        let metadata = ctx.metadata().await?;
        if metadata.is_isolated_only(symbol) {
            return Err(VulcanError::validation(
                "ISOLATED_ONLY_MARKET",
                format!(
                    "{} is isolated-only. Use --isolated --collateral <AMOUNT>.",
                    symbol
                ),
            ));
        }

        let mut limit_ixs = builder
            .build_limit_order(authority, trader_pda, symbol, side, price, num_base_lots)
            .map_err(|e| VulcanError::api("BUILD_ORDER_FAILED", e.to_string()))?;

        // Append bracket legs if TP/SL specified
        if let Some(ref bracket) = bracket {
            let bracket_ixs = builder
                .build_bracket_leg_orders(authority, trader_pda, symbol, side, bracket)
                .map_err(|e| VulcanError::api("BUILD_BRACKET_FAILED", e.to_string()))?;
            limit_ixs.extend(bracket_ixs);
        }

        limit_ixs
    };

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    let side_str = match side {
        Side::Bid => "buy",
        Side::Ask => "sell",
    };

    Ok(OrderResult {
        action: format!("limit-{}", side_str),
        symbol: symbol.to_string(),
        side: side_str.to_string(),
        size,
        price: Some(price),
        tp,
        sl,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

pub async fn execute_multi_limit_order_inner(
    ctx: &AppContext,
    symbol: &str,
    bids: Vec<(f64, u64)>,
    asks: Vec<(f64, u64)>,
    slide: bool,
) -> Result<MultiLimitOrderResult, VulcanError> {
    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let builder = ctx.tx_builder().await?;

    let metadata = ctx.metadata().await?;
    if metadata.is_isolated_only(symbol) {
        return Err(VulcanError::validation(
            "ISOLATED_ONLY_MARKET",
            format!(
                "{} is isolated-only. Multi-limit orders are not supported for isolated markets.",
                symbol
            ),
        ));
    }

    let ixs = builder
        .build_multi_limit_order(authority, trader_pda, symbol, &bids, &asks, slide)
        .map_err(|e| VulcanError::api("BUILD_MULTI_ORDER_FAILED", e.to_string()))?;

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    let bid_entries: Vec<MultiLimitOrderEntry> = bids
        .iter()
        .map(|(price, size)| MultiLimitOrderEntry {
            side: "buy".to_string(),
            price: *price,
            size: *size,
        })
        .collect();

    let ask_entries: Vec<MultiLimitOrderEntry> = asks
        .iter()
        .map(|(price, size)| MultiLimitOrderEntry {
            side: "sell".to_string(),
            price: *price,
            size: *size,
        })
        .collect();

    Ok(MultiLimitOrderResult {
        action: "multi-limit".to_string(),
        symbol: symbol.to_string(),
        bids: bid_entries,
        asks: ask_entries,
        slide,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_limit_order(
    ctx: &AppContext,
    symbol: &str,
    size: f64,
    price: f64,
    side: Side,
    tp: Option<f64>,
    sl: Option<f64>,
    isolated: bool,
    collateral: Option<f64>,
    reduce_only: bool,
) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm trade, or --dry-run to simulate",
        ));
    }

    let result = execute_limit_order_inner(
        ctx,
        symbol,
        size,
        price,
        side,
        tp,
        sl,
        isolated,
        collateral,
        reduce_only,
    )
    .await?;

    let side_str = &result.side;
    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({
            "command": format!("trade limit-{}", side_str),
            "symbol": symbol,
            "dry_run": ctx.dry_run,
        }),
    );
    Ok(())
}

pub async fn execute_cancel_inner(
    ctx: &AppContext,
    symbol: &str,
    order_ids: Vec<String>,
) -> Result<CancelResult, VulcanError> {
    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let builder = ctx.tx_builder().await?;

    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let trader = traders
        .iter()
        .find(|t| t.trader_subaccount_index == 0)
        .ok_or_else(|| {
            VulcanError::api("NO_TRADER_ACCOUNT", "No registered trader account found")
        })?;

    let symbol_upper = symbol.to_ascii_uppercase();
    let all_orders = trader
        .limit_orders
        .get(&symbol_upper)
        .cloned()
        .unwrap_or_default();

    let metadata = ctx.metadata().await?;
    let calc = metadata
        .get_market_calculator(&symbol_upper)
        .ok_or_else(|| {
            VulcanError::validation("UNKNOWN_MARKET", format!("Unknown market: {}", symbol))
        })?;

    let cancel_ids: Vec<phoenix_sdk::CancelId> = all_orders
        .iter()
        .filter(|o| order_ids.contains(&o.order_sequence_number))
        .map(|o| {
            let price_f64 = o.price.value as f64 / 10f64.powi(o.price.decimals as i32);
            let ticks = calc.price_to_ticks(price_f64).unwrap_or_default();
            phoenix_sdk::CancelId::new(
                ticks.into(),
                o.order_sequence_number.parse::<u64>().unwrap_or(0),
            )
        })
        .collect();

    if cancel_ids.is_empty() {
        return Err(VulcanError::validation(
            "INVALID_ORDER_IDS",
            "No matching open orders found for the provided IDs",
        ));
    }

    let ixs = builder
        .build_cancel_orders(authority, trader_pda, symbol, cancel_ids)
        .map_err(|e| VulcanError::api("BUILD_CANCEL_FAILED", e.to_string()))?;

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    Ok(CancelResult {
        symbol: symbol.to_string(),
        cancelled_ids: order_ids,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

async fn execute_cancel(
    ctx: &AppContext,
    symbol: &str,
    order_ids: Vec<String>,
) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm cancellation, or --dry-run to simulate",
        ));
    }

    let result = execute_cancel_inner(ctx, symbol, order_ids).await?;

    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({ "command": "trade cancel", "symbol": symbol, "dry_run": ctx.dry_run }),
    );
    Ok(())
}

pub async fn execute_cancel_all_inner(
    ctx: &AppContext,
    symbol: &str,
) -> Result<CancelResult, VulcanError> {
    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;

    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let trader = traders
        .iter()
        .find(|t| t.trader_subaccount_index == 0)
        .ok_or_else(|| {
            VulcanError::api("NO_TRADER_ACCOUNT", "No registered trader account found")
        })?;

    let symbol_upper = symbol.to_ascii_uppercase();
    let orders = trader
        .limit_orders
        .get(&symbol_upper)
        .cloned()
        .unwrap_or_default();

    if orders.is_empty() {
        return Ok(CancelResult {
            symbol: symbol.to_string(),
            cancelled_ids: vec![],
            dry_run: ctx.dry_run,
            tx_signature: None,
            num_instructions: 0,
        });
    }

    let order_ids: Vec<String> = orders
        .iter()
        .map(|o| o.order_sequence_number.clone())
        .collect();

    let metadata = ctx.metadata().await?;
    let calc = metadata
        .get_market_calculator(&symbol_upper)
        .ok_or_else(|| {
            VulcanError::validation("UNKNOWN_MARKET", format!("Unknown market: {}", symbol))
        })?;

    let cancel_ids: Vec<phoenix_sdk::CancelId> = orders
        .iter()
        .map(|o| {
            let price_f64 = o.price.value as f64 / 10f64.powi(o.price.decimals as i32);
            let ticks = calc.price_to_ticks(price_f64).unwrap_or_default();
            phoenix_sdk::CancelId::new(
                ticks.into(),
                o.order_sequence_number.parse::<u64>().unwrap_or(0),
            )
        })
        .collect();

    let builder = ctx.tx_builder().await?;

    let ixs = builder
        .build_cancel_orders(authority, trader_pda, symbol, cancel_ids)
        .map_err(|e| VulcanError::api("BUILD_CANCEL_FAILED", e.to_string()))?;

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    Ok(CancelResult {
        symbol: symbol.to_string(),
        cancelled_ids: order_ids,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

async fn execute_cancel_all(ctx: &AppContext, symbol: &str) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm cancellation, or --dry-run to simulate",
        ));
    }

    let result = execute_cancel_all_inner(ctx, symbol).await?;

    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({ "command": "trade cancel-all", "symbol": symbol, "dry_run": ctx.dry_run }),
    );
    Ok(())
}

pub async fn execute_orders_inner(
    ctx: &AppContext,
    symbol: Option<&str>,
) -> Result<OrdersResult, VulcanError> {
    let authority = resolve_authority(ctx)?;

    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let trader = traders
        .iter()
        .find(|t| t.trader_subaccount_index == 0)
        .ok_or_else(|| {
            VulcanError::api(
                "NO_TRADER_ACCOUNT",
                "No registered trader account found. Use 'vulcan account register' first.",
            )
        })?;

    let order_infos = match symbol {
        Some(sym) => {
            let symbol_upper = sym.to_ascii_uppercase();
            let orders = trader
                .limit_orders
                .get(&symbol_upper)
                .cloned()
                .unwrap_or_default();
            orders_to_infos(&symbol_upper, &orders)
        }
        None => {
            let mut all = Vec::new();
            for (sym, orders) in &trader.limit_orders {
                all.extend(orders_to_infos(sym, orders));
            }
            all
        }
    };

    Ok(OrdersResult {
        symbol: symbol.map(|s| s.to_ascii_uppercase()),
        orders: order_infos,
    })
}

fn orders_to_infos(symbol: &str, orders: &[phoenix_types::LimitOrder]) -> Vec<OrderInfo> {
    orders
        .iter()
        .map(|o| {
            let side = match o.side {
                phoenix_types::Side::Bid => "Buy",
                phoenix_types::Side::Ask => "Sell",
            };
            OrderInfo {
                symbol: symbol.to_string(),
                side: side.to_string(),
                order_id: o.order_sequence_number.clone(),
                price: o.price.ui.clone(),
                size_remaining: o.trade_size_remaining.ui.clone(),
                initial_size: o.initial_trade_size.ui.clone(),
                reduce_only: o.is_reduce_only,
                is_stop_loss: o.is_stop_loss,
            }
        })
        .collect()
}

async fn execute_orders(ctx: &AppContext, symbol: Option<&str>) -> Result<(), VulcanError> {
    let result = execute_orders_inner(ctx, symbol).await?;

    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({ "command": "trade orders", "symbol": symbol }),
    );

    if ctx.watch {
        let authority = resolve_authority(ctx)?;
        let sym = symbol.map(|s| s.to_string());
        crate::watch::watch_loop(ctx, crate::watch::WatchKind::TraderState(authority), || {
            let sym = sym.clone();
            async move {
                let result = execute_orders_inner(ctx, sym.as_deref()).await?;
                render_success(
                    ctx.output_format,
                    &result,
                    serde_json::json!({ "command": "trade orders", "symbol": sym }),
                );
                Ok(())
            }
        })
        .await?;
    }

    Ok(())
}

// ── TP/SL result types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SetTpSlResult {
    pub symbol: String,
    pub side: String,
    pub tp: Option<f64>,
    pub sl: Option<f64>,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
    pub num_instructions: usize,
}

impl TableRenderable for SetTpSlResult {
    fn render_table(&self) {
        if self.dry_run {
            println!(
                "[DRY RUN] Would set TP/SL on {} {} position:",
                self.symbol, self.side
            );
        } else {
            println!("TP/SL set on {} {} position:", self.symbol, self.side);
        }
        if let Some(tp) = self.tp {
            println!("  Take profit: ${:.2}", tp);
        }
        if let Some(sl) = self.sl {
            println!("  Stop loss: ${:.2}", sl);
        }
        println!("  Instructions: {}", self.num_instructions);
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CancelTpSlResult {
    pub symbol: String,
    pub cancelled_tp: bool,
    pub cancelled_sl: bool,
    pub dry_run: bool,
    pub tx_signature: Option<String>,
    pub num_instructions: usize,
}

impl TableRenderable for CancelTpSlResult {
    fn render_table(&self) {
        let mut legs = Vec::new();
        if self.cancelled_tp {
            legs.push("TP");
        }
        if self.cancelled_sl {
            legs.push("SL");
        }
        if self.dry_run {
            println!(
                "[DRY RUN] Would cancel {} on {}:",
                legs.join("/"),
                self.symbol
            );
        } else {
            println!("Cancelled {} on {}:", legs.join("/"), self.symbol);
        }
        println!("  Instructions: {}", self.num_instructions);
        if let Some(sig) = &self.tx_signature {
            println!("  Tx: {}", sig);
        }
    }
}

// ── set-tpsl ───────────────────────────────────────────────────────────

pub async fn execute_set_tpsl_inner(
    ctx: &AppContext,
    symbol: &str,
    tp: Option<f64>,
    sl: Option<f64>,
) -> Result<SetTpSlResult, VulcanError> {
    if tp.is_none() && sl.is_none() {
        return Err(VulcanError::validation(
            "NO_TP_SL",
            "Specify at least one of --tp or --sl",
        ));
    }

    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let symbol_upper = symbol.to_ascii_uppercase();

    // Fetch trader state to detect position side
    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let pos = traders
        .iter()
        .find_map(|t| {
            t.positions
                .iter()
                .find(|p| p.symbol.to_ascii_uppercase() == symbol_upper)
        })
        .ok_or_else(|| {
            VulcanError::validation(
                "NO_POSITION",
                format!(
                    "No open position for '{}'. TP/SL requires an existing position.",
                    symbol
                ),
            )
        })?;

    let is_long = !pos.position_size.ui.starts_with('-');
    let primary_side = if is_long { Side::Bid } else { Side::Ask };
    let side_str = if is_long { "Long" } else { "Short" };

    let bracket = phoenix_sdk::BracketLegOrders {
        take_profit_price: tp,
        stop_loss_price: sl,
    };

    let builder = ctx.tx_builder().await?;
    let ixs = builder
        .build_bracket_leg_orders(authority, trader_pda, &symbol_upper, primary_side, &bracket)
        .map_err(|e| VulcanError::api("BUILD_TPSL_FAILED", e.to_string()))?;

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    Ok(SetTpSlResult {
        symbol: symbol_upper,
        side: side_str.to_string(),
        tp,
        sl,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

async fn execute_set_tpsl(
    ctx: &AppContext,
    symbol: &str,
    tp: Option<f64>,
    sl: Option<f64>,
) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm, or --dry-run to simulate",
        ));
    }

    let result = execute_set_tpsl_inner(ctx, symbol, tp, sl).await?;
    let symbol_upper = result.symbol.clone();

    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({
            "command": "trade set-tpsl",
            "symbol": symbol_upper,
            "dry_run": ctx.dry_run,
        }),
    );
    Ok(())
}

// ── cancel-tpsl ────────────────────────────────────────────────────────

pub async fn execute_cancel_tpsl_inner(
    ctx: &AppContext,
    symbol: &str,
    cancel_tp: bool,
    cancel_sl: bool,
) -> Result<CancelTpSlResult, VulcanError> {
    if !cancel_tp && !cancel_sl {
        return Err(VulcanError::validation(
            "NO_TP_SL",
            "Specify at least one of --tp or --sl to cancel",
        ));
    }

    let (wallet, authority, trader_pda) = resolve_wallet_and_pda(ctx, None)?;
    let symbol_upper = symbol.to_ascii_uppercase();
    let builder = ctx.tx_builder().await?;

    // Detect position side to determine correct directions
    let traders = ctx
        .http_client
        .get_traders(&authority)
        .await
        .map_err(|e| VulcanError::api("TRADERS_FETCH_FAILED", e.to_string()))?;

    let pos = traders
        .iter()
        .find_map(|t| {
            t.positions
                .iter()
                .find(|p| p.symbol.to_ascii_uppercase() == symbol_upper)
        })
        .ok_or_else(|| {
            VulcanError::validation("NO_POSITION", format!("No open position for '{}'", symbol))
        })?;

    let is_long = !pos.position_size.ui.starts_with('-');

    // For longs: TP triggers GreaterThan, SL triggers LessThan
    // For shorts: TP triggers LessThan, SL triggers GreaterThan
    let mut ixs = Vec::new();

    if cancel_tp {
        let tp_direction = if is_long {
            phoenix_sdk::Direction::GreaterThan
        } else {
            phoenix_sdk::Direction::LessThan
        };
        let tp_ixs = builder
            .build_cancel_bracket_leg(authority, trader_pda, &symbol_upper, tp_direction)
            .map_err(|e| VulcanError::api("BUILD_CANCEL_TP_FAILED", e.to_string()))?;
        ixs.extend(tp_ixs);
    }

    if cancel_sl {
        let sl_direction = if is_long {
            phoenix_sdk::Direction::LessThan
        } else {
            phoenix_sdk::Direction::GreaterThan
        };
        let sl_ixs = builder
            .build_cancel_bracket_leg(authority, trader_pda, &symbol_upper, sl_direction)
            .map_err(|e| VulcanError::api("BUILD_CANCEL_SL_FAILED", e.to_string()))?;
        ixs.extend(sl_ixs);
    }

    let num_ixs = ixs.len();
    let sig = send_or_dry_run(ctx, ixs, &wallet).await?;

    Ok(CancelTpSlResult {
        symbol: symbol_upper,
        cancelled_tp: cancel_tp,
        cancelled_sl: cancel_sl,
        dry_run: ctx.dry_run,
        tx_signature: sig,
        num_instructions: num_ixs,
    })
}

async fn execute_cancel_tpsl(
    ctx: &AppContext,
    symbol: &str,
    cancel_tp: bool,
    cancel_sl: bool,
) -> Result<(), VulcanError> {
    if !ctx.yes && !ctx.dry_run {
        return Err(VulcanError::validation(
            "CONFIRMATION_REQUIRED",
            "Pass --yes to confirm, or --dry-run to simulate",
        ));
    }

    let result = execute_cancel_tpsl_inner(ctx, symbol, cancel_tp, cancel_sl).await?;
    let symbol_upper = result.symbol.clone();

    render_success(
        ctx.output_format,
        &result,
        serde_json::json!({
            "command": "trade cancel-tpsl",
            "symbol": symbol_upper,
            "dry_run": ctx.dry_run,
        }),
    );
    Ok(())
}
