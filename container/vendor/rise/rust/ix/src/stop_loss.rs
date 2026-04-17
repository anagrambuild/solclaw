//! Place stop loss instruction construction.
//!
//! Used for both stop-loss and take-profit bracket leg orders.
//! The `execution_direction` field determines trigger behavior:
//! - `LessThan`: triggers when price drops below threshold (stop-loss on longs)
//! - `GreaterThan`: triggers when price rises above threshold (take-profit on
//!   longs)

use solana_pubkey::Pubkey;

use crate::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    get_stop_loss_address, place_stop_loss_discriminant,
};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Direction, Instruction, Side, StopLossOrderKind};

/// Parameters for placing a stop loss order (used for SL and TP bracket legs).
#[derive(Debug, Clone)]
pub struct StopLossParams {
    funder: Pubkey,
    trader_account: Pubkey,
    position_authority: Pubkey,
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    asset_id: u64,
    trigger_price: u64,
    execution_price: u64,
    trade_side: Side,
    execution_direction: Direction,
    order_kind: StopLossOrderKind,
}

impl StopLossParams {
    pub fn builder() -> StopLossParamsBuilder {
        StopLossParamsBuilder::new()
    }

    pub fn funder(&self) -> Pubkey {
        self.funder
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn position_authority(&self) -> Pubkey {
        self.position_authority
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn orderbook(&self) -> Pubkey {
        self.orderbook
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn asset_id(&self) -> u64 {
        self.asset_id
    }

    pub fn trigger_price(&self) -> u64 {
        self.trigger_price
    }

    pub fn execution_price(&self) -> u64 {
        self.execution_price
    }

    pub fn trade_side(&self) -> Side {
        self.trade_side
    }

    pub fn execution_direction(&self) -> Direction {
        self.execution_direction
    }

    pub fn order_kind(&self) -> StopLossOrderKind {
        self.order_kind
    }
}

/// Builder for `StopLossParams`.
#[derive(Default)]
pub struct StopLossParamsBuilder {
    funder: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    position_authority: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    asset_id: Option<u64>,
    trigger_price: Option<u64>,
    execution_price: Option<u64>,
    trade_side: Option<Side>,
    execution_direction: Option<Direction>,
    order_kind: Option<StopLossOrderKind>,
}

impl StopLossParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn funder(mut self, funder: Pubkey) -> Self {
        self.funder = Some(funder);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn position_authority(mut self, position_authority: Pubkey) -> Self {
        self.position_authority = Some(position_authority);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn asset_id(mut self, asset_id: u64) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn trigger_price(mut self, trigger_price: u64) -> Self {
        self.trigger_price = Some(trigger_price);
        self
    }

    pub fn execution_price(mut self, execution_price: u64) -> Self {
        self.execution_price = Some(execution_price);
        self
    }

    pub fn trade_side(mut self, trade_side: Side) -> Self {
        self.trade_side = Some(trade_side);
        self
    }

    pub fn execution_direction(mut self, execution_direction: Direction) -> Self {
        self.execution_direction = Some(execution_direction);
        self
    }

    pub fn order_kind(mut self, order_kind: StopLossOrderKind) -> Self {
        self.order_kind = Some(order_kind);
        self
    }

    pub fn build(self) -> Result<StopLossParams, PhoenixIxError> {
        Ok(StopLossParams {
            funder: self.funder.ok_or(PhoenixIxError::MissingField("funder"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            position_authority: self
                .position_authority
                .ok_or(PhoenixIxError::MissingField("position_authority"))?,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            global_trader_index: self
                .global_trader_index
                .ok_or(PhoenixIxError::MissingField("global_trader_index"))?,
            active_trader_buffer: self
                .active_trader_buffer
                .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?,
            asset_id: self
                .asset_id
                .ok_or(PhoenixIxError::MissingField("asset_id"))?,
            trigger_price: self
                .trigger_price
                .ok_or(PhoenixIxError::MissingField("trigger_price"))?,
            execution_price: self
                .execution_price
                .ok_or(PhoenixIxError::MissingField("execution_price"))?,
            trade_side: self
                .trade_side
                .ok_or(PhoenixIxError::MissingField("trade_side"))?,
            execution_direction: self
                .execution_direction
                .ok_or(PhoenixIxError::MissingField("execution_direction"))?,
            order_kind: self
                .order_kind
                .ok_or(PhoenixIxError::MissingField("order_kind"))?,
        })
    }
}

/// Create a place stop loss instruction.
pub fn create_place_stop_loss_ix(params: StopLossParams) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_stop_loss(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &StopLossParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index.is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer.is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

fn encode_stop_loss(params: &StopLossParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(35);

    // 8 bytes: discriminant
    data.extend_from_slice(&place_stop_loss_discriminant());
    // 8 bytes: trigger_price
    data.extend_from_slice(&params.trigger_price.to_le_bytes());
    // 8 bytes: execution_price
    data.extend_from_slice(&params.execution_price.to_le_bytes());
    // 8 bytes: _trade_size = 0
    data.extend_from_slice(&0u64.to_le_bytes());
    // 1 byte: trade_side
    data.push(params.trade_side as u8);
    // 1 byte: execution_direction
    data.push(params.execution_direction as u8);
    // 1 byte: order_kind
    data.push(params.order_kind as u8);

    data
}

fn build_accounts(params: &StopLossParams) -> Vec<AccountMeta> {
    let stop_loss_pda = get_stop_loss_address(&params.trader_account, params.asset_id);

    let mut accounts = Vec::new();

    // LogAccountGroupAccounts
    accounts.push(AccountMeta::readonly(PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(PHOENIX_LOG_AUTHORITY));

    // PlaceStopLossInstructionGroupAccounts
    accounts.push(AccountMeta::writable(PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable_signer(params.funder));

    // MatchingEngineAccountGroupAccounts (same ordering as limit_order.rs)
    accounts.push(AccountMeta::writable(params.trader_account));
    accounts.push(AccountMeta::writable(params.perp_asset_map));
    for addr in &params.global_trader_index {
        accounts.push(AccountMeta::writable(*addr));
    }
    for addr in &params.active_trader_buffer {
        accounts.push(AccountMeta::writable(*addr));
    }
    accounts.push(AccountMeta::writable(params.orderbook));
    accounts.push(AccountMeta::writable(params.spline_collection));

    // Trailing accounts
    accounts.push(AccountMeta::readonly_signer(params.position_authority));
    accounts.push(AccountMeta::writable(stop_loss_pda));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> StopLossParams {
        StopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .asset_id(1)
            .trigger_price(50000)
            .execution_price(50000)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .order_kind(StopLossOrderKind::IOC)
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_stop_loss_ix() {
        let params = test_params();
        let ix = create_place_stop_loss_ix(params).unwrap();

        assert_eq!(ix.program_id, PHOENIX_PROGRAM_ID);
        // 2 log + 2 (global_config, funder) + 2 (trader_account, perp_asset_map)
        // + 1 gti + 1 atb + 2 (orderbook, spline) + 3 (authority, stop_loss, system) =
        //   13
        assert_eq!(ix.accounts.len(), 13);
        assert_eq!(&ix.data[..8], &place_stop_loss_discriminant());
    }

    #[test]
    fn test_stop_loss_data_encoding() {
        let params = test_params();
        let data = encode_stop_loss(&params);

        // 8 discriminant + 8 trigger + 8 execution + 8 trade_size + 1 side + 1 dir + 1
        // kind = 35
        assert_eq!(data.len(), 35);

        let trigger = u64::from_le_bytes(data[8..16].try_into().unwrap());
        assert_eq!(trigger, 50000);

        let execution = u64::from_le_bytes(data[16..24].try_into().unwrap());
        assert_eq!(execution, 50000);

        let trade_size = u64::from_le_bytes(data[24..32].try_into().unwrap());
        assert_eq!(trade_size, 0);

        assert_eq!(data[32], Side::Ask as u8);
        assert_eq!(data[33], Direction::LessThan as u8);
        assert_eq!(data[34], StopLossOrderKind::IOC as u8);
    }

    #[test]
    fn test_stop_loss_account_positions() {
        let params = test_params();
        let accounts = build_accounts(&params);

        // Position 0: program id (readonly)
        assert_eq!(accounts[0].pubkey, PHOENIX_PROGRAM_ID);
        assert!(!accounts[0].is_signer);
        assert!(!accounts[0].is_writable);

        // Position 1: log authority (readonly)
        assert_eq!(accounts[1].pubkey, PHOENIX_LOG_AUTHORITY);
        assert!(!accounts[1].is_signer);

        // Position 2: global config (writable)
        assert_eq!(accounts[2].pubkey, PHOENIX_GLOBAL_CONFIGURATION);
        assert!(accounts[2].is_writable);

        // Position 3: funder (writable signer)
        assert_eq!(accounts[3].pubkey, params.funder);
        assert!(accounts[3].is_signer);
        assert!(accounts[3].is_writable);

        // Position 4: trader_account (writable)
        assert_eq!(accounts[4].pubkey, params.trader_account);
        assert!(accounts[4].is_writable);

        // Position 5: perp_asset_map (writable)
        assert_eq!(accounts[5].pubkey, params.perp_asset_map);
        assert!(accounts[5].is_writable);

        // Positions 6..N-4: gti + atb (writable)
        // Position N-4: orderbook (writable)
        // Position N-3: spline_collection (writable)
        let n = accounts.len();

        // Position N-3: position_authority (readonly signer)
        assert_eq!(accounts[n - 3].pubkey, params.position_authority);
        assert!(accounts[n - 3].is_signer);
        assert!(!accounts[n - 3].is_writable);

        // Position N-2: stop_loss_account (writable)
        let expected_sl_pda = get_stop_loss_address(&params.trader_account, params.asset_id);
        assert_eq!(accounts[n - 2].pubkey, expected_sl_pda);
        assert!(accounts[n - 2].is_writable);

        // Position N-1: system_program (readonly)
        assert_eq!(accounts[n - 1].pubkey, SYSTEM_PROGRAM_ID);
        assert!(!accounts[n - 1].is_signer);
        assert!(!accounts[n - 1].is_writable);
    }

    #[test]
    fn test_empty_global_trader_index_fails() {
        let params = StopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .asset_id(1)
            .trigger_price(50000)
            .execution_price(50000)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .order_kind(StopLossOrderKind::IOC)
            .build()
            .unwrap();

        let result = create_place_stop_loss_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = StopLossParams::builder()
            .funder(Pubkey::new_unique())
            .build();
        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }
}
