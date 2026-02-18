//! 指令解析器模块
//!
//! 包含所有 DEX 协议的指令解析器实现

pub mod meteora_amm;
pub mod meteora_damm;
pub mod meteora_dlmm;
pub mod orca_whirlpool;
pub mod program_ids;
pub mod pump;
pub mod pump_amm;
pub mod raydium_amm;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_launchpad;
pub mod utils;

// Inner instruction 解析器（16字节 discriminator）
pub mod all_inner;
pub mod inner_common; // 通用零拷贝读取函数
pub mod pump_amm_inner; // PumpSwap inner instruction
pub mod pump_inner; // PumpFun inner instruction
pub mod raydium_clmm_inner; // Raydium CLMM inner instruction // 其他所有协议的 inner instruction（统一文件）
use crate::grpc::types::{EventType, EventTypeFilter};
use crate::logs::perf_hints::unlikely;

// 重新导出主要解析函数
pub use meteora_damm::parse_instruction as parse_meteora_damm_instruction;
pub use pump::parse_instruction as parse_pumpfun_instruction;
pub use pump_amm::parse_instruction as parse_pumpswap_instruction;

// 重新导出工具函数
pub use utils::*;

use crate::core::events::DexEvent;
use program_ids::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionParseMode {
    Strict,
    Logless,
}

/// 统一的指令解析入口函数
#[inline]
pub fn parse_instruction_unified(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    program_id: &Pubkey,
) -> Option<DexEvent> {
    parse_instruction_unified_with_mode(
        instruction_data,
        accounts,
        signature,
        slot,
        tx_index,
        block_time_us,
        grpc_recv_us,
        event_type_filter,
        program_id,
        InstructionParseMode::Strict,
    )
}

#[inline]
pub fn parse_instruction_unified_with_mode(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    program_id: &Pubkey,
    parse_mode: InstructionParseMode,
) -> Option<DexEvent> {
    // 快速检查指令数据长度，避免无效解析
    if instruction_data.is_empty() {
        return None;
    }

    if unlikely(!should_parse_instruction_for_filter(event_type_filter, parse_mode)) {
        return None;
    }

    let event = if *program_id == PUMPFUN_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpfun() {
            return None;
        }
        pump::parse_instruction_with_mode(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
            matches!(parse_mode, InstructionParseMode::Logless),
        )
    } else if *program_id == PUMPSWAP_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpswap() {
            return None;
        }
        parse_pumpswap_instruction(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        )
    } else if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_meteora_damm_v2() {
            return None;
        }
        parse_meteora_damm_instruction(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        )
    } else {
        None
    }?;

    if matches!(parse_mode, InstructionParseMode::Logless)
        && !event_allowed_for_filter(&event, event_type_filter)
    {
        return None;
    }

    Some(event)
}

#[inline]
fn should_parse_instruction_for_filter(
    filter: Option<&EventTypeFilter>,
    parse_mode: InstructionParseMode,
) -> bool {
    let Some(filter) = filter else { return true };
    let Some(ref include_only) = filter.include_only else {
        return true;
    };

    let should_parse = include_only.iter().any(|t| {
        matches!(
            t,
            EventType::PumpFunMigrate
                | EventType::MeteoraDammV2Swap
                | EventType::MeteoraDammV2AddLiquidity
                | EventType::MeteoraDammV2CreatePosition
                | EventType::MeteoraDammV2ClosePosition
                | EventType::MeteoraDammV2RemoveLiquidity
        ) || matches!(parse_mode, InstructionParseMode::Logless)
            && matches!(
                t,
                EventType::PumpFunTrade
                    | EventType::PumpFunBuy
                    | EventType::PumpFunSell
                    | EventType::PumpFunBuyExactSolIn
                    | EventType::PumpFunCreate
                    | EventType::PumpSwapBuy
                    | EventType::PumpSwapSell
                    | EventType::PumpSwapCreatePool
                    | EventType::PumpSwapLiquidityAdded
                    | EventType::PumpSwapLiquidityRemoved
            )
    });

    should_parse
}

#[inline]
fn event_allowed_for_filter(event: &DexEvent, filter: Option<&EventTypeFilter>) -> bool {
    let Some(filter) = filter else { return true };

    if let Some(ref include_only) = filter.include_only {
        return event_matches_include(event, include_only);
    }

    if let Some(ref exclude_types) = filter.exclude_types {
        return !event_matches_exclude(event, exclude_types);
    }

    true
}

#[inline]
fn event_matches_include(event: &DexEvent, include_only: &[EventType]) -> bool {
    match event {
        DexEvent::PumpFunBuy(_) => {
            include_only.contains(&EventType::PumpFunBuy)
                || include_only.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunSell(_) => {
            include_only.contains(&EventType::PumpFunSell)
                || include_only.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunBuyExactSolIn(_) => {
            include_only.contains(&EventType::PumpFunBuyExactSolIn)
                || include_only.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunTrade(_) => include_only.contains(&EventType::PumpFunTrade),
        DexEvent::PumpFunCreate(_) => include_only.contains(&EventType::PumpFunCreate),
        DexEvent::PumpFunMigrate(_) => include_only.contains(&EventType::PumpFunMigrate),
        DexEvent::PumpSwapBuy(_) => include_only.contains(&EventType::PumpSwapBuy),
        DexEvent::PumpSwapSell(_) => include_only.contains(&EventType::PumpSwapSell),
        DexEvent::PumpSwapCreatePool(_) => include_only.contains(&EventType::PumpSwapCreatePool),
        DexEvent::PumpSwapLiquidityAdded(_) => {
            include_only.contains(&EventType::PumpSwapLiquidityAdded)
        }
        DexEvent::PumpSwapLiquidityRemoved(_) => {
            include_only.contains(&EventType::PumpSwapLiquidityRemoved)
        }
        DexEvent::MeteoraDammV2Swap(_) => include_only.contains(&EventType::MeteoraDammV2Swap),
        DexEvent::MeteoraDammV2AddLiquidity(_) => {
            include_only.contains(&EventType::MeteoraDammV2AddLiquidity)
        }
        DexEvent::MeteoraDammV2RemoveLiquidity(_) => {
            include_only.contains(&EventType::MeteoraDammV2RemoveLiquidity)
        }
        DexEvent::MeteoraDammV2CreatePosition(_) => {
            include_only.contains(&EventType::MeteoraDammV2CreatePosition)
        }
        DexEvent::MeteoraDammV2ClosePosition(_) => {
            include_only.contains(&EventType::MeteoraDammV2ClosePosition)
        }
        _ => false,
    }
}

#[inline]
fn event_matches_exclude(event: &DexEvent, exclude_types: &[EventType]) -> bool {
    match event {
        DexEvent::PumpFunBuy(_) => {
            exclude_types.contains(&EventType::PumpFunBuy)
                || exclude_types.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunSell(_) => {
            exclude_types.contains(&EventType::PumpFunSell)
                || exclude_types.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunBuyExactSolIn(_) => {
            exclude_types.contains(&EventType::PumpFunBuyExactSolIn)
                || exclude_types.contains(&EventType::PumpFunTrade)
        }
        DexEvent::PumpFunTrade(_) => exclude_types.contains(&EventType::PumpFunTrade),
        DexEvent::PumpFunCreate(_) => exclude_types.contains(&EventType::PumpFunCreate),
        DexEvent::PumpFunMigrate(_) => exclude_types.contains(&EventType::PumpFunMigrate),
        DexEvent::PumpSwapBuy(_) => exclude_types.contains(&EventType::PumpSwapBuy),
        DexEvent::PumpSwapSell(_) => exclude_types.contains(&EventType::PumpSwapSell),
        DexEvent::PumpSwapCreatePool(_) => exclude_types.contains(&EventType::PumpSwapCreatePool),
        DexEvent::PumpSwapLiquidityAdded(_) => {
            exclude_types.contains(&EventType::PumpSwapLiquidityAdded)
        }
        DexEvent::PumpSwapLiquidityRemoved(_) => {
            exclude_types.contains(&EventType::PumpSwapLiquidityRemoved)
        }
        DexEvent::MeteoraDammV2Swap(_) => exclude_types.contains(&EventType::MeteoraDammV2Swap),
        DexEvent::MeteoraDammV2AddLiquidity(_) => {
            exclude_types.contains(&EventType::MeteoraDammV2AddLiquidity)
        }
        DexEvent::MeteoraDammV2RemoveLiquidity(_) => {
            exclude_types.contains(&EventType::MeteoraDammV2RemoveLiquidity)
        }
        DexEvent::MeteoraDammV2CreatePosition(_) => {
            exclude_types.contains(&EventType::MeteoraDammV2CreatePosition)
        }
        DexEvent::MeteoraDammV2ClosePosition(_) => {
            exclude_types.contains(&EventType::MeteoraDammV2ClosePosition)
        }
        _ => false,
    }
}
