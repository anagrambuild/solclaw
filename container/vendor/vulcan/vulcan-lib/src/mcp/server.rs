//! MCP server handler — implements rmcp ServerHandler.

use crate::commands;
use crate::context::AppContext;
use crate::mcp::registry::{self, ToolDef};
use phoenix_sdk::Side;
use rmcp::model::*;
use rmcp::ServerHandler;
use serde_json::Value;
use std::sync::Arc;

/// Vulcan MCP server — exposes trading tools over stdio.
pub struct VulcanMcpServer {
    ctx: Arc<AppContext>,
    tools: Vec<&'static ToolDef>,
    allow_dangerous: bool,
}

impl VulcanMcpServer {
    pub fn new(ctx: Arc<AppContext>, allow_dangerous: bool, groups: Option<Vec<String>>) -> Self {
        let tools = registry::tools_for_groups(&groups);
        Self {
            ctx,
            tools,
            allow_dangerous,
        }
    }

    /// Convert a ToolDef into an rmcp Tool model.
    fn to_rmcp_tool(def: &ToolDef) -> Tool {
        let schema = (def.schema)();
        let schema_obj: serde_json::Map<String, Value> = match schema {
            Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        Tool::new(def.name, def.description, Arc::new(schema_obj))
    }

    /// Dispatch a tool call to the appropriate command inner function.
    async fn dispatch(&self, name: &str, args: &Value) -> Result<Value, crate::error::VulcanError> {
        match name {
            // ── Market ──────────────────────────────────────────────────
            "vulcan_market_list" => {
                let result = commands::market::execute_list_inner(&self.ctx).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_market_ticker" => {
                let symbol = arg_str(args, "symbol")?;
                let result = commands::market::execute_ticker_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_market_info" => {
                let symbol = arg_str(args, "symbol")?;
                let result = commands::market::execute_info_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_market_orderbook" => {
                let symbol = arg_str(args, "symbol")?;
                let depth = arg_usize_or(args, "depth", 10);
                let result =
                    commands::market::execute_orderbook_inner(&self.ctx, &symbol, depth).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_market_candles" => {
                let symbol = arg_str(args, "symbol")?;
                let interval = arg_str_or(args, "interval", "1h");
                let limit = arg_usize_or(args, "limit", 50);
                let result =
                    commands::market::execute_candles_inner(&self.ctx, &symbol, &interval, limit)
                        .await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Trade ───────────────────────────────────────────────────
            "vulcan_trade_market_buy" => {
                let symbol = arg_str(args, "symbol")?;
                let size = arg_f64(args, "size")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let isolated = arg_bool_or(args, "isolated", false);
                let collateral = arg_f64_opt(args, "collateral");
                let reduce_only = arg_bool_or(args, "reduce_only", false);
                let result = commands::trade::execute_market_order_inner(
                    &self.ctx,
                    &symbol,
                    size,
                    Side::Bid,
                    tp,
                    sl,
                    isolated,
                    collateral,
                    reduce_only,
                )
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_market_sell" => {
                let symbol = arg_str(args, "symbol")?;
                let size = arg_f64(args, "size")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let isolated = arg_bool_or(args, "isolated", false);
                let collateral = arg_f64_opt(args, "collateral");
                let reduce_only = arg_bool_or(args, "reduce_only", false);
                let result = commands::trade::execute_market_order_inner(
                    &self.ctx,
                    &symbol,
                    size,
                    Side::Ask,
                    tp,
                    sl,
                    isolated,
                    collateral,
                    reduce_only,
                )
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_limit_buy" => {
                let symbol = arg_str(args, "symbol")?;
                let size = arg_f64(args, "size")?;
                let price = arg_f64(args, "price")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let isolated = arg_bool_or(args, "isolated", false);
                let collateral = arg_f64_opt(args, "collateral");
                let reduce_only = arg_bool_or(args, "reduce_only", false);
                let result = commands::trade::execute_limit_order_inner(
                    &self.ctx,
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
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_limit_sell" => {
                let symbol = arg_str(args, "symbol")?;
                let size = arg_f64(args, "size")?;
                let price = arg_f64(args, "price")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let isolated = arg_bool_or(args, "isolated", false);
                let collateral = arg_f64_opt(args, "collateral");
                let reduce_only = arg_bool_or(args, "reduce_only", false);
                let result = commands::trade::execute_limit_order_inner(
                    &self.ctx,
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
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_multi_limit" => {
                let symbol = arg_str(args, "symbol")?;
                let slide = arg_bool_or(args, "slide", false);
                let bids = arg_order_array(args, "bids")?;
                let asks = arg_order_array(args, "asks")?;
                let result = commands::trade::execute_multi_limit_order_inner(
                    &self.ctx, &symbol, bids, asks, slide,
                )
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_orders" => {
                let symbol = arg_str_opt(args, "symbol");
                let result =
                    commands::trade::execute_orders_inner(&self.ctx, symbol.as_deref()).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_cancel" => {
                let symbol = arg_str(args, "symbol")?;
                let order_ids = arg_str_array(args, "order_ids")?;
                let result =
                    commands::trade::execute_cancel_inner(&self.ctx, &symbol, order_ids).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_cancel_all" => {
                let symbol = arg_str(args, "symbol")?;
                let result = commands::trade::execute_cancel_all_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            "vulcan_trade_set_tpsl" => {
                let symbol = arg_str(args, "symbol")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let result =
                    commands::trade::execute_set_tpsl_inner(&self.ctx, &symbol, tp, sl).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_trade_cancel_tpsl" => {
                let symbol = arg_str(args, "symbol")?;
                let cancel_tp = arg_bool_or(args, "tp", false);
                let cancel_sl = arg_bool_or(args, "sl", false);
                let result = commands::trade::execute_cancel_tpsl_inner(
                    &self.ctx, &symbol, cancel_tp, cancel_sl,
                )
                .await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Position ────────────────────────────────────────────────
            "vulcan_position_list" => {
                let result = commands::position::execute_list_inner(&self.ctx).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_position_show" => {
                let symbol = arg_str(args, "symbol")?;
                let result = commands::position::execute_show_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_position_close" => {
                let symbol = arg_str(args, "symbol")?;
                let result = commands::position::execute_close_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Margin ──────────────────────────────────────────────────
            "vulcan_margin_status" => {
                let result = commands::margin::execute_status_inner(&self.ctx).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_deposit" => {
                let amount = arg_f64(args, "amount")?;
                let result =
                    commands::margin::execute_deposit_withdraw_inner(&self.ctx, amount, true)
                        .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_withdraw" => {
                let amount = arg_f64(args, "amount")?;
                let result =
                    commands::margin::execute_deposit_withdraw_inner(&self.ctx, amount, false)
                        .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_transfer" => {
                let from = arg_u8(args, "from_subaccount")?;
                let to = arg_u8(args, "to_subaccount")?;
                let amount = arg_f64(args, "amount")?;
                let result =
                    commands::margin::execute_transfer_inner(&self.ctx, amount, from, to).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_add_collateral" => {
                let symbol = arg_str(args, "symbol")?;
                let amount = arg_f64(args, "amount")?;
                let result =
                    commands::margin::execute_add_collateral_inner(&self.ctx, &symbol, amount)
                        .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_transfer_child_to_parent" => {
                let child = arg_u8(args, "child_subaccount")?;
                let result =
                    commands::margin::execute_transfer_child_to_parent_inner(&self.ctx, child)
                        .await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_sync_parent_to_child" => {
                let child = arg_u8(args, "child_subaccount")?;
                let result =
                    commands::margin::execute_sync_parent_to_child_inner(&self.ctx, child).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_margin_leverage_tiers" => {
                let symbol = arg_str(args, "symbol")?;
                let result =
                    commands::margin::execute_leverage_tiers_inner(&self.ctx, &symbol).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Position (new) ──────────────────────────────────────────
            "vulcan_position_reduce" => {
                let symbol = arg_str(args, "symbol")?;
                let size = arg_f64(args, "size")?;
                let result =
                    commands::position::execute_reduce_inner(&self.ctx, &symbol, size).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_position_tp_sl" => {
                let symbol = arg_str(args, "symbol")?;
                let tp = arg_f64_opt(args, "tp");
                let sl = arg_f64_opt(args, "sl");
                let result =
                    commands::position::execute_tp_sl_inner(&self.ctx, &symbol, tp, sl).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── History ─────────────────────────────────────────────────
            "vulcan_history_trades"
            | "vulcan_history_orders"
            | "vulcan_history_collateral"
            | "vulcan_history_funding"
            | "vulcan_history_pnl" => Err(crate::error::VulcanError::validation(
                "NOT_IMPLEMENTED",
                format!("{} is not yet implemented", name),
            )),

            // ── Status ────────────────────────────────────────────────────
            "vulcan_status" => {
                let result = commands::status::execute_inner(&self.ctx).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Wallet ────────────────────────────────────────────────────
            "vulcan_wallet_list" => {
                let result = commands::wallet::execute_list_inner(&self.ctx)?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_wallet_balance" => {
                let name = arg_str_opt(args, "name");
                let result = commands::wallet::execute_balance_inner(&self.ctx, name.as_deref())?;
                Ok(serde_json::to_value(result).unwrap())
            }

            // ── Account ───────────────────────────────────────────────────
            "vulcan_account_info" => {
                let result = commands::account::execute_info_inner(&self.ctx).await?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "vulcan_account_register" => {
                let invite_code = arg_str(args, "invite_code")?;
                let result =
                    commands::account::execute_register_inner(&self.ctx, &invite_code).await?;
                Ok(serde_json::to_value(result).unwrap())
            }

            _ => Err(crate::error::VulcanError::validation(
                "UNKNOWN_TOOL",
                format!("Unknown tool: {}", name),
            )),
        }
    }
}

// ── Embedded agent resources ────────────────────────────────────────────
static RESOURCES: &[(&str, &str, &str)] = &[
    // ── Context ────────────────────────────────────────────────────────
    (
        "vulcan://context",
        "Vulcan Runtime Context for AI Agents",
        include_str!("../../../CONTEXT.md"),
    ),
    // ── Legacy agent resources ─────────────────────────────────────────
    (
        "vulcan://agents/system",
        "Vulcan System Prompt",
        include_str!("../../../agents/system.md"),
    ),
    (
        "vulcan://agents/workflows/trade",
        "Trade Workflow",
        include_str!("../../../agents/workflows/trade.md"),
    ),
    (
        "vulcan://agents/workflows/portfolio",
        "Portfolio Overview Workflow",
        include_str!("../../../agents/workflows/portfolio.md"),
    ),
    (
        "vulcan://agents/workflows/risk",
        "Risk Management Rules",
        include_str!("../../../agents/workflows/risk.md"),
    ),
    (
        "vulcan://agents/workflows/onboarding",
        "Onboarding & Registration Workflow",
        include_str!("../../../agents/workflows/onboarding.md"),
    ),
    (
        "vulcan://agents/error-catalog",
        "Error Catalog — codes, categories, and recovery hints",
        include_str!("../../../agents/error-catalog.json"),
    ),
    // ── Skills ─────────────────────────────────────────────────────────
    (
        "vulcan://skills/index",
        "Skills Index — all available workflow skills",
        include_str!("../../../skills/INDEX.md"),
    ),
    (
        "vulcan://skills/vulcan-shared",
        "Shared runtime contract: auth, invocation, symbol format, safety",
        include_str!("../../../skills/vulcan-shared/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-risk-management",
        "Pre-trade risk checks, leverage tiers, margin health, when to warn",
        include_str!("../../../skills/vulcan-risk-management/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-error-recovery",
        "Error category routing, tx_failed recovery, network error handling",
        include_str!("../../../skills/vulcan-error-recovery/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-trade-execution",
        "Safe order execution with pre-trade checks and post-trade verification",
        include_str!("../../../skills/vulcan-trade-execution/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-lot-size-calculator",
        "Convert desired token amounts to base lots with worked examples",
        include_str!("../../../skills/vulcan-lot-size-calculator/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-tpsl-management",
        "Take-profit and stop-loss: direction rules, constraints, set/cancel flows",
        include_str!("../../../skills/vulcan-tpsl-management/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-market-intel",
        "Ticker, orderbook, candles, market info, and pre-trade analysis",
        include_str!("../../../skills/vulcan-market-intel/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-portfolio-intel",
        "Portfolio snapshot: margin status, positions, orders, funding rates",
        include_str!("../../../skills/vulcan-portfolio-intel/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-margin-operations",
        "Deposit, withdraw, transfer, isolated margin, collateral management",
        include_str!("../../../skills/vulcan-margin-operations/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-position-management",
        "List, show, close, reduce positions and manage TP/SL",
        include_str!("../../../skills/vulcan-position-management/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-onboarding",
        "New user setup: wallet creation, registration, first deposit",
        include_str!("../../../skills/vulcan-onboarding/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-twap-execution",
        "Execute large orders as time-weighted slices to reduce market impact",
        include_str!("../../../skills/vulcan-twap-execution/SKILL.md"),
    ),
    (
        "vulcan://skills/vulcan-grid-trading",
        "Grid trading with layered limit orders across a price range",
        include_str!("../../../skills/vulcan-grid-trading/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-emergency-flatten",
        "Cancel all orders and close all positions across all markets",
        include_str!("../../../skills/recipe-emergency-flatten/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-open-hedged-position",
        "Open a position with TP/SL protection in one flow",
        include_str!("../../../skills/recipe-open-hedged-position/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-morning-portfolio-check",
        "Daily portfolio review with margin, positions, and funding rates",
        include_str!("../../../skills/recipe-morning-portfolio-check/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-scale-into-position",
        "Add to an existing position in calculated increments",
        include_str!("../../../skills/recipe-scale-into-position/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-funding-rate-harvest",
        "Scan markets for favorable funding rates and open positions",
        include_str!("../../../skills/recipe-funding-rate-harvest/SKILL.md"),
    ),
    (
        "vulcan://skills/recipe-close-and-withdraw",
        "Close all positions and withdraw collateral to wallet",
        include_str!("../../../skills/recipe-close-and-withdraw/SKILL.md"),
    ),
];

impl ServerHandler for VulcanMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_instructions(
            "Vulcan MCP server for Phoenix Perpetuals DEX on Solana. \
             Use market tools for price data, trade tools for order management, \
             position tools to monitor open positions, and margin tools for collateral. \
             Dangerous tools (trades, deposits, withdrawals, cancellations) require \
             acknowledged=true. Size is in base lots — call vulcan_market_info first. \
             Read vulcan://context for the full runtime contract. \
             Read vulcan://skills/index for goal-oriented workflow skills.",
        )
    }

    #[allow(deprecated)]
    #[allow(clippy::manual_async_fn)]
    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async {
            let resources: Vec<Resource> = RESOURCES
                .iter()
                .map(|(uri, name, _)| {
                    Annotated::new(
                        RawResource::new(*uri, *name).with_mime_type("text/markdown"),
                        None,
                    )
                })
                .collect();
            Ok(ListResourcesResult::with_all_items(resources))
        }
    }

    #[allow(deprecated)]
    #[allow(clippy::manual_async_fn)]
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        async move {
            let uri = request.uri.as_str();
            let content = RESOURCES.iter().find(|(u, _, _)| *u == uri);
            match content {
                Some((uri, _, text)) => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                    *text, *uri,
                )
                .with_mime_type("text/markdown")])),
                None => Err(ErrorData::resource_not_found(
                    format!("Unknown resource: {}", uri),
                    None,
                )),
            }
        }
    }

    #[allow(deprecated)]
    #[allow(clippy::manual_async_fn)]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        async {
            let tools: Vec<Tool> = self.tools.iter().map(|t| Self::to_rmcp_tool(t)).collect();
            Ok(ListToolsResult::with_all_items(tools))
        }
    }

    #[allow(deprecated)]
    #[allow(clippy::manual_async_fn)]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
            let name = request.name.as_ref();
            let args = match &request.arguments {
                Some(map) => Value::Object(map.clone()),
                None => Value::Object(serde_json::Map::new()),
            };

            // Audit log to stderr (tool name + arg keys only, never values)
            let arg_keys: Vec<&String> = match &args {
                Value::Object(m) => m.keys().collect(),
                _ => vec![],
            };
            eprintln!("[mcp] call_tool: {} args={:?}", name, arg_keys);

            // Find tool definition
            let tool_def = self.tools.iter().find(|t| t.name == name);
            let tool_def = match tool_def {
                Some(t) => t,
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Unknown tool: {}",
                        name
                    ))]));
                }
            };

            // Dangerous command gating
            if tool_def.dangerous {
                if !self.allow_dangerous {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "This tool is dangerous and --allow-dangerous was not set on the server.",
                    )]));
                }
                let acknowledged = args
                    .get("acknowledged")
                    .map(|v| {
                        v.as_bool().unwrap_or_else(|| {
                            // Some MCP clients send "true" as a string instead of a JSON boolean
                            v.as_str()
                                .map(|s| s.eq_ignore_ascii_case("true"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                if !acknowledged {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "Dangerous operation requires acknowledged=true. \
                         This is a real financial transaction. Set acknowledged=true to proceed.",
                    )]));
                }
            }

            // Dispatch
            match self.dispatch(name, &args).await {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(text)]))
                }
                Err(e) => {
                    eprintln!("[mcp] error: {}", e);
                    let error_json = serde_json::json!({
                        "category": e.category.to_string(),
                        "code": e.code,
                        "message": e.message,
                        "retryable": e.category.is_retryable(),
                    });
                    Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_json).unwrap_or(e.message.clone()),
                    )]))
                }
            }
        }
    }
}

// ── Arg extraction helpers ──────────────────────────────────────────────

fn arg_str(args: &Value, key: &str) -> Result<String, crate::error::VulcanError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            crate::error::VulcanError::validation(
                "MISSING_ARG",
                format!("Required argument '{}' is missing or not a string", key),
            )
        })
}

fn arg_str_opt(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn arg_str_or(args: &Value, key: &str, default: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn arg_f64(args: &Value, key: &str) -> Result<f64, crate::error::VulcanError> {
    args.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
        .ok_or_else(|| {
            crate::error::VulcanError::validation(
                "MISSING_ARG",
                format!("Required argument '{}' is missing or not a number", key),
            )
        })
}

fn arg_f64_opt(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
}

fn arg_usize_or(args: &Value, key: &str, default: usize) -> usize {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        .map(|v| v as usize)
        .unwrap_or(default)
}

fn arg_bool_or(args: &Value, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|v| {
            v.as_bool()
                .or_else(|| v.as_str().map(|s| s.eq_ignore_ascii_case("true")))
        })
        .unwrap_or(default)
}

fn arg_u8(args: &Value, key: &str) -> Result<u8, crate::error::VulcanError> {
    args.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
        .map(|v| v as u8)
        .ok_or_else(|| {
            crate::error::VulcanError::validation(
                "MISSING_ARG",
                format!("Required argument '{}' is missing or not a number", key),
            )
        })
}

fn arg_order_array(args: &Value, key: &str) -> Result<Vec<(f64, u64)>, crate::error::VulcanError> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let price = entry
                        .get("price")
                        .and_then(|v| v.as_f64())?;
                    let size = entry
                        .get("size")
                        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))?;
                    Some((price, size))
                })
                .collect()
        })
        .ok_or_else(|| {
            crate::error::VulcanError::validation(
                "MISSING_ARG",
                format!(
                    "Required argument '{}' is missing or not an array of {{price, size}} objects",
                    key
                ),
            )
        })
}

fn arg_str_array(args: &Value, key: &str) -> Result<Vec<String>, crate::error::VulcanError> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .ok_or_else(|| {
            crate::error::VulcanError::validation(
                "MISSING_ARG",
                format!("Required argument '{}' is missing or not an array", key),
            )
        })
}
