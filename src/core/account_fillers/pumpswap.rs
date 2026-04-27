//! PumpSwap 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 通用的 PumpSwap 交易账户填充宏
///
/// PumpSwap Buy/Sell instruction account mapping (based on IDL):
/// 0: pool
/// 1: user
/// 2: authority
/// 3: baseMint
/// 4: quoteMint
/// 5: userBaseTokenAccount
/// 6: userQuoteTokenAccount
/// 7: poolBaseTokenAccount
/// 8: poolQuoteTokenAccount
/// 9: protocolFeeRecipient
/// 10: protocolFeeRecipientTokenAccount
/// 11: baseTokenProgram
/// 12: quoteTokenProgram
/// ... (13-16 optional system/associated token accounts)
/// 17: coinCreatorVaultAta (optional)
/// 18: coinCreatorVaultAuthority (optional)
macro_rules! fill_pumpswap_trade_common {
    ($event:expr, $get:expr) => {{
        let e = $event;
        let get = $get;

        if e.pool == Pubkey::default() {
            e.pool = get(0);
        }
        if e.user == Pubkey::default() {
            e.user = get(1);
        }
        if e.base_mint == Pubkey::default() {
            e.base_mint = get(3);
        }
        if e.quote_mint == Pubkey::default() {
            e.quote_mint = get(4);
        }
        if e.user_base_token_account == Pubkey::default() {
            e.user_base_token_account = get(5);
        }
        if e.user_quote_token_account == Pubkey::default() {
            e.user_quote_token_account = get(6);
        }
        if e.pool_base_token_account == Pubkey::default() {
            e.pool_base_token_account = get(7);
        }
        if e.pool_quote_token_account == Pubkey::default() {
            e.pool_quote_token_account = get(8);
        }
        if e.protocol_fee_recipient == Pubkey::default() {
            e.protocol_fee_recipient = get(9);
        }
        if e.protocol_fee_recipient_token_account == Pubkey::default() {
            e.protocol_fee_recipient_token_account = get(10);
        }
        if e.base_token_program == Pubkey::default() {
            e.base_token_program = get(11);
        }
        if e.quote_token_program == Pubkey::default() {
            e.quote_token_program = get(12);
        }
        if e.coin_creator_vault_ata == Pubkey::default() {
            e.coin_creator_vault_ata = get(17);
        }
        if e.coin_creator_vault_authority == Pubkey::default() {
            e.coin_creator_vault_authority = get(18);
        }
    }};
}

pub fn fill_buy_accounts(e: &mut PumpSwapBuyEvent, get: &AccountGetter<'_>) {
    fill_pumpswap_trade_common!(e, get);
}

pub fn fill_sell_accounts(e: &mut PumpSwapSellEvent, get: &AccountGetter<'_>) {
    fill_pumpswap_trade_common!(e, get);
}

pub fn fill_trade_accounts(_e: &mut PumpSwapTradeEvent, _get: &AccountGetter<'_>) {
    // PumpSwapTradeEvent is a different event structure (from IDL TradeEvent)
    // It doesn't have the same account fields as Buy/Sell events
    // All its fields are already parsed from the event data, no need to fill from instruction accounts
}

/// 填充 PumpSwap CreatePool 事件账户
///
/// CreatePool instruction account mapping (based on IDL):
/// 0: pool
/// 1: globalConfig
/// 2: creator
/// 3: baseMint
/// 4: quoteMint
/// 5: lpMint
/// 6: userBaseTokenAccount
/// 7: userQuoteTokenAccount
pub fn fill_create_pool_accounts(e: &mut PumpSwapCreatePoolEvent, get: &AccountGetter<'_>) {
    if e.pool == Pubkey::default() {
        e.pool = get(0);
    }
    if e.creator == Pubkey::default() {
        e.creator = get(2);
    }
    if e.base_mint == Pubkey::default() {
        e.base_mint = get(3);
    }
    if e.quote_mint == Pubkey::default() {
        e.quote_mint = get(4);
    }
    if e.lp_mint == Pubkey::default() {
        e.lp_mint = get(5);
    }
    if e.user_base_token_account == Pubkey::default() {
        e.user_base_token_account = get(6);
    }
    if e.user_quote_token_account == Pubkey::default() {
        e.user_quote_token_account = get(7);
    }
}

/// PumpSwap Liquidity Added 账户填充
///
/// deposit instruction account mapping (based on IDL):
/// 0: pool
/// 1: global_config
/// 2: user
/// 3: base_mint
/// 4: quote_mint
/// 5: lp_mint
/// 6: user_base_token_account
/// 7: user_quote_token_account
/// 8: user_pool_token_account
/// 9: pool_base_token_account
/// 10: pool_quote_token_account
/// 11: token_program
/// 12: token_2022_program
/// 13: event_authority
/// 14: program
pub fn fill_liquidity_added_accounts(_e: &mut PumpSwapLiquidityAdded, _get: &AccountGetter<'_>) {
    // 大部分字段已从事件数据解析
    // PumpSwapLiquidityAdded 事件结构不包含账户字段，只有数值字段
}

/// PumpSwap Liquidity Removed 账户填充
///
/// 注意：PumpSwap IDL 中没有明确的 removeLiquidity 指令
/// 此事件可能通过其他机制触发或暂未实现
pub fn fill_liquidity_removed_accounts(
    _e: &mut PumpSwapLiquidityRemoved,
    _get: &AccountGetter<'_>,
) {
    // 大部分字段已从事件数据解析
    // PumpSwapLiquidityRemoved 事件结构不包含账户字段，只有数值字段
}
