//! PumpSwap 账户解析
//!
//! 提供 PumpSwap Global Config 和 Pool 账户的解析功能

use crate::core::events::{
    EventMetadata, PumpSwapGlobalConfig, PumpSwapGlobalConfigAccountEvent, PumpSwapPool,
    PumpSwapPoolAccountEvent,
};
use crate::DexEvent;

use super::token::AccountData;
use super::utils::*;

/// PumpSwap 账户 discriminators
pub mod discriminators {
    /// Global Config 账户的 discriminator
    pub const GLOBAL_CONFIG_ACCOUNT: &[u8] = &[149, 8, 156, 202, 160, 252, 176, 217];

    /// Pool 账户的 discriminator
    pub const POOL_ACCOUNT: &[u8] = &[241, 154, 109, 4, 17, 177, 109, 188];
}

/// Global Config 账户大小常量
pub const GLOBAL_CONFIG_SIZE: usize = 32 + 8 + 8 + 1 + 32 * 8 + 8 + 32;

/// Pool 账户大小常量
pub const POOL_SIZE: usize = 1 + 2 + 32 * 6 + 8 + 32;

/// 解析 PumpSwap Global Config 账户
///
/// # Arguments
/// * `account` - 账户数据
/// * `metadata` - 事件元数据
///
/// # Returns
/// 返回 `Some(DexEvent::PumpSwapGlobalConfigAccount)` 如果解析成功，否则返回 `None`
pub fn parse_global_config(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    // 检查账户数据长度（discriminator + data）
    if account.data.len() < GLOBAL_CONFIG_SIZE + 8 {
        return None;
    }

    // 检查 discriminator
    if !has_discriminator(&account.data, discriminators::GLOBAL_CONFIG_ACCOUNT) {
        return None;
    }

    // 解析 Global Config 数据（跳过 8 字节 discriminator）
    let data = &account.data[8..];
    let mut offset = 0;

    let admin = read_pubkey(data, offset)?;
    offset += 32;

    let lp_fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;

    let protocol_fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;

    let disable_flags = read_u8(data, offset)?;
    offset += 1;

    // 读取 8 个 protocol_fee_recipients
    let mut protocol_fee_recipients = [solana_sdk::pubkey::Pubkey::default(); 8];
    for i in 0..8 {
        protocol_fee_recipients[i] = read_pubkey(data, offset)?;
        offset += 32;
    }

    let coin_creator_fee_basis_points = read_u64_le(data, offset)?;
    offset += 8;

    let admin_set_coin_creator_authority = read_pubkey(data, offset)?;
    offset += 32;

    let whitelist_pda = read_pubkey(data, offset)?;
    offset += 32;

    let reserved_fee_recipient = read_pubkey(data, offset)?;
    offset += 32;

    let mayhem_mode_enabled = read_u8(data, offset)? != 0;
    offset += 1;

    // 读取 7 个 reserved_fee_recipients
    let mut reserved_fee_recipients = [solana_sdk::pubkey::Pubkey::default(); 7];
    for i in 0..7 {
        reserved_fee_recipients[i] = read_pubkey(data, offset)?;
        offset += 32;
    }

    let global_config = PumpSwapGlobalConfig {
        admin,
        lp_fee_basis_points,
        protocol_fee_basis_points,
        disable_flags,
        protocol_fee_recipients,
        coin_creator_fee_basis_points,
        admin_set_coin_creator_authority,
        whitelist_pda,
        reserved_fee_recipient,
        mayhem_mode_enabled,
        reserved_fee_recipients,
    };

    Some(DexEvent::PumpSwapGlobalConfigAccount(PumpSwapGlobalConfigAccountEvent {
        metadata,
        pubkey: account.pubkey,
        executable: account.executable,
        lamports: account.lamports,
        owner: account.owner,
        rent_epoch: account.rent_epoch,
        global_config,
    }))
}

/// 解析 PumpSwap Pool 账户
///
/// # Arguments
/// * `account` - 账户数据
/// * `metadata` - 事件元数据
///
/// # Returns
/// 返回 `Some(DexEvent::PumpSwapPoolAccount)` 如果解析成功，否则返回 `None`
pub fn parse_pool(account: &AccountData, metadata: EventMetadata) -> Option<DexEvent> {
    // 检查账户数据长度（discriminator + data）
    if account.data.len() < POOL_SIZE + 8 {
        return None;
    }

    // 检查 discriminator
    if !has_discriminator(&account.data, discriminators::POOL_ACCOUNT) {
        return None;
    }

    // 解析 Pool 数据（跳过 8 字节 discriminator）
    let data = &account.data[8..];
    let mut offset = 0;

    let pool_bump = read_u8(data, offset)?;
    offset += 1;

    let index = read_u16_le(data, offset)?;
    offset += 2;

    let creator = read_pubkey(data, offset)?;
    offset += 32;

    let base_mint = read_pubkey(data, offset)?;
    offset += 32;

    let quote_mint = read_pubkey(data, offset)?;
    offset += 32;

    let lp_mint = read_pubkey(data, offset)?;
    offset += 32;

    let pool_base_token_account = read_pubkey(data, offset)?;
    offset += 32;

    let pool_quote_token_account = read_pubkey(data, offset)?;
    offset += 32;

    let lp_supply = read_u64_le(data, offset)?;
    offset += 8;

    let coin_creator = read_pubkey(data, offset)?;

    let pool = PumpSwapPool {
        pool_bump,
        index,
        creator,
        base_mint,
        quote_mint,
        lp_mint,
        pool_base_token_account,
        pool_quote_token_account,
        lp_supply,
        coin_creator,
    };

    Some(DexEvent::PumpSwapPoolAccount(PumpSwapPoolAccountEvent {
        metadata,
        pubkey: account.pubkey,
        executable: account.executable,
        lamports: account.lamports,
        owner: account.owner,
        rent_epoch: account.rent_epoch,
        pool,
    }))
}

/// 检查账户是否是 PumpSwap Global Config 账户
pub fn is_global_config_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::GLOBAL_CONFIG_ACCOUNT)
}

/// 检查账户是否是 PumpSwap Pool 账户
pub fn is_pool_account(data: &[u8]) -> bool {
    has_discriminator(data, discriminators::POOL_ACCOUNT)
}
