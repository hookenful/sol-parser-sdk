//! Raydium CLMM Inner Instruction 解析器
//!
//! ## 解析器插件系统
//!
//! 本模块提供两种可插拔的解析器实现：
//!
//! ### 1. Borsh 反序列化解析器（默认，推荐）
//! - **启用**: `cargo build --features parse-borsh` （默认）
//! - **优点**: 类型安全、代码简洁、易维护、自动验证
//! - **适用**: 一般场景、需要稳定性和可维护性的项目
//!
//! ### 2. 零拷贝解析器（高性能）
//! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
//! - **优点**: 最快、零拷贝、无验证开销、适合超高频场景
//! - **适用**: 性能关键路径、每秒数万次解析的场景

use crate::core::events::*;
use crate::instr::inner_common::*;
use solana_sdk::pubkey::Pubkey;


/// Raydium CLMM inner instruction discriminators (16 bytes)
pub mod discriminators {
    /// SwapEvent
    pub const SWAP: [u8; 16] = [
        248, 198, 158, 145, 225, 117, 135, 200,
        155, 167, 108, 32, 122, 76, 173, 64,
    ];

    /// IncreaseLiquidityEvent
    pub const INCREASE_LIQUIDITY: [u8; 16] = [
        133, 29, 89, 223, 69, 238, 176, 10,
        155, 167, 108, 32, 122, 76, 173, 64,
    ];

    /// DecreaseLiquidityEvent
    pub const DECREASE_LIQUIDITY: [u8; 16] = [
        160, 38, 208, 111, 104, 91, 44, 1,
        155, 167, 108, 32, 122, 76, 173, 64,
    ];

    /// CreatePoolEvent
    pub const CREATE_POOL: [u8; 16] = [
        233, 146, 209, 142, 207, 104, 64, 188,
        155, 167, 108, 32, 122, 76, 173, 64,
    ];

    /// CollectFeeEvent
    pub const COLLECT_FEE: [u8; 16] = [
        164, 152, 207, 99, 187, 104, 171, 119,
        155, 167, 108, 32, 122, 76, 173, 64,
    ];
}

#[inline]
pub fn parse_raydium_clmm_inner_instruction(
    discriminator: &[u8; 16],
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    match discriminator {
        &discriminators::SWAP => parse_swap_inner(data, metadata),
        &discriminators::INCREASE_LIQUIDITY => parse_increase_liquidity_inner(data, metadata),
        &discriminators::DECREASE_LIQUIDITY => parse_decrease_liquidity_inner(data, metadata),
        &discriminators::CREATE_POOL => parse_create_pool_inner(data, metadata),
        &discriminators::COLLECT_FEE => parse_collect_fee_inner(data, metadata),
        _ => None,
    }
}

// ============================================================================
// Swap 事件解析器
// ============================================================================

/// 解析 Swap 事件（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_swap_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_swap_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_swap_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - Swap 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_swap_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool_state: Pubkey (32 bytes)
    // token_account_0: Pubkey (32 bytes)
    // token_account_1: Pubkey (32 bytes)
    // amount_0: u64 (8 bytes)
    // amount_1: u64 (8 bytes)
    // zero_for_one: bool (1 byte)
    // sqrt_price_x64: u128 (16 bytes)
    // liquidity: u128 (16 bytes)
    // Total: 145 bytes
    const SWAP_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8 + 1 + 16 + 16;

    if data.len() < SWAP_EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<RaydiumClmmSwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;

    Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器 - Swap 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_swap_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool_state: Pubkey (32 bytes)
    // token_account_0: Pubkey (32 bytes)
    // token_account_1: Pubkey (32 bytes)
    // amount_0: u64 (8 bytes)
    // amount_1: u64 (8 bytes)
    // zero_for_one: bool (1 byte)
    // sqrt_price_x64: u128 (16 bytes)
    // liquidity: u128 (16 bytes)
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 8 + 8 + 1 + 16 + 16) {
            return None;
        }

        let mut offset = 0;
        let pool_id = read_pubkey_unchecked(data, offset);
        offset += 32;
        let input_vault = read_pubkey_unchecked(data, offset);
        offset += 32;
        let output_vault = read_pubkey_unchecked(data, offset);
        offset += 32;
        let input_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let output_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let zero_for_one = read_bool_unchecked(data, offset);
        offset += 1;
        let sqrt_price_x64 = read_u128_unchecked(data, offset);
        offset += 16;
        let liquidity = read_u128_unchecked(data, offset);

        Some(DexEvent::RaydiumClmmSwap(RaydiumClmmSwapEvent {
            metadata,
            pool_state: pool_id,
            sender: Pubkey::default(),
            token_account_0: input_vault,
            token_account_1: output_vault,
            amount_0: input_amount,
            transfer_fee_0: 0,
            amount_1: output_amount,
            transfer_fee_1: 0,
            zero_for_one,
            sqrt_price_x64,
            liquidity,
            tick: 0,
        }))
    }
}

// ============================================================================
// IncreaseLiquidity 事件解析器
// ============================================================================

/// 解析 IncreaseLiquidity 事件（统一入口）
#[inline(always)]
fn parse_increase_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_increase_liquidity_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_increase_liquidity_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - IncreaseLiquidity 事件
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_increase_liquidity_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool: Pubkey (32 bytes)
    // position_nft_mint: Pubkey (32 bytes)
    // amount0_max: u64 (8 bytes)
    // amount1_max: u64 (8 bytes)
    // liquidity: u128 (16 bytes)
    // Total: 96 bytes
    const EVENT_SIZE: usize = 32 + 32 + 8 + 8 + 16;

    if data.len() < EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<RaydiumClmmIncreaseLiquidityEvent>(&data[..EVENT_SIZE]).ok()?;

    Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器 - IncreaseLiquidity 事件
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_increase_liquidity_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 16) {
            return None;
        }

        let mut offset = 0;
        let pool_id = read_pubkey_unchecked(data, offset);
        offset += 32;
        let position = read_pubkey_unchecked(data, offset);
        offset += 32;
        let token_0_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let token_1_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let liquidity = read_u128_unchecked(data, offset);

        Some(DexEvent::RaydiumClmmIncreaseLiquidity(RaydiumClmmIncreaseLiquidityEvent {
            metadata,
            pool: pool_id,
            position_nft_mint: position,
            user: Pubkey::default(),
            liquidity,
            amount0_max: token_0_amount,
            amount1_max: token_1_amount,
        }))
    }
}

// ============================================================================
// DecreaseLiquidity 事件解析器
// ============================================================================

/// 解析 DecreaseLiquidity 事件（统一入口）
#[inline(always)]
fn parse_decrease_liquidity_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_decrease_liquidity_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_decrease_liquidity_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - DecreaseLiquidity 事件
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_decrease_liquidity_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool: Pubkey (32 bytes)
    // position_nft_mint: Pubkey (32 bytes)
    // amount0_min: u64 (8 bytes)
    // amount1_min: u64 (8 bytes)
    // liquidity: u128 (16 bytes)
    // Total: 96 bytes
    const EVENT_SIZE: usize = 32 + 32 + 8 + 8 + 16;

    if data.len() < EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<RaydiumClmmDecreaseLiquidityEvent>(&data[..EVENT_SIZE]).ok()?;

    Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器 - DecreaseLiquidity 事件
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_decrease_liquidity_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8 + 16) {
            return None;
        }

        let mut offset = 0;
        let pool_id = read_pubkey_unchecked(data, offset);
        offset += 32;
        let position = read_pubkey_unchecked(data, offset);
        offset += 32;
        let token_0_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let token_1_amount = read_u64_unchecked(data, offset);
        offset += 8;
        let liquidity = read_u128_unchecked(data, offset);

        Some(DexEvent::RaydiumClmmDecreaseLiquidity(RaydiumClmmDecreaseLiquidityEvent {
            metadata,
            pool: pool_id,
            position_nft_mint: position,
            user: Pubkey::default(),
            liquidity,
            amount0_min: token_0_amount,
            amount1_min: token_1_amount,
        }))
    }
}

// ============================================================================
// CreatePool 事件解析器
// ============================================================================

/// 解析 CreatePool 事件（统一入口）
#[inline(always)]
fn parse_create_pool_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_create_pool_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_create_pool_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - CreatePool 事件
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_create_pool_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool: Pubkey (32 bytes)
    // token_0_mint: Pubkey (32 bytes)
    // token_1_mint: Pubkey (32 bytes)
    // tick_spacing: u16 (2 bytes)
    // fee_rate: u32 (4 bytes)
    // sqrt_price_x64: u128 (16 bytes)
    // Total: 118 bytes
    const EVENT_SIZE: usize = 32 + 32 + 32 + 2 + 4 + 16;

    if data.len() < EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<RaydiumClmmCreatePoolEvent>(&data[..EVENT_SIZE]).ok()?;

    Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器 - CreatePool 事件
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_create_pool_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 32 + 2 + 4 + 16) {
            return None;
        }

        let mut offset = 0;
        let pool_id = read_pubkey_unchecked(data, offset);
        offset += 32;
        let token_0_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let token_1_mint = read_pubkey_unchecked(data, offset);
        offset += 32;
        let tick_spacing = read_u16_unchecked(data, offset);
        offset += 2;
        let fee_rate = read_u32_unchecked(data, offset);
        offset += 4;
        let sqrt_price_x64 = read_u128_unchecked(data, offset);

        Some(DexEvent::RaydiumClmmCreatePool(RaydiumClmmCreatePoolEvent {
            metadata,
            pool: pool_id,
            token_0_mint,
            token_1_mint,
            tick_spacing,
            fee_rate,
            creator: Pubkey::default(),
            sqrt_price_x64,
            open_time: 0,
        }))
    }
}

// ============================================================================
// CollectFee 事件解析器
// ============================================================================

/// 解析 CollectFee 事件（统一入口）
#[inline(always)]
fn parse_collect_fee_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_collect_fee_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_collect_fee_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - CollectFee 事件
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_collect_fee_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // 数据结构:
    // pool_state: Pubkey (32 bytes)
    // position_nft_mint: Pubkey (32 bytes)
    // amount_0: u64 (8 bytes)
    // amount_1: u64 (8 bytes)
    // Total: 80 bytes
    const EVENT_SIZE: usize = 32 + 32 + 8 + 8;

    if data.len() < EVENT_SIZE {
        return None;
    }

    let event = borsh::from_slice::<RaydiumClmmCollectFeeEvent>(&data[..EVENT_SIZE]).ok()?;

    Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
        metadata,
        ..event
    }))
}

/// 零拷贝解析器 - CollectFee 事件
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_collect_fee_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if !check_length(data, 32 + 32 + 8 + 8) {
            return None;
        }

        let mut offset = 0;
        let pool_id = read_pubkey_unchecked(data, offset);
        offset += 32;
        let position = read_pubkey_unchecked(data, offset);
        offset += 32;
        let token_0_fee = read_u64_unchecked(data, offset);
        offset += 8;
        let token_1_fee = read_u64_unchecked(data, offset);

        Some(DexEvent::RaydiumClmmCollectFee(RaydiumClmmCollectFeeEvent {
            metadata,
            pool_state: pool_id,
            position_nft_mint: position,
            amount_0: token_0_fee,
            amount_1: token_1_fee,
        }))
    }
}
