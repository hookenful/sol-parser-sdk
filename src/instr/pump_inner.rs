//! PumpFun Inner Instruction 解析器
//!
//! Inner instructions 使用 16 字节的 discriminator（与 8 字节的 instruction 不同）
//! 这些是程序内部通过 CPI (Cross-Program Invocation) 触发的事件
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
//!
//! ## 使用示例
//!
//! ```bash
//! # 使用 Borsh 解析器（推荐，默认）
//! cargo build --release
//!
//! # 使用零拷贝解析器（极致性能）
//! cargo build --release --features parse-zero-copy --no-default-features
//! ```

use crate::core::events::*;

// ============================================================================
// Inner Instruction Discriminators (16 bytes)
// ============================================================================

/// PumpFun inner instruction discriminators
pub mod discriminators {
    /// TradeEvent discriminator (CPI log event)
    /// discriminator = sha256("event:TradeEvent")[..16]
    pub const TRADE_EVENT: [u8; 16] = [
        189, 219, 127, 211, 78, 230, 97, 238, // 前8字节
        155, 167, 108, 32, 122, 76, 173, 64, // 后8字节
    ];

    /// CreateTokenEvent discriminator
    pub const CREATE_TOKEN_EVENT: [u8; 16] =
        [27, 114, 169, 77, 222, 235, 99, 118, 155, 167, 108, 32, 122, 76, 173, 64];

    /// MigrateEvent discriminator (PumpAmm migration)
    pub const COMPLETE_PUMP_AMM_MIGRATION_EVENT: [u8; 16] =
        [189, 233, 93, 185, 92, 148, 234, 148, 155, 167, 108, 32, 122, 76, 173, 64];
}

// ============================================================================
// 零拷贝读取函数（仅用于 zero-copy 解析器）
// ============================================================================

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_u64_unchecked(data: &[u8], offset: usize) -> u64 {
    let ptr = data.as_ptr().add(offset) as *const u64;
    u64::from_le(ptr.read_unaligned())
}

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_i64_unchecked(data: &[u8], offset: usize) -> i64 {
    let ptr = data.as_ptr().add(offset) as *const i64;
    i64::from_le(ptr.read_unaligned())
}

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_bool_unchecked(data: &[u8], offset: usize) -> bool {
    *data.get_unchecked(offset) == 1
}

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_pubkey_unchecked(data: &[u8], offset: usize) -> solana_sdk::pubkey::Pubkey {
    use solana_sdk::pubkey::Pubkey;
    let ptr = data.as_ptr().add(offset);
    let mut bytes = [0u8; 32];
    std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), 32);
    Pubkey::new_from_array(bytes)
}

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_str_unchecked(data: &[u8], offset: usize) -> Option<(&str, usize)> {
    if data.len() < offset + 4 {
        return None;
    }

    let len = read_u32_unchecked(data, offset) as usize;
    if data.len() < offset + 4 + len {
        return None;
    }

    let string_bytes = &data[offset + 4..offset + 4 + len];
    let s = std::str::from_utf8_unchecked(string_bytes);
    Some((s, 4 + len))
}

#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
unsafe fn read_u32_unchecked(data: &[u8], offset: usize) -> u32 {
    let ptr = data.as_ptr().add(offset) as *const u32;
    u32::from_le(ptr.read_unaligned())
}

// ============================================================================
// Inner Instruction 解析函数
// ============================================================================

/// 解析 PumpFun inner instruction (统一入口)
///
/// # 参数
/// - `discriminator`: 16 字节的 inner instruction discriminator
/// - `data`: inner instruction 数据（不含 discriminator）
/// - `metadata`: 事件元数据
///
/// # 返回
/// 解析成功返回 `Some(DexEvent)`，否则返回 `None`
///
/// # is_created_buy
/// 当同笔交易内存在 PumpFun create 时由外层传入 true，表示创建者首次买入，与 log 解析行为一致
#[inline]
pub fn parse_pumpfun_inner_instruction(
    discriminator: &[u8; 16],
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    match discriminator {
        &discriminators::TRADE_EVENT => parse_trade_event_inner(data, metadata, is_created_buy),
        &discriminators::CREATE_TOKEN_EVENT => parse_create_event_inner(data, metadata),
        &discriminators::COMPLETE_PUMP_AMM_MIGRATION_EVENT => {
            parse_migrate_event_inner(data, metadata)
        }
        _ => None,
    }
}

// ============================================================================
// Trade 事件解析器
// ============================================================================

/// 解析 TradeEvent（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_trade_event_inner(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_trade_event_inner_borsh(data, metadata, is_created_buy)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_trade_event_inner_zero_copy(data, metadata, is_created_buy)
    }
}

/// Borsh 反序列化解析器 - Trade 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_trade_event_inner_borsh(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    // PumpFun TradeEvent 不是固定大小，因为包含 String 字段
    // 我们需要解析整个数据
    let mut event = borsh::from_slice::<PumpFunTradeEvent>(data).ok()?;
    event.metadata = metadata;
    event.is_created_buy = is_created_buy;
    event.is_cashback_coin = event.cashback_fee_basis_points > 0;

    // 根据 ix_name 返回不同的事件类型
    match event.ix_name.as_str() {
        "buy" => Some(DexEvent::PumpFunBuy(event)),
        "sell" => Some(DexEvent::PumpFunSell(event)),
        "buy_exact_sol_in" => Some(DexEvent::PumpFunBuyExactSolIn(event)),
        _ => Some(DexEvent::PumpFunTrade(event)),
    }
}

/// 零拷贝解析器 - Trade 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_trade_event_inner_zero_copy(
    data: &[u8],
    metadata: EventMetadata,
    is_created_buy: bool,
) -> Option<DexEvent> {
    unsafe {
        // 快速边界检查
        if data.len() < 32 + 8 + 8 + 1 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 8 + 8 + 32 + 8 + 8 {
            return None;
        }

        let mut offset = 0;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let token_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let is_buy = read_bool_unchecked(data, offset);
        offset += 1;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let fee_recipient = read_pubkey_unchecked(data, offset);
        offset += 32;

        let fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let fee = read_u64_unchecked(data, offset);
        offset += 8;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator_fee_basis_points = read_u64_unchecked(data, offset);
        offset += 8;

        let creator_fee = read_u64_unchecked(data, offset);
        offset += 8;

        // 可选字段
        let track_volume =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;

        let total_unclaimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let total_claimed_tokens =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let current_sol_volume =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let last_update_timestamp =
            if offset + 8 <= data.len() { read_i64_unchecked(data, offset) } else { 0 };
        offset += 8;

        let (ix_name, ix_name_len) = if offset + 4 <= data.len() {
            if let Some((s, consumed)) = read_str_unchecked(data, offset) {
                (s.to_string(), consumed)
            } else {
                (String::new(), 0)
            }
        } else {
            (String::new(), 0)
        };
        offset += ix_name_len;

        // TradeEvent 新增字段 (PUMP_CASHBACK_README): mayhem_mode, cashback_fee_basis_points, cashback
        let mayhem_mode =
            if offset + 1 <= data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;
        let cashback_fee_basis_points =
            if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };
        offset += 8;
        let cashback = if offset + 8 <= data.len() { read_u64_unchecked(data, offset) } else { 0 };

        // Inner instruction 只包含日志数据，不含指令上下文账户；is_created_buy 由外层根据同 tx 是否含 create 传入
        let trade_event = PumpFunTradeEvent {
            metadata,
            mint,
            sol_amount,
            token_amount,
            is_buy,
            is_created_buy,
            user,
            timestamp,
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            fee_recipient,
            fee_basis_points,
            fee,
            creator,
            creator_fee_basis_points,
            creator_fee,
            track_volume,
            total_unclaimed_tokens,
            total_claimed_tokens,
            current_sol_volume,
            last_update_timestamp,
            ix_name: ix_name.clone(),
            mayhem_mode,
            cashback_fee_basis_points,
            cashback,
            is_cashback_coin: cashback_fee_basis_points > 0,
            ..Default::default() // 其他账户字段由 instruction 提供
        };

        // 根据 ix_name 返回不同的事件类型
        match ix_name.as_str() {
            "buy" => Some(DexEvent::PumpFunBuy(trade_event)),
            "sell" => Some(DexEvent::PumpFunSell(trade_event)),
            "buy_exact_sol_in" => Some(DexEvent::PumpFunBuyExactSolIn(trade_event)),
            _ => Some(DexEvent::PumpFunTrade(trade_event)),
        }
    }
}

// ============================================================================
// Create 事件解析器
// ============================================================================

/// 解析 CreateTokenEvent（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_create_event_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_create_event_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_create_event_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - Create 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_create_event_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // CreateTokenEvent 包含多个 String 字段，不是固定大小
    let mut event = borsh::from_slice::<PumpFunCreateTokenEvent>(data).ok()?;
    event.metadata = metadata;
    Some(DexEvent::PumpFunCreate(event))
}

/// 零拷贝解析器 - Create 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_create_event_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        let mut offset = 0;

        let (name, name_len) = read_str_unchecked(data, offset)?;
        offset += name_len;

        let (symbol, symbol_len) = read_str_unchecked(data, offset)?;
        offset += symbol_len;

        let (uri, uri_len) = read_str_unchecked(data, offset)?;
        offset += uri_len;

        if data.len() < offset + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 1 {
            return None;
        }

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let creator = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let virtual_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let virtual_sol_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let real_token_reserves = read_u64_unchecked(data, offset);
        offset += 8;

        let token_total_supply = read_u64_unchecked(data, offset);
        offset += 8;

        let token_program = if offset + 32 <= data.len() {
            read_pubkey_unchecked(data, offset)
        } else {
            solana_sdk::pubkey::Pubkey::default()
        };
        offset += 32;

        let is_mayhem_mode =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };
        offset += 1;

        // IDL CreateEvent 最后一列: is_cashback_enabled
        let is_cashback_enabled =
            if offset < data.len() { read_bool_unchecked(data, offset) } else { false };

        Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
            metadata,
            name: name.to_string(),
            symbol: symbol.to_string(),
            uri: uri.to_string(),
            mint,
            bonding_curve,
            user,
            creator,
            timestamp,
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            token_total_supply,
            token_program,
            is_mayhem_mode,
            is_cashback_enabled,
        }))
    }
}

// ============================================================================
// Migrate 事件解析器
// ============================================================================

/// 解析 MigrateEvent（统一入口）
///
/// 根据编译时的 feature flag 自动选择解析器实现
#[inline(always)]
fn parse_migrate_event_inner(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    #[cfg(feature = "parse-borsh")]
    {
        parse_migrate_event_inner_borsh(data, metadata)
    }

    #[cfg(feature = "parse-zero-copy")]
    {
        parse_migrate_event_inner_zero_copy(data, metadata)
    }
}

/// Borsh 反序列化解析器 - Migrate 事件
///
/// **优点**: 类型安全、代码简洁、自动验证
#[cfg(feature = "parse-borsh")]
#[inline(always)]
fn parse_migrate_event_inner_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    // MigrateEvent 固定大小
    const MIGRATE_EVENT_SIZE: usize = 32 + 32 + 8 + 8 + 8 + 32 + 8 + 32; // 200 bytes

    if data.len() < MIGRATE_EVENT_SIZE {
        return None;
    }

    let mut event = borsh::from_slice::<PumpFunMigrateEvent>(&data[..MIGRATE_EVENT_SIZE]).ok()?;
    event.metadata = metadata;
    Some(DexEvent::PumpFunMigrate(event))
}

/// 零拷贝解析器 - Migrate 事件
///
/// **优点**: 最快、零拷贝、无验证开销
#[cfg(feature = "parse-zero-copy")]
#[inline(always)]
fn parse_migrate_event_inner_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    unsafe {
        if data.len() < 32 + 32 + 8 + 8 + 8 + 32 + 8 + 32 {
            return None;
        }

        let mut offset = 0;

        let user = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint = read_pubkey_unchecked(data, offset);
        offset += 32;

        let mint_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let sol_amount = read_u64_unchecked(data, offset);
        offset += 8;

        let pool_migration_fee = read_u64_unchecked(data, offset);
        offset += 8;

        let bonding_curve = read_pubkey_unchecked(data, offset);
        offset += 32;

        let timestamp = read_i64_unchecked(data, offset);
        offset += 8;

        let pool = read_pubkey_unchecked(data, offset);

        Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
            metadata,
            user,
            mint,
            mint_amount,
            sol_amount,
            pool_migration_fee,
            bonding_curve,
            timestamp,
            pool,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Signature;

    #[test]
    fn test_discriminator_match() {
        // 验证 discriminator 匹配
        let disc = discriminators::TRADE_EVENT;
        assert_eq!(disc.len(), 16);
    }

    #[test]
    fn test_parse_trade_event_boundary() {
        // 测试边界条件 - 数据不足
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 0,
            tx_index: 0,
            block_time_us: 0,
            grpc_recv_us: 0,
            recent_blockhash: None,
        };

        let short_data = vec![0u8; 10];
        let result = parse_trade_event_inner(&short_data, metadata, false);
        assert!(result.is_none());
    }
}
