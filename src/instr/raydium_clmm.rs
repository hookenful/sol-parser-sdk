//! Raydium CLMM 指令解析器
//!
//! 使用 match discriminator 模式解析 Raydium CLMM 指令

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium CLMM discriminator 常量
/// 参考: solana-streamer/src/streaming/event_parser/protocols/raydium_clmm/events.rs
pub mod discriminators {
    pub const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    pub const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
    pub const INCREASE_LIQUIDITY_V2: [u8; 8] = [133, 29, 89, 223, 69, 238, 176, 10];
    pub const DECREASE_LIQUIDITY_V2: [u8; 8] = [58, 127, 188, 62, 79, 82, 196, 96]; // ✅ 修复：使用 V2 discriminator
    pub const CREATE_POOL: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];
    pub const OPEN_POSITION_V2: [u8; 8] = [77, 184, 74, 214, 112, 86, 241, 199];
    pub const OPEN_POSITION_WITH_TOKEN_22_NFT: [u8; 8] = [77, 255, 174, 82, 125, 29, 201, 46];
    pub const CLOSE_POSITION: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
}

/// Raydium CLMM 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_CLMM_PROGRAM_ID;

/// 主要的 Raydium CLMM 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::SWAP => {
            parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::SWAP_V2 => {
            parse_swap_v2_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::INCREASE_LIQUIDITY_V2 => parse_increase_liquidity_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::DECREASE_LIQUIDITY_V2 => parse_decrease_liquidity_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::CREATE_POOL => {
            parse_create_pool_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        discriminators::OPEN_POSITION_V2 => parse_open_position_v2_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::OPEN_POSITION_WITH_TOKEN_22_NFT => {
            parse_open_position_with_token_22_nft_instruction(
                data,
                accounts,
                signature,
                slot,
                tx_index,
                block_time_us,
            )
        }
        discriminators::CLOSE_POSITION => parse_close_position_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        _ => None,
    }
}

/// 解析交换指令
fn parse_swap_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let _amount = read_u64_le(data, offset)?;
    offset += 8;

    let _other_amount_threshold = read_u64_le(data, offset)?;
    offset += 8;

    let sqrt_price_limit_x64 = read_u64_le(data, offset)? as u128;
    offset += 8;

    let is_base_input = data.get(offset)? == &1;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,

        // IDL SwapEvent 事件字段
        pool_state: pool,
        sender: get_account(accounts, 1).unwrap_or_default(),
        token_account_0: Pubkey::default(),
        token_account_1: Pubkey::default(),
        amount_0: 0,       // 从日志填充
        transfer_fee_0: 0, // 从日志填充
        amount_1: 0,       // 从日志填充
        transfer_fee_1: 0, // 从日志填充
        zero_for_one: is_base_input,
        sqrt_price_x64: sqrt_price_limit_x64,
        // is_base_input,
        liquidity: 0, // 从日志填充
        tick: 0,      // 从日志填充

                      // 暂时注释，以后会用，AI禁止改动
                      // 指令参数字段
                      // amount,
                      // other_amount_threshold,
                      // sqrt_price_limit_x64,
    }))
}

/// 解析 Swap V2 指令（支持 Token2022）
fn parse_swap_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // SwapV2 与 Swap 参数相同，只是支持 Token2022
    parse_swap_instruction(data, accounts, signature, slot, tx_index, block_time_us)
}

/// 解析增加流动性 V2 指令
fn parse_increase_liquidity_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity = read_u64_le(data, offset)? as u128;
    offset += 8;

    let amount_0_max = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_max = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        pool,
        position_nft_mint: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        liquidity,
        amount0_max: amount_0_max,
        amount1_max: amount_1_max,
    }))
}

/// 解析减少流动性 V2 指令
fn parse_decrease_liquidity_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let liquidity = read_u64_le(data, offset)? as u128;
    offset += 8;

    let amount_0_min = read_u64_le(data, offset)?;
    offset += 8;

    let amount_1_min = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        pool,
        position_nft_mint: get_account(accounts, 1).unwrap_or_default(),
        user: get_account(accounts, 2).unwrap_or_default(),
        liquidity,
        amount0_min: amount_0_min,
        amount1_min: amount_1_min,
    }))
}

/// 解析池创建指令
fn parse_create_pool_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let sqrt_price_x64 = read_u64_le(data, offset)? as u128;
    offset += 8;

    let open_time = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        pool,
        token_0_mint: get_account(accounts, 2).unwrap_or_default(),
        token_1_mint: get_account(accounts, 3).unwrap_or_default(),
        tick_spacing: 0, // 从主指令解析
        fee_rate: 0,     // 从主指令解析
        creator: get_account(accounts, 1).unwrap_or_default(),
        sqrt_price_x64,
        open_time,
    }))
}

/// 解析开启头寸指令
fn parse_open_position_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0;

    let tick_lower_index = read_u32_le(data, offset)? as i32;
    offset += 4;

    let tick_upper_index = read_u32_le(data, offset)? as i32;
    offset += 4;

    let _tick_array_lower_start_index = read_u32_le(data, offset)? as i32;
    offset += 4;

    let _tick_array_upper_start_index = read_u32_le(data, offset)? as i32;
    offset += 4;

    let liquidity = read_u64_le(data, offset)? as u128;
    offset += 8;

    let _amount_0_max = read_u64_le(data, offset)?;
    offset += 8;

    let _amount_1_max = read_u64_le(data, offset)?;

    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmOpenPosition(RaydiumClmmOpenPositionEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        position_nft_mint: get_account(accounts, 2).unwrap_or_default(),
        tick_lower_index,
        tick_upper_index,
        liquidity,
    }))
}

/// 解析关闭头寸指令
fn parse_close_position_instruction(
    _data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let pool = get_account(accounts, 0)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool);

    Some(DexEvent::RaydiumClmmClosePosition(RaydiumClmmClosePositionEvent {
        metadata,
        pool,
        user: get_account(accounts, 1).unwrap_or_default(),
        position_nft_mint: get_account(accounts, 2).unwrap_or_default(),
    }))
}
/// 解析打开仓位 V2 指令
fn parse_open_position_v2_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // V2 版本与原版参数相同
    parse_open_position_instruction(data, accounts, signature, slot, tx_index, block_time_us)
}

/// 解析打开仓位（Token22 NFT）指令
fn parse_open_position_with_token_22_nft_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // Token22 NFT 版本与 V2 参数相同
    parse_open_position_v2_instruction(data, accounts, signature, slot, tx_index, block_time_us)
}
