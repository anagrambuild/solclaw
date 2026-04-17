//! Vulcan CLI entry point.

use clap::Parser;
use vulcan_lib::cli::{Cli, Command};
use vulcan_lib::context::AppContext;
use vulcan_lib::error::VulcanError;
use vulcan_lib::output::render_error;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("vulcan=debug")
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("vulcan=warn")
            .with_writer(std::io::stderr)
            .init();
    }

    let ctx = match AppContext::new(
        cli.output,
        cli.dry_run,
        cli.yes,
        cli.verbose,
        cli.watch,
        cli.rpc_url,
        cli.api_url,
        cli.api_key,
    ) {
        Ok(ctx) => ctx,
        Err(e) => {
            let err = VulcanError::config("INIT_FAILED", e.to_string());
            render_error(cli.output, &err);
            std::process::exit(err.exit_code());
        }
    };

    let result = match cli.command {
        Command::Wallet(cmd) => vulcan_lib::commands::wallet::execute(&ctx, cmd).await,
        Command::Market(cmd) => vulcan_lib::commands::market::execute(&ctx, cmd).await,
        Command::Trade(cmd) => vulcan_lib::commands::trade::execute(&ctx, cmd).await,
        Command::Position(cmd) => vulcan_lib::commands::position::execute(&ctx, cmd).await,
        Command::Margin(cmd) => vulcan_lib::commands::margin::execute(&ctx, cmd).await,
        Command::Account(cmd) => vulcan_lib::commands::account::execute(&ctx, cmd).await,
        Command::History(_cmd) => Err(vulcan_lib::error::VulcanError::validation(
            "NOT_IMPLEMENTED",
            "history commands are not yet implemented",
        )),
        Command::Status => vulcan_lib::commands::status::execute(&ctx).await,
        Command::Setup => vulcan_lib::commands::setup::execute(&ctx.wallet_store),
        Command::Version => {
            println!("vulcan {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::AgentContext => {
            print!("{}", include_str!("../../CONTEXT.md"));
            Ok(())
        }
        Command::Mcp {
            allow_dangerous,
            groups,
        } => run_mcp(allow_dangerous, groups, cli.verbose).await,
    };

    if let Err(err) = result {
        render_error(ctx.output_format, &err);
        std::process::exit(err.exit_code());
    }
}

async fn run_mcp(
    allow_dangerous: bool,
    groups: Option<Vec<String>>,
    verbose: bool,
) -> Result<(), VulcanError> {
    use rmcp::ServiceExt;
    use std::sync::Arc;

    let mut mcp_ctx = AppContext::new(
        vulcan_lib::output::OutputFormat::Json,
        false, // dry_run
        true,  // yes (auto-confirm for MCP)
        verbose,
        false, // watch
        None,
        None,
        None,
    )
    .map_err(|e| VulcanError::config("MCP_INIT_FAILED", e.to_string()))?;

    // Unlock session wallet if dangerous tools are enabled
    if allow_dangerous {
        let password = match std::env::var("VULCAN_WALLET_PASSWORD") {
            Ok(pw) => pw,
            Err(_) => {
                eprint!("Wallet password (for MCP session): ");
                rpassword::read_password()
                    .map_err(|e| VulcanError::io("PASSWORD_READ_FAILED", e.to_string()))?
            }
        };

        let wallet_name = mcp_ctx
            .wallet_store
            .default_wallet()
            .map_err(|e| VulcanError::config("CONFIG_ERROR", e.to_string()))?
            .ok_or_else(|| {
                VulcanError::config(
                    "NO_DEFAULT_WALLET",
                    "No default wallet set. Use 'vulcan wallet set-default <NAME>'",
                )
            })?;

        let wallet_file = mcp_ctx
            .wallet_store
            .load(&wallet_name)
            .map_err(|e| VulcanError::auth("WALLET_NOT_FOUND", e.to_string()))?;

        let wallet = vulcan_lib::wallet::Wallet::decrypt(&wallet_file.encrypted, &password)
            .map_err(|e| VulcanError::auth("DECRYPT_FAILED", e.to_string()))?;

        let session_wallet =
            vulcan_lib::mcp::session_wallet::SessionWallet::new(&wallet, &wallet_file)?;

        eprintln!("[mcp] Session wallet unlocked: {}", wallet_file.public_key);
        mcp_ctx.session_wallet = Some(Arc::new(session_wallet));
    }

    let ctx = Arc::new(mcp_ctx);
    let server = vulcan_lib::mcp::server::VulcanMcpServer::new(ctx, allow_dangerous, groups);

    eprintln!("[mcp] Starting Vulcan MCP server over stdio...");

    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| VulcanError::internal("MCP_SERVE_FAILED", e.to_string()))?;

    service
        .waiting()
        .await
        .map_err(|e| VulcanError::internal("MCP_WAIT_FAILED", e.to_string()))?;

    Ok(())
}
