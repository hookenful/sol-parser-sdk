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
    // 快速检查指令数据长度，避免无效解析
    if instruction_data.is_empty() {
        return None;
    }

    // 提前过滤和解析
    if let Some(filter) = event_type_filter {
        if let Some(ref include_only) = filter.include_only {
            let should_parse = include_only.iter().any(|t| {
                matches!(
                    t,
                    EventType::PumpFunMigrate
                        | EventType::MeteoraDammV2Swap
                        | EventType::MeteoraDammV2AddLiquidity
                        | EventType::MeteoraDammV2CreatePosition
                        | EventType::MeteoraDammV2ClosePosition
                        | EventType::MeteoraDammV2RemoveLiquidity
                )
            });
            if unlikely(!should_parse) {
                return None;
            }
        }
    }

    // 根据程序 ID 路由到相应的解析器，按使用频率排序

    // Pumpfun
    if *program_id == PUMPFUN_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpfun() {
            return None;
        }
        return parse_pumpfun_instruction(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }
    // PumpSwap (Pump AMM)
    else if *program_id == PUMPSWAP_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_pumpswap() {
            return None;
        }
        return parse_pumpswap_instruction(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        );
    }
    // Meteora DAMM
    else if *program_id == METEORA_DAMM_V2_PROGRAM_ID {
        if event_type_filter.is_some() && !event_type_filter.unwrap().includes_meteora_damm_v2() {
            return None;
        }
        return parse_meteora_damm_instruction(
            instruction_data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    None
}
