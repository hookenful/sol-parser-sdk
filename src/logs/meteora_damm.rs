//! Meteora DAMM V2 日志解析器
//!
//! 解析 Meteora DAMM V2 程序的日志事件

use super::utils::*;
use crate::core::events::*;
use solana_sdk::signature::Signature;

/// Meteora DAMM V2 事件 discriminator 常量
pub mod discriminators {
    pub const SWAP_EVENT: [u8; 8] = [27, 60, 21, 213, 138, 170, 187, 147];
    pub const SWAP2_EVENT: [u8; 8] = [189, 66, 51, 168, 38, 80, 117, 153];
    pub const ADD_LIQUIDITY_EVENT: [u8; 8] = [175, 242, 8, 157, 30, 247, 185, 169];
    pub const REMOVE_LIQUIDITY_EVENT: [u8; 8] = [87, 46, 88, 98, 175, 96, 34, 91];
    pub const INITIALIZE_POOL_EVENT: [u8; 8] = [228, 50, 246, 85, 203, 66, 134, 37];
    pub const CREATE_POSITION_EVENT: [u8; 8] = [156, 15, 119, 198, 29, 181, 221, 55];
    pub const CLOSE_POSITION_EVENT: [u8; 8] = [20, 145, 144, 68, 143, 142, 214, 178];
    pub const CLAIM_POSITION_FEE_EVENT: [u8; 8] = [198, 182, 183, 52, 97, 12, 49, 56];
    pub const INITIALIZE_REWARD_EVENT: [u8; 8] = [129, 91, 188, 3, 246, 52, 185, 249];
    pub const FUND_REWARD_EVENT: [u8; 8] = [104, 233, 237, 122, 199, 191, 121, 85];
    pub const CLAIM_REWARD_EVENT: [u8; 8] = [218, 86, 147, 200, 235, 188, 215, 231];
}

/// 主要的 Meteora DAMM V2 日志解析函数
pub fn parse_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, tx_index, block_time_us, grpc_recv_us)
}

/// 解析结构化日志（基于 discriminator）
fn parse_structured_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let program_data = extract_program_data(log)?;

    if program_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = program_data[0..8].try_into().ok()?;
    let data = &program_data[8..];

    match discriminator {
        discriminators::SWAP_EVENT => {
            parse_swap_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::SWAP2_EVENT => {
            parse_swap2_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::ADD_LIQUIDITY_EVENT => {
            parse_add_liquidity_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::REMOVE_LIQUIDITY_EVENT => parse_remove_liquidity_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::INITIALIZE_POOL_EVENT => parse_initialize_pool_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CREATE_POSITION_EVENT => parse_create_position_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::CLOSE_POSITION_EVENT => {
            parse_close_position_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CLAIM_POSITION_FEE_EVENT => parse_claim_position_fee_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::INITIALIZE_REWARD_EVENT => parse_initialize_reward_event(
            data,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),
        discriminators::FUND_REWARD_EVENT => {
            parse_fund_reward_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        discriminators::CLAIM_REWARD_EVENT => {
            parse_claim_reward_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        }
        _ => None,
    }
}

/// 解析 Swap 事件
fn parse_swap_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // config (Pubkey - 32 bytes)
    let _config = read_pubkey(data, offset)?;
    offset += 32;

    // tradeDirection (u8 - 1 byte)
    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    // hasReferral (bool - 1 byte)
    let has_referral = read_bool(data, offset)?;
    offset += 1;

    // SwapParameters
    // params.amountIn (u64 - 8 bytes)
    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    // params.minimumAmountOut (u64 - 8 bytes)
    let minimum_amount_out = read_u64_le(data, offset)?;
    offset += 8;

    // SwapResult
    // swapResult.actual_input_amount (u64 - 8 bytes)
    let actual_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.output_amount (u64 - 8 bytes)
    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.next_sqrt_price (u128 - 16 bytes)
    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    // swapResult.trading_fee (u64 - 8 bytes)
    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.protocol_fee (u64 - 8 bytes)
    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.referral_fee (u64 - 8 bytes)
    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    // amount_in (u64 - 8 bytes) - 重复字段
    let _amount_in_dup = read_u64_le(data, offset)?;
    offset += 8;

    // currentTimestamp (u64 - 8 bytes)
    let current_timestamp = read_u64_le(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee: 0, // EvtSwap 没有 partner_fee
        referral_fee,
        actual_amount_in: actual_input_amount,
        current_timestamp,
        ..Default::default()
    }))
}

/// 解析 Swap2 事件 (EvtSwap2 格式)
fn parse_swap2_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;
    offset += 32;

    // trade_direction (u8 - 1 byte)
    let trade_direction = read_u8(data, offset)?;
    offset += 1;

    // collect_fee_mode (u8 - 1 byte)
    let _collect_fee_mode = read_u8(data, offset)?;
    offset += 1;

    // has_referral (bool - 1 byte)
    let has_referral = read_bool(data, offset)?;
    offset += 1;

    // SwapParameters2
    // params.amount_0 (u64 - 8 bytes)
    let amount_0 = read_u64_le(data, offset)?;
    offset += 8;

    // params.amount_1 (u64 - 8 bytes)
    let amount_1 = read_u64_le(data, offset)?;
    offset += 8;

    // params.swap_mode (u8 - 1 byte)
    let swap_mode = read_u8(data, offset)?;
    offset += 1;

    // SwapResult2
    // swapResult.included_fee_input_amount (u64 - 8 bytes)
    let included_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.excluded_fee_input_amount (u64 - 8 bytes)
    let _excluded_fee_input_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.amount_left (u64 - 8 bytes)
    let _amount_left = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.output_amount (u64 - 8 bytes)
    let output_amount = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.next_sqrt_price (u128 - 16 bytes)
    let next_sqrt_price = read_u128_le(data, offset)?;
    offset += 16;

    // swapResult.trading_fee (u64 - 8 bytes)
    let lp_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.protocol_fee (u64 - 8 bytes)
    let protocol_fee = read_u64_le(data, offset)?;
    offset += 8;

    // swapResult.referral_fee (u64 - 8 bytes)
    let referral_fee = read_u64_le(data, offset)?;
    offset += 8;

    // quote_reserve_amount (u64 - 8 bytes)
    let _quote_reserve_amount = read_u64_le(data, offset)?;
    offset += 8;

    // migration_threshold (u64 - 8 bytes)
    let _migration_threshold = read_u64_le(data, offset)?;
    offset += 8;

    // currentTimestamp (u64 - 8 bytes)
    let current_timestamp = read_u64_le(data, offset)?;

    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, pool, grpc_recv_us);

    // 根据 swap_mode 和 trade_direction 确定实际的 amount_in
    // swap_mode: 0 = ExactIn, 1 = ExactOut
    let (amount_in, minimum_amount_out) = if swap_mode == 0 {
        // ExactIn: amount_0 is amount_in, amount_1 is minimum_amount_out
        (amount_0, amount_1)
    } else {
        // ExactOut: amount_1 is maximum_amount_in, amount_0 is amount_out
        (amount_1, amount_0)
    };

    let actual_amount_in = included_fee_input_amount;

    Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
        metadata,
        pool,
        trade_direction,
        has_referral,
        amount_in,
        minimum_amount_out,
        output_amount,
        next_sqrt_price,
        lp_fee,
        protocol_fee,
        partner_fee: 0, // SwapResult2 没有 partner_fee
        referral_fee,
        actual_amount_in,
        current_timestamp,
        ..Default::default()
    }))
}

/// 解析 Add Liquidity 事件
fn parse_add_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let from = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let amounts = [
    //     read_u64_le(data, offset)?,
    //     read_u64_le(data, offset + 8)?,
    // ];
    // offset += 16;

    // let active_bin_id = read_i32_le(data, offset)?;

    // let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
    //     metadata,
    //     lb_pair,
    //     from,
    //     position,
    //     amounts,
    //     active_bin_id,
    // }))
    None
}

/// 解析 Remove Liquidity 事件
fn parse_remove_liquidity_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let from = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let amounts = [
    //     read_u64_le(data, offset)?,
    //     read_u64_le(data, offset + 8)?,
    // ];
    // offset += 16;

    // let active_bin_id = read_i32_le(data, offset)?;

    // let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
    //     metadata,
    //     lb_pair,
    //     from,
    //     position,
    //     amounts,
    //     active_bin_id,
    // }))
    None
}

/// 解析 Initialize Pool 事件
fn parse_initialize_pool_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let bin_step = read_u16_le(data, offset)?;
    // offset += 2;

    // let token_x = read_pubkey(data, offset)?;
    // offset += 32;

    // let token_y = read_pubkey(data, offset)?;

    // let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2InitializePool(MeteoraDammV2InitializePoolEvent {
    //     metadata,
    //     lb_pair,
    //     bin_step,
    //     token_x,
    //     token_y,
    // }))
    None
}

/// 解析 Create Position 事件
fn parse_create_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;

    // let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
    //     metadata,
    //     lb_pair,
    //     position,
    //     owner,
    // }))
    None
}

/// 解析 Close Position 事件
fn parse_close_position_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;

    // let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, position, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
    //     metadata,
    //     position,
    //     owner,
    // }))
    None
}

/// 解析 Claim Position Fee 事件
fn parse_claim_position_fee_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;
    // offset += 32;

    // let fee_x = read_u64_le(data, offset)?;
    // offset += 8;

    // let fee_y = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2ClaimPositionFee(MeteoraDammV2ClaimPositionFeeEvent {
    //     metadata,
    //     lb_pair,
    //     position,
    //     owner,
    //     fee_x,
    //     fee_y,
    // }))
    None
}

/// 解析 Initialize Reward 事件
fn parse_initialize_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_mint = read_pubkey(data, offset)?;
    // offset += 32;

    // let funder = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let reward_duration = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2InitializeReward(MeteoraDammV2InitializeRewardEvent {
    //     metadata,
    //     lb_pair,
    //     reward_mint,
    //     funder,
    //     reward_index,
    //     reward_duration,
    // }))
    None
}

/// 解析 Fund Reward 事件
fn parse_fund_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let funder = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let amount = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2FundReward(MeteoraDammV2FundRewardEvent {
    //     metadata,
    //     lb_pair,
    //     funder,
    //     reward_index,
    //     amount,
    // }))
    None
}

/// 解析 Claim Reward 事件
fn parse_claim_reward_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // let mut offset = 0;

    // let lb_pair = read_pubkey(data, offset)?;
    // offset += 32;

    // let position = read_pubkey(data, offset)?;
    // offset += 32;

    // let owner = read_pubkey(data, offset)?;
    // offset += 32;

    // let reward_index = read_u64_le(data, offset)?;
    // offset += 8;

    // let total_reward = read_u64_le(data, offset)?;

    // let metadata =
    //     create_metadata_simple(signature, slot, tx_index, block_time_us, lb_pair, grpc_recv_us);

    // Some(DexEvent::MeteoraDammV2ClaimReward(MeteoraDammV2ClaimRewardEvent {
    //     metadata,
    //     lb_pair,
    //     position,
    //     owner,
    //     reward_index,
    //     total_reward,
    // }))
    None
}

/// 解析文本格式日志
fn parse_text_log(
    _log: &str,
    _signature: Signature,
    _slot: u64,
    tx_index: u64,
    _block_time_us: Option<i64>,
) -> Option<DexEvent> {
    // 目前暂不实现文本解析，主要依赖结构化解析
    None
}
