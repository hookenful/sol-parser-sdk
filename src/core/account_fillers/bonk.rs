//! Bonk 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 填充 Bonk Trade 事件账户
///
/// Trade instruction account mapping (based on IDL):
/// 0: user
/// 1: poolState
/// 2: userTokenAccount
/// 3: poolTokenAccount
pub fn fill_trade_accounts(e: &mut BonkTradeEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(0);
    }
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(1);
    }
}

/// Bonk Pool Create 账户填充
///
/// createPool instruction account mapping (based on IDL):
/// 0: state
/// 1: pool
/// 2: tokenX
/// 3: tokenY
/// 4: poolXAccount
/// 5: poolYAccount
/// 6: adminXAccount
/// 7: adminYAccount
/// 8: admin
/// 9: projectOwner
/// 10: programAuthority
/// 11: systemProgram
/// 12: tokenProgram
/// 13: rent
pub fn fill_pool_create_accounts(e: &mut BonkPoolCreateEvent, get: &AccountGetter<'_>) {
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(1); // pool
    }
    if e.creator == Pubkey::default() {
        e.creator = get(8); // admin
    }
    // base_mint_param 已从事件数据或其他来源解析
}

/// Bonk InitializeV2 账户填充
///
/// initialize_v2 instruction account mapping (based on IDL):
/// 0: payer
/// 1: creator
/// 2: global_config
/// 3: platform_config
/// 4: authority
/// 5: pool_state
/// 6: base_mint
/// 7: quote_mint
/// 8: base_vault
/// 9: quote_vault
pub fn fill_initialize_v2_accounts(e: &mut BonkInitializeV2Event, get: &AccountGetter<'_>) {
    if e.payer == Pubkey::default() {
        e.payer = get(0);
    }
    if e.creator == Pubkey::default() {
        e.creator = get(1);
    }
    if e.global_config == Pubkey::default() {
        e.global_config = get(2);
    }
    if e.platform_config == Pubkey::default() {
        e.platform_config = get(3);
    }
    if e.authority == Pubkey::default() {
        e.authority = get(4);
    }
    if e.pool_state == Pubkey::default() {
        e.pool_state = get(5);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(6);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(7);
    }
    if e.base_vault == Pubkey::default() {
        e.base_vault = get(8);
    }
    if e.quote_vault == Pubkey::default() {
        e.quote_vault = get(9);
    }
}

/// Bonk CreatePlatformConfig 账户填充
///
/// create_platform_config instruction account mapping (based on IDL):
/// 0: platform_admin
/// 1: platform_fee_wallet
/// 2: platform_nft_wallet
/// 3: platform_config
/// 4: cpswap_config
/// 6: transfer_fee_extension_authority
/// 7: platform_vesting_wallet
pub fn fill_create_platform_config_accounts(e: &mut BonkCreatePlatformConfigEvent, get: &AccountGetter<'_>) {
    if e.platform_admin == Pubkey::default() {
        e.platform_admin = get(0);
    }
    if e.platform_fee_wallet == Pubkey::default() {
        e.platform_fee_wallet = get(1);
    }
    if e.platform_nft_wallet == Pubkey::default() {
        e.platform_nft_wallet = get(2);
    }
    if e.platform_config == Pubkey::default() {
        e.platform_config = get(3);
    }
    if e.cpswap_config == Pubkey::default() {
        e.cpswap_config = get(4);
    }
    if e.transfer_fee_extension_authority == Pubkey::default() {
        e.transfer_fee_extension_authority = get(6);
    }
    if e.platform_vesting_wallet == Pubkey::default() {
        e.platform_vesting_wallet = get(7);
    }
}
