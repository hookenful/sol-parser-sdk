//! Optimized log matcher with early discriminator filtering
//!
//! Performance strategy:
//! 1. SIMD-based log type detection (~50ns)
//! 2. Extract discriminator BEFORE full parsing (~50ns)
//! 3. Check filter at discriminator level - skip parsing if not needed
//! 4. Only parse events user actually configured
//! 5. Compiler-optimized base64 decoding (auto-vectorized with target-cpu=native)

use super::perf_hints::{likely, unlikely};
use crate::core::events::{DexEvent, EventMetadata};
use crate::grpc::types::{EventType, EventTypeFilter};
use memchr::memmem;
use once_cell::sync::Lazy;
use solana_sdk::signature::Signature;

/// SIMD 优化的字符串查找器 - 预编译一次，重复使用
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
static RAYDIUM_AMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"));
static RAYDIUM_CLMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6"));
static RAYDIUM_CPMM_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD"));
static BONK_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b"));
static PROGRAM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"Program"));
static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: "));
static PUMPFUN_CREATE_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: G3KpTd7rY3Y"));
static WHIRL_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"whirL"));
static METEORA_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"meteora"));
static METEORA_LB_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"LB"));
static METEORA_DLMM_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"DLMM"));
static PUMPSWAP_LOWER_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"pumpswap"));
static PUMPSWAP_UPPER_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"PumpSwap"));

/// 预计算的程序 ID 字符串常量
pub mod program_id_strings {
    pub const PUMPFUN_INVOKE: &str = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke";
    pub const PUMPFUN_SUCCESS: &str = "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success";
    pub const PUMPFUN_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

    pub const BONK_INVOKE: &str = "Program Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b invoke";
    pub const BONK_SUCCESS: &str = "Program Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b success";
    pub const BONK_ID: &str = "Bxby5A7E8xPDGGc3FyJw7m5eK5aqNVLU83H2zLTQDH1b";

    pub const RAYDIUM_CLMM_INVOKE: &str =
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 invoke";
    pub const RAYDIUM_CLMM_SUCCESS: &str =
        "Program CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6 success";
    pub const RAYDIUM_CLMM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMdRBFSzKNT3t4ivN6";

    pub const RAYDIUM_CPMM_INVOKE: &str =
        "Program CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD invoke";
    pub const RAYDIUM_CPMM_SUCCESS: &str =
        "Program CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD success";
    pub const RAYDIUM_CPMM_ID: &str = "CPMDWBwJDtYax9qKcQP3CtKz7tHjJsN3H8hGrYVD9mZD";

    pub const RAYDIUM_AMM_V4_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    // 常用的日志模式
    pub const PROGRAM_DATA: &str = "Program data: ";
    pub const PROGRAM_LOG: &str = "Program log: ";

    // PumpFun 事件 discriminator (base64)
    pub const PUMPFUN_CREATE_DISCRIMINATOR: &str = "GB7IKAUcB3c"; // [24, 30, 200, 40, 5, 28, 7, 119]
}

/// 快速日志类型枚举
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LogType {
    PumpFun,
    RaydiumLaunchpad,
    PumpAmm,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumAmm,
    OrcaWhirlpool,
    MeteoraAmm,
    MeteoraDamm,
    MeteoraDlmm,
    Unknown,
}

/// SIMD 优化的日志类型检测器 - 激进早期退出
#[inline(always)]
pub fn detect_log_type(log: &str) -> LogType {
    let log_bytes = log.as_bytes();

    // 第一步：快速长度检查 - 太短的日志直接跳过
    if log_bytes.len() < 20 {
        return LogType::Unknown;
    }

    // 第二步：检查是否有 "Program data:" - 这是事件日志的标志
    let has_program_data = PROGRAM_DATA_FINDER.find(log_bytes).is_some();

    // 只有 "Program data:" 日志才可能是交易事件
    if unlikely(!has_program_data) {
        return LogType::Unknown;
    }

    // 第三步：使用 SIMD 快速检测具体协议
    // Raydium AMM - 高频，有明确程序ID（最常见）
    if likely(RAYDIUM_AMM_FINDER.find(log_bytes).is_some()) {
        return LogType::RaydiumAmm;
    }

    // Raydium CLMM
    if RAYDIUM_CLMM_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumClmm;
    }

    // Raydium CPMM
    if RAYDIUM_CPMM_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumCpmm;
    }

    // Raydium Launchpad (Bonk)
    if BONK_FINDER.find(log_bytes).is_some() {
        return LogType::RaydiumLaunchpad;
    }

    // Orca Whirlpool
    if WHIRL_FINDER.find(log_bytes).is_some() {
        return LogType::OrcaWhirlpool;
    }

    // Meteora - SIMD 优化
    if let Some(pos) = METEORA_FINDER.find(log_bytes) {
        let rest = &log_bytes[pos..];
        if METEORA_LB_FINDER.find(rest).is_some() {
            return LogType::MeteoraDamm;
        } else if METEORA_DLMM_FINDER.find(rest).is_some() {
            return LogType::MeteoraDlmm;
        } else {
            return LogType::MeteoraAmm;
        }
    }

    // Pump AMM
    if PUMPSWAP_LOWER_FINDER.find(log_bytes).is_some()
        || PUMPSWAP_UPPER_FINDER.find(log_bytes).is_some()
    {
        return LogType::PumpAmm;
    }

    // PumpFun - 特殊处理：可能有程序ID，也可能直接是base64数据
    // 1. 先检查是否包含程序ID（高频事件）
    if likely(PUMPFUN_FINDER.find(log_bytes).is_some()) {
        return LogType::PumpFun;
    }

    // 2. 兜底：有 "Program data:" 但无法识别协议的，尝试作为 PumpFun 解析
    // PumpFun的日志格式：Program data: <base64>
    // 只要日志够长且包含Program data，就认为可能是PumpFun
    if log.len() > 30 {
        return LogType::PumpFun;
    }

    LogType::Unknown
}

// ============================================================================
// Discriminator constants (compile-time computed) - All protocols
// ============================================================================
mod discriminators {
    // PumpFun discriminators
    pub const PUMPFUN_CREATE: u64 = u64::from_le_bytes([27, 114, 169, 77, 222, 235, 99, 118]);
    pub const PUMPFUN_TRADE: u64 = u64::from_le_bytes([189, 219, 127, 211, 78, 230, 97, 238]);
    pub const PUMPFUN_MIGRATE: u64 = u64::from_le_bytes([189, 233, 93, 185, 92, 148, 234, 148]);

    // PumpSwap discriminators
    pub const PUMPSWAP_BUY: u64 = u64::from_le_bytes([103, 244, 82, 31, 44, 245, 119, 119]);
    pub const PUMPSWAP_SELL: u64 = u64::from_le_bytes([62, 47, 55, 10, 165, 3, 220, 42]);
    pub const PUMPSWAP_CREATE_POOL: u64 =
        u64::from_le_bytes([177, 49, 12, 210, 160, 118, 167, 116]);
    pub const PUMPSWAP_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([120, 248, 61, 83, 31, 142, 107, 144]);
    pub const PUMPSWAP_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([22, 9, 133, 26, 160, 44, 71, 192]);

    // Raydium CLMM discriminators
    pub const RAYDIUM_CLMM_SWAP: u64 = u64::from_le_bytes([248, 198, 158, 145, 225, 117, 135, 200]);
    pub const RAYDIUM_CLMM_INCREASE_LIQUIDITY: u64 =
        u64::from_le_bytes([133, 29, 89, 223, 69, 238, 176, 10]);
    pub const RAYDIUM_CLMM_DECREASE_LIQUIDITY: u64 =
        u64::from_le_bytes([160, 38, 208, 111, 104, 91, 44, 1]);
    pub const RAYDIUM_CLMM_CREATE_POOL: u64 =
        u64::from_le_bytes([233, 146, 209, 142, 207, 104, 64, 188]);
    pub const RAYDIUM_CLMM_COLLECT_FEE: u64 =
        u64::from_le_bytes([164, 152, 207, 99, 187, 104, 171, 119]);

    // Raydium CPMM discriminators
    pub const RAYDIUM_CPMM_SWAP_BASE_IN: u64 =
        u64::from_le_bytes([143, 190, 90, 218, 196, 30, 51, 222]);
    pub const RAYDIUM_CPMM_SWAP_BASE_OUT: u64 =
        u64::from_le_bytes([55, 217, 98, 86, 163, 74, 180, 173]);
    pub const RAYDIUM_CPMM_CREATE_POOL: u64 =
        u64::from_le_bytes([233, 146, 209, 142, 207, 104, 64, 188]);
    pub const RAYDIUM_CPMM_DEPOSIT: u64 =
        u64::from_le_bytes([242, 35, 198, 137, 82, 225, 242, 182]);
    pub const RAYDIUM_CPMM_WITHDRAW: u64 =
        u64::from_le_bytes([183, 18, 70, 156, 148, 109, 161, 34]);

    // Raydium AMM V4 discriminators
    pub const RAYDIUM_AMM_SWAP_BASE_IN: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 9]);
    pub const RAYDIUM_AMM_SWAP_BASE_OUT: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 11]);
    pub const RAYDIUM_AMM_DEPOSIT: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 3]);
    pub const RAYDIUM_AMM_WITHDRAW: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 4]);
    pub const RAYDIUM_AMM_INITIALIZE2: u64 = u64::from_le_bytes([0, 0, 0, 0, 0, 0, 0, 1]);

    // Orca Whirlpool discriminators
    pub const ORCA_TRADED: u64 = u64::from_le_bytes([225, 202, 73, 175, 147, 43, 160, 150]);
    pub const ORCA_LIQUIDITY_INCREASED: u64 =
        u64::from_le_bytes([30, 7, 144, 181, 102, 254, 155, 161]);
    pub const ORCA_LIQUIDITY_DECREASED: u64 =
        u64::from_le_bytes([166, 1, 36, 71, 112, 202, 181, 171]);
    pub const ORCA_POOL_INITIALIZED: u64 =
        u64::from_le_bytes([100, 118, 173, 87, 12, 198, 254, 229]);

    // Meteora AMM discriminators
    pub const METEORA_AMM_SWAP: u64 = u64::from_le_bytes([81, 108, 227, 190, 205, 208, 10, 196]);
    pub const METEORA_AMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([31, 94, 125, 90, 227, 52, 61, 186]);
    pub const METEORA_AMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([116, 244, 97, 232, 103, 31, 152, 58]);
    pub const METEORA_AMM_BOOTSTRAP_LIQUIDITY: u64 =
        u64::from_le_bytes([121, 127, 38, 136, 92, 55, 14, 247]);
    pub const METEORA_AMM_POOL_CREATED: u64 =
        u64::from_le_bytes([202, 44, 41, 88, 104, 220, 157, 82]);

    // Meteora DAMM V2 discriminators
    pub const METEORA_DAMM_SWAP: u64 = u64::from_le_bytes([27, 60, 21, 213, 138, 170, 187, 147]);
    pub const METEORA_DAMM_SWAP2: u64 = u64::from_le_bytes([189, 66, 51, 168, 38, 80, 117, 153]);
    pub const METEORA_DAMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([175, 242, 8, 157, 30, 247, 185, 169]);
    pub const METEORA_DAMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([87, 46, 88, 98, 175, 96, 34, 91]);
    pub const METEORA_DAMM_INITIALIZE_POOL: u64 =
        u64::from_le_bytes([228, 50, 246, 85, 203, 66, 134, 37]);
    pub const METEORA_DAMM_CREATE_POSITION: u64 =
        u64::from_le_bytes([156, 15, 119, 198, 29, 181, 221, 55]);
    pub const METEORA_DAMM_CLOSE_POSITION: u64 =
        u64::from_le_bytes([20, 145, 144, 68, 143, 142, 214, 178]);

    // Meteora DLMM discriminators
    pub const METEORA_DLMM_SWAP: u64 = u64::from_le_bytes([143, 190, 90, 218, 196, 30, 51, 222]);
    pub const METEORA_DLMM_ADD_LIQUIDITY: u64 =
        u64::from_le_bytes([181, 157, 89, 67, 143, 182, 52, 72]);
    pub const METEORA_DLMM_REMOVE_LIQUIDITY: u64 =
        u64::from_le_bytes([80, 85, 209, 72, 24, 206, 35, 178]);
    pub const METEORA_DLMM_INITIALIZE_POOL: u64 =
        u64::from_le_bytes([95, 180, 10, 172, 84, 174, 232, 40]);
    pub const METEORA_DLMM_CREATE_POSITION: u64 =
        u64::from_le_bytes([123, 233, 11, 43, 146, 180, 97, 119]);
    pub const METEORA_DLMM_CLOSE_POSITION: u64 =
        u64::from_le_bytes([94, 168, 102, 45, 59, 122, 137, 54]);
}

/// Optimized unified log parser with **single-decode, early-filter** strategy
///
/// **Performance Strategy**:
/// 1. Decode base64 ONCE to stack buffer (~100ns)
/// 2. Extract discriminator from decoded data (~5ns)
/// 3. Check filter BEFORE parsing fields - return None if not wanted
/// 4. Parse only the specific event type requested
///
/// **Key optimization**: NO double base64 decoding!
/// Old: extract_discriminator(decode) -> parser(decode again) = 2x decode
/// New: decode once -> check filter -> parse from buffer = 1x decode
#[inline(always)]
/// `recent_blockhash`: pass as `Option<&[u8]>`; only cloned when an event is built (low latency).
pub fn parse_log_optimized(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
    event_type_filter: Option<&EventTypeFilter>,
    is_created_buy: bool,
    recent_blockhash: Option<&[u8]>,
) -> Option<DexEvent> {
    // Step 1: Find "Program data: " prefix using SIMD
    let log_bytes = log.as_bytes();
    let pos = PROGRAM_DATA_FINDER.find(log_bytes)?;
    let data_start = pos + 14; // "Program data: " length

    if log_bytes.len() <= data_start {
        return None;
    }

    // Step 2: Decode base64 ONCE to stack buffer (compiler auto-vectorized, zero heap allocation)
    let mut buf = [0u8; 2048]; // Increased back to 2048 to prevent buffer overflow panics
    let data_part = &log[data_start..];
    let trimmed = data_part.trim();

    // Validate input size before decoding (base64: 4 chars -> 3 bytes, so max input = (2048/3)*4 = ~2730 chars)
    // Add safety margin to prevent base64-simd assertion failures
    if trimmed.len() > 2700 {
        return None;
    }

    // SIMD-accelerated base64 decoding (AVX2/SSE4/NEON)
    use base64_simd::AsOut;
    let decoded_slice =
        base64_simd::STANDARD.decode(trimmed.as_bytes(), buf.as_mut().as_out()).ok()?;
    let decoded_len = decoded_slice.len();

    if decoded_len < 8 {
        return None;
    }

    let program_data = &buf[..decoded_len];

    // Step 3: Extract discriminator (~5ns, just read 8 bytes)
    let discriminator = unsafe {
        let ptr = program_data.as_ptr() as *const u64;
        ptr.read_unaligned()
    };

    // Step 4: Map discriminator to EventType for early filtering
    let event_type = discriminator_to_event_type(discriminator);

    // Step 5: Early filter check - BEFORE parsing any fields!
    if let Some(filter) = event_type_filter {
        if let Some(et) = event_type {
            if !filter.should_include(et) {
                return None; // Skip ALL parsing - saves ~200-500ns
            }
        } else {
            // Unknown discriminator - check if any supported protocol is wanted
            if let Some(ref include_only) = filter.include_only {
                let wants_supported = include_only.iter().any(|t| {
                    matches!(
                        t,
                        EventType::PumpFunTrade
                            | EventType::PumpFunCreate
                            | EventType::PumpFunMigrate
                            | EventType::PumpFunBuy
                            | EventType::PumpFunSell
                            | EventType::PumpFunBuyExactSolIn
                            | EventType::PumpSwapBuy
                            | EventType::PumpSwapSell
                            | EventType::PumpSwapCreatePool
                            | EventType::PumpSwapLiquidityAdded
                            | EventType::PumpSwapLiquidityRemoved
                    )
                });
                if !wants_supported {
                    return None;
                }
            }
        }
    }

    // Step 6: Parse the specific event type (data already decoded!)
    let data = &program_data[8..]; // Skip discriminator

    use crate::core::events::*;

    let metadata = EventMetadata {
        signature,
        slot,
        tx_index,
        block_time_us: block_time_us.unwrap_or(0),
        grpc_recv_us,
        recent_blockhash: recent_blockhash.map(|s| bs58::encode(s).into_string()),
    };

    // ========================================================================
    // Hot-path optimization: Fast check for top 5 most common discriminators
    // This avoids the large match statement for ~80% of events
    // Expected savings: 5-20ns per hot event
    // ========================================================================

    // Check hot-path discriminators first (ordered by frequency)
    if likely(discriminator == discriminators::PUMPFUN_TRADE) {
        // PumpFun Trade - Most common (~40% of all events)
        let event = crate::logs::pump::parse_trade_from_data(data, metadata, is_created_buy)?;
        // Secondary filter check
        if let Some(filter) = event_type_filter {
            if let Some(ref include_only) = filter.include_only {
                let has_specific_filter = include_only.iter().any(|t| {
                    matches!(
                        t,
                        EventType::PumpFunBuy
                            | EventType::PumpFunSell
                            | EventType::PumpFunBuyExactSolIn
                            | EventType::PumpFunCreate
                            | EventType::PumpFunCreateV2
                    )
                });
                if has_specific_filter {
                    let event_type_matches = match &event {
                        DexEvent::PumpFunBuy(_) => include_only.contains(&EventType::PumpFunBuy),
                        DexEvent::PumpFunSell(_) => include_only.contains(&EventType::PumpFunSell),
                        DexEvent::PumpFunBuyExactSolIn(_) => {
                            include_only.contains(&EventType::PumpFunBuyExactSolIn)
                        }
                        DexEvent::PumpFunTrade(_) => {
                            include_only.contains(&EventType::PumpFunTrade)
                        }
                        DexEvent::PumpFunCreate(_) => {
                            include_only.contains(&EventType::PumpFunCreate)
                        }
                        DexEvent::PumpFunCreateV2(_) => {
                            include_only.contains(&EventType::PumpFunCreateV2)
                        }
                        _ => false,
                    };
                    if !event_type_matches {
                        return None;
                    }
                }
            }
        }
        return Some(event);
    }

    if likely(discriminator == discriminators::RAYDIUM_CLMM_SWAP) {
        // Raydium CLMM Swap - High frequency (~20% of events)
        return crate::logs::raydium_clmm::parse_swap_from_data(data, metadata);
    }

    if likely(discriminator == discriminators::RAYDIUM_AMM_SWAP_BASE_IN) {
        // Raydium AMM Swap Base In - High frequency (~15% of events)
        return crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata);
    }

    if likely(discriminator == discriminators::PUMPSWAP_BUY) {
        // PumpSwap Buy - Medium frequency (~10% of events)
        return crate::logs::pump_amm::parse_buy_from_data(data, metadata);
    }

    if discriminator == discriminators::PUMPSWAP_SELL {
        // PumpSwap Sell - Medium frequency (~5% of events)
        return crate::logs::pump_amm::parse_sell_from_data(data, metadata);
    }

    // ========================================================================
    // Cold path: Handle remaining ~10% of events via match statement
    // ========================================================================

    match discriminator {
        // Note: Hot-path discriminators (PUMPFUN_TRADE, RAYDIUM_CLMM_SWAP, RAYDIUM_AMM_SWAP_BASE_IN,
        // PUMPSWAP_BUY, PUMPSWAP_SELL) are handled above and never reach this match statement

        // PumpFun events (cold path)
        discriminators::PUMPFUN_CREATE => crate::logs::pump::parse_create_from_data(data, metadata),
        discriminators::PUMPFUN_MIGRATE => {
            crate::logs::pump::parse_migrate_from_data(data, metadata)
        }

        // PumpSwap events (cold path)
        discriminators::PUMPSWAP_CREATE_POOL => {
            crate::logs::pump_amm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::PUMPSWAP_ADD_LIQUIDITY => {
            crate::logs::pump_amm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::PUMPSWAP_REMOVE_LIQUIDITY => {
            crate::logs::pump_amm::parse_remove_liquidity_from_data(data, metadata)
        }

        // ========== Other protocols - route by discriminator ==========
        // Raydium CLMM - use from_data functions (cold path)
        discriminators::RAYDIUM_CLMM_INCREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_DECREASE_LIQUIDITY => {
            crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_CREATE_POOL => {
            crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CLMM_COLLECT_FEE => {
            crate::logs::raydium_clmm::parse_collect_fee_from_data(data, metadata)
        }

        // Raydium CPMM - use from_data functions (single decode)
        discriminators::RAYDIUM_CPMM_SWAP_BASE_IN => {
            crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CPMM_SWAP_BASE_OUT => {
            crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
        }
        // Note: RAYDIUM_CPMM_CREATE_POOL discriminator conflicts with RAYDIUM_CLMM_CREATE_POOL
        // CPMM create pool is rare, handled via log content detection if needed
        discriminators::RAYDIUM_CPMM_DEPOSIT => {
            crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
        }
        discriminators::RAYDIUM_CPMM_WITHDRAW => {
            crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
        }

        // Raydium AMM V4 - use from_data functions (single decode)
        discriminators::RAYDIUM_AMM_SWAP_BASE_IN => {
            crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_SWAP_BASE_OUT => {
            crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_DEPOSIT => {
            crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_WITHDRAW => {
            crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
        }
        discriminators::RAYDIUM_AMM_INITIALIZE2 => {
            crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
        }

        // Orca Whirlpool - use from_data functions (single decode)
        discriminators::ORCA_TRADED => {
            crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
        }
        discriminators::ORCA_LIQUIDITY_INCREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
        }
        discriminators::ORCA_LIQUIDITY_DECREASED => {
            crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
        }
        discriminators::ORCA_POOL_INITIALIZED => {
            crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
        }

        // Meteora AMM - use from_data functions (single decode)
        discriminators::METEORA_AMM_SWAP => {
            crate::logs::meteora_amm::parse_swap_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_ADD_LIQUIDITY => {
            crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_REMOVE_LIQUIDITY => {
            crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_BOOTSTRAP_LIQUIDITY => {
            crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
        }
        discriminators::METEORA_AMM_POOL_CREATED => {
            crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
        }

        // Meteora DAMM V2
        discriminators::METEORA_DAMM_SWAP
        | discriminators::METEORA_DAMM_SWAP2
        | discriminators::METEORA_DAMM_ADD_LIQUIDITY
        | discriminators::METEORA_DAMM_REMOVE_LIQUIDITY
        | discriminators::METEORA_DAMM_INITIALIZE_POOL
        | discriminators::METEORA_DAMM_CREATE_POSITION
        | discriminators::METEORA_DAMM_CLOSE_POSITION => crate::logs::parse_meteora_damm_log(
            log,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        ),

        // NOTE: Meteora DLMM discriminators conflict with Raydium CPMM!
        // METEORA_DLMM_SWAP == RAYDIUM_CPMM_SWAP_BASE_IN
        // Handle DLMM in fallback using log content detection

        // Unknown discriminator - try fallback protocols
        _ => {
            // Try Meteora DLMM (has discriminator conflict with Raydium CPMM)
            if let Some(event) = crate::logs::parse_meteora_dlmm_log(
                log,
                signature,
                slot,
                tx_index,
                block_time_us,
                grpc_recv_us,
            ) {
                return Some(event);
            }
            None
        }
    }
}

/// Map discriminator to EventType (compile-time optimized match)
#[inline(always)]
fn discriminator_to_event_type(discriminator: u64) -> Option<EventType> {
    match discriminator {
        discriminators::PUMPFUN_CREATE => Some(EventType::PumpFunCreate),
        discriminators::PUMPFUN_TRADE => Some(EventType::PumpFunTrade),
        discriminators::PUMPFUN_MIGRATE => Some(EventType::PumpFunMigrate),
        discriminators::PUMPSWAP_BUY => Some(EventType::PumpSwapBuy),
        discriminators::PUMPSWAP_SELL => Some(EventType::PumpSwapSell),
        discriminators::PUMPSWAP_CREATE_POOL => Some(EventType::PumpSwapCreatePool),
        discriminators::PUMPSWAP_ADD_LIQUIDITY => Some(EventType::PumpSwapLiquidityAdded),
        discriminators::PUMPSWAP_REMOVE_LIQUIDITY => Some(EventType::PumpSwapLiquidityRemoved),
        _ => None,
    }
}

// ============================================================================
// SIMD utilities for log detection
// ============================================================================
#[inline]
pub fn detect_pumpfun_create(logs: &[String]) -> bool {
    logs.iter().any(|log| PUMPFUN_CREATE_FINDER.find(log.as_bytes()).is_some())
}

/// SIMD 优化的 "invoke [" 查找器
static INVOKE_FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(b"invoke ["));

/// 从日志中解析指令调用信息 (SIMD 优化版本)
/// 返回 (program_id, depth)
#[inline]
pub fn parse_invoke_info(log: &str) -> Option<(&str, usize)> {
    let log_bytes = log.as_bytes();

    // SIMD 快速查找 "invoke ["
    let invoke_start = INVOKE_FINDER.find(log_bytes)?;
    let bracket_start = invoke_start + 8; // "invoke [" 长度

    // 边界检查
    if bracket_start >= log_bytes.len() {
        return None;
    }

    // 解析深度数字，直到遇到 ']'
    let mut depth = 0usize;
    for &byte in &log_bytes[bracket_start..] {
        match byte {
            b'0'..=b'9' => {
                depth = depth * 10 + (byte - b'0') as usize;
            }
            b']' => break,
            _ => return None, // 遇到非数字非']'字符，解析失败
        }
    }

    // 提取程序ID：从 "Program " 开始到 " invoke" 结束
    if invoke_start < 8 {
        return None; // 没有足够空间放 "Program "
    }

    let program_start = 8; // "Program " 的长度
    let program_end = invoke_start - 1; // " invoke" 前面的空格位置

    if program_end <= program_start {
        return None;
    }

    let program_id = std::str::from_utf8(&log_bytes[program_start..program_end]).ok()?;

    Some((program_id, depth))
}
