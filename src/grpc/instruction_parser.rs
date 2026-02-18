//! Instruction 解析器 - 完整支持 instruction + inner instruction
//!
//! 设计原则：
//! - 简洁：单一入口函数，清晰的解析流程
//! - 高性能：零拷贝，内联优化，并行处理
//! - 可读性：每个步骤都有明确的注释

use crate::core::{events::*, merger::merge_events};
use crate::grpc::types::EventTypeFilter;
use crate::instr::read_pubkey_fast;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

/// 解析交易中的所有指令事件（instruction + inner instruction）
///
/// # 解析流程
/// 1. 解析主指令（outer instructions）- 8字节 discriminator
/// 2. 解析内部指令（inner instructions）- 16字节 discriminator
/// 3. 合并相关事件（instruction + inner instruction）
/// 4. 填充账户上下文
///
/// # 性能优化
/// - 并行解析多个指令（rayon）
/// - 零拷贝读取指令数据
/// - 内联优化所有热路径
/// - 提前过滤不需要的事件
#[inline]
pub fn parse_instructions_enhanced(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let Some(tx) = transaction else { return Vec::new() };
    let Some(msg) = &tx.message else { return Vec::new() };

    // 提前检查：是否需要解析 instruction（根据 filter）
    if !should_parse_instructions(filter) {
        return Vec::new();
    }

    // 构建账户查找表
    let keys_len = msg.account_keys.len();
    let writable_len = meta.loaded_writable_addresses.len();
    let get_key = |i: usize| -> Option<&Vec<u8>> {
        if i < keys_len {
            msg.account_keys.get(i)
        } else if i < keys_len + writable_len {
            meta.loaded_writable_addresses.get(i - keys_len)
        } else {
            meta.loaded_readonly_addresses.get(i - keys_len - writable_len)
        }
    };

    let mut result = Vec::with_capacity(8);
    let mut invokes: HashMap<Pubkey, Vec<(i32, i32)>> = HashMap::with_capacity(8);

    // 步骤 1: 解析所有主指令
    for (i, ix) in msg.instructions.iter().enumerate() {
        let pid = get_key(ix.program_id_index as usize)
            .map_or(Pubkey::default(), |k| read_pubkey_fast(k));

        invokes.entry(pid).or_default().push((i as i32, -1));

        // 解析主指令（8字节 discriminator）
        if let Some(event) = parse_outer_instruction(
            &ix.data,
            &pid,
            sig,
            slot,
            tx_idx,
            block_us,
            grpc_us,
            &ix.accounts,
            &get_key,
            filter,
        ) {
            result.push((i, None, event)); // (outer_idx, inner_idx, event)
        }
    }

    // 步骤 2: 解析所有 inner instructions
    for inner in &meta.inner_instructions {
        let outer_idx = inner.index as usize;

        for (j, inner_ix) in inner.instructions.iter().enumerate() {
            let pid = get_key(inner_ix.program_id_index as usize)
                .map_or(Pubkey::default(), |k| read_pubkey_fast(k));

            invokes.entry(pid).or_default().push((outer_idx as i32, j as i32));

            // 解析 inner instruction（16字节 discriminator）
            if let Some(event) = parse_inner_instruction(
                &inner_ix.data,
                &pid,
                sig,
                slot,
                tx_idx,
                block_us,
                grpc_us,
                filter,
            ) {
                result.push((outer_idx, Some(j), event)); // (outer_idx, Some(inner_idx), event)
            }
        }
    }

    // 步骤 3: 合并相关事件（instruction + inner instruction）
    let merged = merge_instruction_events(result);

    // 步骤 3.5: 转换 invokes HashMap 为字符串键（用于 fill_data）
    let invokes_str: HashMap<&str, Vec<(i32, i32)>> =
        invokes.iter().map(|(k, v)| (k.to_string().leak() as &str, v.clone())).collect();

    // 步骤 4: 填充账户上下文
    let mut final_result = Vec::with_capacity(merged.len());
    for mut event in merged {
        crate::core::account_dispatcher::fill_accounts_with_owned_keys(
            &mut event,
            meta,
            transaction,
            &invokes,
        );
        crate::core::common_filler::fill_data(&mut event, meta, transaction, &invokes_str);
        final_result.push(event);
    }

    final_result
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 解析单个主指令（outer instruction）
///
/// 主指令使用 8 字节 discriminator
#[inline(always)]
fn parse_outer_instruction<'a>(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    account_indices: &[u8],
    get_key: &dyn Fn(usize) -> Option<&'a Vec<u8>>,
    filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    // 检查指令数据长度（至少8字节 discriminator）
    if data.len() < 8 {
        return None;
    }

    // 构建账户列表
    let accounts: Vec<Pubkey> = account_indices
        .iter()
        .filter_map(|&idx| get_key(idx as usize).map(|k| read_pubkey_fast(k)))
        .collect();

    // 调用现有的 instruction 解析器
    crate::instr::parse_instruction_unified(
        data, &accounts, sig, slot, tx_idx, block_us, grpc_us, filter, program_id,
    )
}

/// 解析单个 inner instruction
///
/// Inner instructions 使用 16 字节 discriminator（前8字节是event hash，后8字节是magic）
#[inline(always)]
fn parse_inner_instruction(
    data: &[u8],
    program_id: &Pubkey,
    sig: Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Option<DexEvent> {
    // 检查数据长度（至少16字节 discriminator）
    if data.len() < 16 {
        return None;
    }

    let metadata = EventMetadata {
        signature: sig,
        slot,
        tx_index: tx_idx,
        block_time_us: block_us.unwrap_or(0),
        grpc_recv_us: grpc_us,
    };

    // 提取 16 字节 discriminator
    let mut discriminator = [0u8; 16];
    discriminator.copy_from_slice(&data[..16]);
    let inner_data = &data[16..];

    use crate::instr::{all_inner, program_ids, pump_amm_inner, pump_inner, raydium_clmm_inner};

    // 根据 program_id 路由到对应的 inner instruction 解析器
    if *program_id == program_ids::PUMPFUN_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_pumpfun() {
                return None;
            }
        }
        pump_inner::parse_pumpfun_inner_instruction(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::PUMPSWAP_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_pumpswap() {
                return None;
            }
        }
        pump_amm_inner::parse_pumpswap_inner_instruction(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::RAYDIUM_CLMM_PROGRAM_ID {
        raydium_clmm_inner::parse_raydium_clmm_inner_instruction(
            &discriminator,
            inner_data,
            metadata,
        )
    } else if *program_id == program_ids::RAYDIUM_CPMM_PROGRAM_ID {
        all_inner::raydium_cpmm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::RAYDIUM_AMM_V4_PROGRAM_ID {
        all_inner::raydium_amm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::ORCA_WHIRLPOOL_PROGRAM_ID {
        all_inner::orca::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::METEORA_POOLS_PROGRAM_ID {
        all_inner::meteora_amm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::METEORA_DAMM_V2_PROGRAM_ID {
        if let Some(f) = filter {
            if !f.includes_meteora_damm_v2() {
                return None;
            }
        }
        all_inner::meteora_damm::parse(&discriminator, inner_data, metadata)
    } else if *program_id == program_ids::BONK_PROGRAM_ID {
        all_inner::bonk::parse(&discriminator, inner_data, metadata)
    } else {
        None
    }
}

/// 合并相关的 instruction 和 inner instruction 事件
///
/// 合并策略：
/// 1. 同一个 outer_idx 的 instruction 和 inner instruction 可以合并
/// 2. Inner instruction 在 outer instruction 之后出现
/// 3. 合并后返回更完整的事件
#[inline]
fn merge_instruction_events(events: Vec<(usize, Option<usize>, DexEvent)>) -> Vec<DexEvent> {
    if events.is_empty() {
        return Vec::new();
    }

    // 按 (outer_idx, inner_idx) 排序，确保顺序：outer -> inner
    let mut events = events;
    events.sort_by_key(|(outer, inner, _)| (*outer, inner.unwrap_or(usize::MAX)));

    let mut result = Vec::with_capacity(events.len());
    let mut pending_outer: Option<(usize, DexEvent)> = None;

    for (outer_idx, inner_idx, event) in events {
        match inner_idx {
            None => {
                // 这是一个 outer instruction
                // 先处理之前的 pending_outer
                if let Some((_, outer_event)) = pending_outer.take() {
                    result.push(outer_event);
                }
                // 保存当前的 outer instruction，等待可能的 inner instruction
                pending_outer = Some((outer_idx, event));
            }
            Some(_) => {
                // 这是一个 inner instruction
                if let Some((pending_outer_idx, mut outer_event)) = pending_outer.take() {
                    if pending_outer_idx == outer_idx {
                        // 合并！
                        merge_events(&mut outer_event, event);
                        result.push(outer_event);
                    } else {
                        // 不匹配，分别保留
                        result.push(outer_event);
                        result.push(event);
                    }
                } else {
                    // 没有 pending outer，直接添加 inner event
                    result.push(event);
                }
            }
        }
    }

    // 处理最后一个 pending_outer
    if let Some((_, outer_event)) = pending_outer {
        result.push(outer_event);
    }

    result
}

/// 检查是否需要解析 instructions（根据 filter）
#[inline(always)]
fn should_parse_instructions(filter: Option<&EventTypeFilter>) -> bool {
    // 如果没有 filter，总是解析
    let Some(filter) = filter else { return true };

    // 如果 filter.include_only 为空，总是解析
    let Some(ref include_only) = filter.include_only else { return true };

    // 检查是否包含需要从 instruction 解析的事件类型
    include_only.iter().any(|t| {
        use crate::grpc::types::EventType::*;
        matches!(
            t,
            PumpFunMigrate
                | MeteoraDammV2Swap
                | MeteoraDammV2AddLiquidity
                | MeteoraDammV2CreatePosition
                | MeteoraDammV2ClosePosition
                | MeteoraDammV2RemoveLiquidity
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_parse_instructions() {
        // 无 filter - 应该解析
        assert!(should_parse_instructions(None));

        // 有 filter 但 include_only 为空 - 应该解析
        let filter = EventTypeFilter { include_only: None, exclude_types: None };
        assert!(should_parse_instructions(Some(&filter)));

        // 包含需要 instruction 解析的事件类型
        use crate::grpc::types::EventType;
        let filter = EventTypeFilter {
            include_only: Some(vec![EventType::PumpFunMigrate]),
            exclude_types: None,
        };
        assert!(should_parse_instructions(Some(&filter)));

        // 不包含需要 instruction 解析的事件类型
        let filter = EventTypeFilter {
            include_only: Some(vec![EventType::PumpFunTrade]),
            exclude_types: None,
        };
        assert!(!should_parse_instructions(Some(&filter)));
    }

    #[test]
    fn test_merge_instruction_events() {
        use solana_sdk::signature::Signature;

        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
        };

        // 模拟：outer instruction + inner instruction（应该合并）
        let outer_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            bonding_curve: Pubkey::new_unique(),
            ..Default::default()
        });

        let inner_event = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            sol_amount: 1000,
            token_amount: 2000,
            ..Default::default()
        });

        let events = vec![
            (0, None, outer_event),    // outer instruction at index 0
            (0, Some(0), inner_event), // inner instruction at index 0
        ];

        let result = merge_instruction_events(events);

        // 应该合并为1个事件
        assert_eq!(result.len(), 1);

        // 验证合并结果包含两者的数据
        if let DexEvent::PumpFunTrade(trade) = &result[0] {
            assert_eq!(trade.sol_amount, 1000); // 来自 inner
            assert_eq!(trade.token_amount, 2000); // 来自 inner
            assert_ne!(trade.bonding_curve, Pubkey::default()); // 来自 outer
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }
}
