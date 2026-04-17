//! Command execution logic.
//!
//! Each module receives parsed CLI args, calls the SDK, and returns typed results.
//! The output layer handles formatting.

pub mod account;
pub mod margin;
pub mod market;
pub mod position;
pub mod setup;
pub mod status;
pub mod trade;
pub mod wallet;
