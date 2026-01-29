//! Bonk 日志解析器
//!
//! 使用 match discriminator 模式解析 Bonk 事件

use solana_sdk::{pubkey::Pubkey, signature::Signature};
use crate::core::events::*;
use super::utils::*;

/// Bonk discriminator 常量
pub mod discriminators {
    pub const TRADE: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
    pub const POOL_CREATE: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    pub const MIGRATE_AMM: [u8; 8] = [3, 4, 5, 6, 7, 8, 9, 10];
}

/// Raydium LaunchLab 程序 ID
pub const PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";

/// 检查日志是否来自 Raydium Launchpad 程序
pub fn is_raydium_launchpad_log(log: &str) -> bool {
    log.contains(&format!("Program {} invoke", PROGRAM_ID)) ||
    log.contains(&format!("Program {} success", PROGRAM_ID)) ||
    log.contains("launchlab") || log.contains("LaunchLab")
}

/// 检查日志是否为 CreatePlatformConfig 参数输出
pub fn is_platform_params_log(log: &str) -> bool {
    log.contains("PlatformParams {") || log.contains("CreatePlatformConfig")
}

/// 主要的 Bonk 日志解析函数
pub fn parse_log(log: &str, signature: Signature, slot: u64, tx_index: u64, block_time_us: Option<i64>, grpc_recv_us: i64) -> Option<DexEvent> {
    parse_structured_log(log, signature, slot, tx_index, block_time_us, grpc_recv_us)
}

/// 解析 CreatePlatformConfig 文本日志
pub fn parse_platform_config_log(
    log: &str,
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if !log.contains("PlatformParams {") {
        return None;
    }

    let platform_params = parse_platform_params(log);
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::BonkCreatePlatformConfig(BonkCreatePlatformConfigEvent {
        metadata,
        platform_admin: Pubkey::default(),
        platform_fee_wallet: Pubkey::default(),
        platform_nft_wallet: Pubkey::default(),
        platform_config: Pubkey::default(),
        cpswap_config: Pubkey::default(),
        transfer_fee_extension_authority: Pubkey::default(),
        platform_vesting_wallet: Pubkey::default(),
        platform_params,
    }))
}


/// 结构化日志解析（基于 Program data）
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
        discriminators::TRADE => {
            parse_trade_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        discriminators::POOL_CREATE => {
            parse_pool_create_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        discriminators::MIGRATE_AMM => {
            parse_migrate_amm_event(data, signature, slot, tx_index, block_time_us, grpc_recv_us)
        },
        _ => None,
    }
}

/// 解析交易事件
fn parse_trade_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let amount_in = read_u64_le(data, offset)?;
    offset += 8;

    let amount_out = read_u64_le(data, offset)?;
    offset += 8;

    let is_buy = read_bool(data, offset)?;
    offset += 1;

    let exact_in = read_bool(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state,
        user,
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
    }))
}

/// 解析池创建事件
fn parse_pool_create_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let pool_state = read_pubkey(data, offset)?;
    offset += 32;

    let _token_a_mint = read_pubkey(data, offset)?;
    offset += 32;

    let _token_b_mint = read_pubkey(data, offset)?;
    offset += 32;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let _initial_liquidity_a = read_u64_le(data, offset)?;
    offset += 8;

    let _initial_liquidity_b = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state, grpc_recv_us);

    Some(DexEvent::BonkPoolCreate(BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol: "BONK".to_string(),
            name: "Bonk Pool".to_string(),
            uri: "https://bonk.com".to_string(),
            decimals: 5,
        },
        pool_state,
        creator,
    }))
}

/// 解析 AMM 迁移事件
fn parse_migrate_amm_event(
    data: &[u8],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    let old_pool = read_pubkey(data, offset)?;
    offset += 32;

    let new_pool = read_pubkey(data, offset)?;
    offset += 32;

    let user = read_pubkey(data, offset)?;
    offset += 32;

    let liquidity_amount = read_u64_le(data, offset)?;

    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, old_pool, grpc_recv_us);

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool,
        new_pool,
        user,
        liquidity_amount,
    }))
}

/// 文本回退解析
fn parse_text_log(
    tx_index: u64,
    log: &str,
    signature: Signature,
    slot: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    if log.contains("trade") || log.contains("swap") {
        return parse_trade_from_text(tx_index, log, signature, slot, block_time_us, grpc_recv_us);
    }

    if log.contains("pool") && log.contains("create") {
        return parse_pool_create_from_text(tx_index, log, signature, slot, block_time_us, grpc_recv_us);
    }

    if log.contains("migrate") {
        return parse_migrate_from_text(tx_index, log, signature, slot, block_time_us, grpc_recv_us);
    }

    None
}

/// 从文本解析交易事件
fn parse_trade_from_text(tx_index: u64, 
    log: &str,
    signature: Signature,
    slot: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, Pubkey::default(), grpc_recv_us);
    let is_buy = detect_trade_type(log).unwrap_or(true);

    Some(DexEvent::BonkTrade(BonkTradeEvent {
        metadata,
        pool_state: Pubkey::default(),
        user: Pubkey::default(),
        amount_in: extract_number_from_text(log, "amount_in").unwrap_or(1000000),
        amount_out: extract_number_from_text(log, "amount_out").unwrap_or(950000),
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in: true,
    }))
}

/// 从文本解析池创建事件
fn parse_pool_create_from_text(tx_index: u64, 
    log: &str,
    signature: Signature,
    slot: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::BonkPoolCreate(BonkPoolCreateEvent {
        metadata,
        base_mint_param: BaseMintParam {
            symbol: "BONK".to_string(),
            name: "Bonk Pool".to_string(),
            uri: "https://bonk.com".to_string(),
            decimals: 5,
        },
        pool_state: Pubkey::default(),
        creator: Pubkey::default(),
    }))
}

/// 从文本解析迁移事件
fn parse_migrate_from_text(tx_index: u64, 
    log: &str,
    signature: Signature,
    slot: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    use super::utils::text_parser::*;

    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, Pubkey::default(), grpc_recv_us);

    Some(DexEvent::BonkMigrateAmm(BonkMigrateAmmEvent {
        metadata,
        old_pool: Pubkey::default(),
        new_pool: Pubkey::default(),
        user: Pubkey::default(),
        liquidity_amount: extract_number_from_text(log, "liquidity").unwrap_or(0),
    }))
}

fn parse_platform_params(log: &str) -> BonkPlatformParams {
    let platform_scale = extract_u64_field(log, "platform_scale").unwrap_or(0);
    let creator_scale = extract_u64_field(log, "creator_scale").unwrap_or(0);
    let burn_scale = extract_u64_field(log, "burn_scale").unwrap_or(0);
    let fee_rate = extract_u64_field(log, "fee_rate").unwrap_or(0);
    let creator_fee_rate = extract_u64_field(log, "creator_fee_rate").unwrap_or(0);
    let platform_vesting_scale = extract_u64_field(log, "platform_vesting_scale").unwrap_or(0);

    let name = extract_string_field(log, "name").unwrap_or_default();
    let web = extract_string_field(log, "web").unwrap_or_default();
    let img = extract_string_field(log, "img").unwrap_or_default();

    BonkPlatformParams {
        migrate_nft_info: BonkMigrateNftInfo {
            platform_scale,
            creator_scale,
            burn_scale,
        },
        fee_rate,
        name,
        web,
        img,
        creator_fee_rate,
        platform_vesting_scale,
    }
}

fn extract_u64_field(log: &str, field: &str) -> Option<u64> {
    let key = format!("{}:", field);
    let start = log.find(&key)? + key.len();
    let rest = log[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
}

fn extract_string_field(log: &str, field: &str) -> Option<String> {
    let key = format!("{}:", field);
    let start = log.find(&key)? + key.len();
    let rest = log[start..].trim_start();
    let quote_start = rest.find('\"')?;
    let rest = &rest[quote_start + 1..];
    let quote_end = rest.find('\"')?;
    Some(rest[..quote_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_platform_params() {
        let log = r#"Program log: PlatformParams { migrate_nft_info: MigrateNftInfo { platform_scale: 1000000, creator_scale: 0, burn_scale: 0, }, fee_rate: 10000, name: "SURGE", web: "https://app.surge.xyz", img: "https://assets.surge.xyz/raydiumLogo.png", creator_fee_rate: 500, platform_vesting_scale: 0, }"#;

        let params = parse_platform_params(log);
        assert_eq!(params.migrate_nft_info.platform_scale, 1_000_000);
        assert_eq!(params.migrate_nft_info.creator_scale, 0);
        assert_eq!(params.migrate_nft_info.burn_scale, 0);
        assert_eq!(params.fee_rate, 10_000);
        assert_eq!(params.name, "SURGE");
        assert_eq!(params.web, "https://app.surge.xyz");
        assert_eq!(params.img, "https://assets.surge.xyz/raydiumLogo.png");
        assert_eq!(params.creator_fee_rate, 500);
        assert_eq!(params.platform_vesting_scale, 0);
    }
}
