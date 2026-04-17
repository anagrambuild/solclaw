//! Error types for Phoenix instruction construction.

use thiserror::Error;

/// Errors that can occur when building Phoenix instructions.
#[derive(Debug, Error)]
pub enum PhoenixIxError {
    #[error("Trader wallet is required")]
    MissingTrader,

    #[error("Trader account is required")]
    MissingTraderAccount,

    #[error("Perp asset map is required")]
    MissingPerpAssetMap,

    #[error("Orderbook is required")]
    MissingOrderbook,

    #[error("Spline collection is required")]
    MissingSplineCollection,

    #[error("Active trader buffer array is required and must not be empty")]
    EmptyActiveTraderBuffer,

    #[error("Global trader index array is required and must not be empty")]
    EmptyGlobalTraderIndex,

    #[error("At least one order ID is required")]
    NoOrderIds,

    #[error("Too many order IDs (maximum 100)")]
    TooManyOrderIds,

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid deposit amount (must be greater than 0)")]
    InvalidDepositAmount,

    #[error("Invalid withdraw amount (must be greater than 0)")]
    InvalidWithdrawAmount,

    #[error("Invalid subaccount index for isolated margin (must be 0-100)")]
    InvalidSubaccountIndex,

    #[error("Invalid transfer amount (must be greater than 0)")]
    InvalidTransferAmount,
}
