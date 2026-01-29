//! Bonk 指令解析器
//!
//! 使用 match discriminator 模式解析 Bonk 指令

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;
use super::program_ids;

/// Bonk discriminator 常量
pub mod discriminators {
    pub const TRADE: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
    pub const POOL_CREATE: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const MIGRATE_AMM: [u8; 8] = [3, 4, 5, 6, 7, 8, 9, 10];
    /// Anchor discriminator for initialize_v2: sha256("global:initialize_v2")[..8]
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];
}

/// Raydium Launchpad 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::BONK_PROGRAM_ID;

/// 主要的 Bonk 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::TRADE => {
            parse_trade_instruction(data, accounts, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        discriminators::POOL_CREATE => {
            parse_pool_create_instruction(data, accounts, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        discriminators::MIGRATE_AMM => {
            parse_migrate_amm_instruction(data, accounts, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        discriminators::INITIALIZE_V2 => {
            parse_initialize_v2_instruction(data, accounts, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析交易指令
#[allow(unused_variables)]
fn parse_trade_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out_min = read_u64_le(data, offset)?;

    let pool_state = get_account(accounts, 0)?;
    let metadata = create_metadata(
        signature,
        slot,
        tx_index,
        block_time_us.unwrap_or_default(),
        grpc_recv_us,
    );

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state,
        user: get_account(accounts, 1).unwrap_or_default(),
        amount_in,
        amount_out: amount_out_min, // 先用指令中的最小值，日志会覆盖实际值
        is_buy: true, // 默认为买入，实际值从日志确定
        trade_direction: TradeDirection::Buy,
        exact_in: true,
    }))
}

/// 解析池创建指令
fn parse_pool_create_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let pool_state = get_account(accounts, 0)?;
    let metadata = create_metadata(
        signature,
        slot,
        tx_index,
        block_time_us.unwrap_or_default(),
        grpc_recv_us,
    );

    Some(DexEvent::BonkPoolCreate(BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol: "BONK".to_string(),
            name: "Bonk Pool".to_string(),
            uri: "https://bonk.com".to_string(),
            decimals: 5,
        },
        pool_state,
        creator: get_account(accounts, 1).unwrap_or_default(),
    }))
}

/// 解析 AMM 迁移指令
fn parse_migrate_amm_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let offset = 0;

    let liquidity_amount = read_u64_le(data, offset)?;

    let old_pool = get_account(accounts, 0)?;
    let metadata = create_metadata(
        signature,
        slot,
        tx_index,
        block_time_us.unwrap_or_default(),
        grpc_recv_us,
    );

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool,
        new_pool: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        liquidity_amount,
    }))
}

/// 解析 InitializeV2 指令
fn parse_initialize_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata(
        signature,
        slot,
        tx_index,
        block_time_us.unwrap_or_default(),
        grpc_recv_us,
    );

    let base_mint_param = parse_base_mint_param(data);

    Some(DexEvent::BonkInitializeV2(BonkInitializeV2Event {
        metadata,
        payer: get_account(accounts, 0).unwrap_or_default(),
        creator: get_account(accounts, 1).unwrap_or_default(),
        global_config: get_account(accounts, 2).unwrap_or_default(),
        platform_config: get_account(accounts, 3).unwrap_or_default(),
        authority: get_account(accounts, 4).unwrap_or_default(),
        pool_state: get_account(accounts, 5).unwrap_or_default(),
        base_mint: get_account(accounts, 6).unwrap_or_default(),
        quote_mint: get_account(accounts, 7).unwrap_or_default(),
        base_vault: get_account(accounts, 8).unwrap_or_default(),
        quote_vault: get_account(accounts, 9).unwrap_or_default(),
        base_mint_param,
    }))
}

fn parse_base_mint_param(data: &[u8]) -> BaseMintParam {
    let mut offset = 0usize;
    let mut param = BaseMintParam {
        symbol: String::new(),
        name: String::new(),
        uri: String::new(),
        decimals: 0,
    };

    let Some(decimals) = read_u8(data, offset) else {
        return param;
    };
    param.decimals = decimals;
    offset += 1;

    let Some((name, consumed)) = read_str_unchecked(data, offset) else {
        return param;
    };
    param.name = name.to_string();
    offset += consumed;

    let Some((symbol, consumed)) = read_str_unchecked(data, offset) else {
        return param;
    };
    param.symbol = symbol.to_string();
    offset += consumed;

    let Some((uri, _consumed)) = read_str_unchecked(data, offset) else {
        return param;
    };
    param.uri = uri.to_string();

    param
}
