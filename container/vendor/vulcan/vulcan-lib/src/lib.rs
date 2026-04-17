//! Vulcan — AI-native CLI for Phoenix Perpetuals DEX on Solana.
//!
//! This is the core library crate. The binary crate (`vulcan`) handles
//! argument parsing and dispatches to command handlers here.

pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod crypto;
pub mod error;
pub mod mcp;
pub mod output;
pub mod wallet;
pub mod watch;
