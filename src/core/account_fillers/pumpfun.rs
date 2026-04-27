//! PumpFun 账户填充模块

use crate::core::events::*;
use solana_sdk::pubkey::Pubkey;

/// 账户获取辅助函数类型
pub type AccountGetter<'a> = dyn Fn(usize) -> Pubkey + 'a;

/// 填充 PumpFun Trade 事件账户
///
/// PumpFun Buy/Sell instruction account mapping (from pumpfun.json IDL):
/// Buy 共 16 个固定账户 + 可选第 17 个:
/// 0 global, 1 fee_recipient, 2 mint, 3 bonding_curve, 4 associated_bonding_curve, 5 associated_user, 6 user,
/// 7 system_program, 8 token_program, 9 creator_vault, 10 event_authority, 11 program,
/// 12 global_volume_accumulator, 13 user_volume_accumulator, 14 fee_config, 15 fee_program,
/// 16 (可选) remaining_account / fee 相关，部分交易存在。
/// Sell 共 14 个固定账户，部分版本也有 17 个账户时 16 同 buy。
pub fn fill_trade_accounts(e: &mut PumpFunTradeEvent, get: &AccountGetter<'_>) {
    if e.user == Pubkey::default() {
        e.user = get(6);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(3);
    }
    if e.associated_bonding_curve == Pubkey::default() {
        e.associated_bonding_curve = get(4);
    }
    if e.creator_vault == Pubkey::default() {
        e.creator_vault = if e.is_buy { get(9) } else { get(8) };
    }
    if e.token_program == Pubkey::default() {
        e.token_program = if e.is_buy { get(8) } else { get(9) };
    }
    // 第 17 个账户 (index 16)：区块浏览器显示为 "Account"，getter 越界时返回 default，故仅非 default 时写入
    let a17 = get(16);
    if a17 != Pubkey::default() {
        e.account = Some(a17);
    }
}

/// 填充 PumpFun Create 事件账户
///
/// PumpFun Create instruction account mapping (based on IDL):
/// 0: mint
/// 1: mintAuthority
/// 2: bondingCurve
/// 3: associatedBondingCurve
/// 4: global
/// 5: mplTokenMetadata
/// 6: metadata
/// 7: user
/// 8: systemProgram
/// 9: tokenProgram
/// 10: associatedTokenProgram
/// 11: rent
/// 12: eventAuthority
/// 13: program
pub fn fill_create_accounts(e: &mut PumpFunCreateTokenEvent, get: &AccountGetter<'_>) {
    if e.mint == Pubkey::default() {
        e.mint = get(0);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(2);
    }
    if e.user == Pubkey::default() {
        e.user = get(7);
    }
}

/// 填充 PumpFun CreateV2 事件账户
///
/// CreateV2 instruction (idl create_v2): 0 mint, 1 mint_authority, 2 bonding_curve,
/// 3 associated_bonding_curve, 4 global, 5 user, 6 system_program, 7 token_program,
/// 8 associated_token_program, 9 mayhem_program_id, 10 global_params, 11 sol_vault,
/// 12 mayhem_state, 13 mayhem_token_vault, 14 event_authority, 15 program.
pub fn fill_create_v2_accounts(e: &mut PumpFunCreateV2TokenEvent, get: &AccountGetter<'_>) {
    if e.mint == Pubkey::default() {
        e.mint = get(0);
    }
    if e.bonding_curve == Pubkey::default() {
        e.bonding_curve = get(2);
    }
    if e.user == Pubkey::default() {
        e.user = get(5);
    }
    if e.mint_authority == Pubkey::default() {
        e.mint_authority = get(1);
    }
    if e.associated_bonding_curve == Pubkey::default() {
        e.associated_bonding_curve = get(3);
    }
    if e.global == Pubkey::default() {
        e.global = get(4);
    }
    if e.system_program == Pubkey::default() {
        e.system_program = get(6);
    }
    if e.token_program == Pubkey::default() {
        e.token_program = get(7);
    }
    if e.associated_token_program == Pubkey::default() {
        e.associated_token_program = get(8);
    }
    if e.mayhem_program_id == Pubkey::default() {
        e.mayhem_program_id = get(9);
    }
    if e.global_params == Pubkey::default() {
        e.global_params = get(10);
    }
    if e.sol_vault == Pubkey::default() {
        e.sol_vault = get(11);
    }
    if e.mayhem_state == Pubkey::default() {
        e.mayhem_state = get(12);
    }
    if e.mayhem_token_vault == Pubkey::default() {
        e.mayhem_token_vault = get(13);
    }
    if e.event_authority == Pubkey::default() {
        e.event_authority = get(14);
    }
    if e.program == Pubkey::default() {
        e.program = get(15);
    }
}

/// 填充 PumpFun Migrate 事件账户
pub fn fill_migrate_accounts(_e: &mut PumpFunMigrateEvent, _get: &AccountGetter<'_>) {
    // 暂未实现 - 需要 IDL
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证第 17 个账户 (account 字段) 在 gRPC/RPC 路径下会被正确填充：fill_trade_accounts 用 get(16) 写入 e.account
    #[test]
    fn test_fill_trade_accounts_account_17() {
        let account_17 = Pubkey::new_from_array([17u8; 32]);
        let get = |i: usize| -> Pubkey {
            if i == 16 {
                account_17
            } else {
                Pubkey::default()
            }
        };
        let getter: &AccountGetter = &get;

        let mut e = PumpFunTradeEvent { account: None, ..Default::default() };

        fill_trade_accounts(&mut e, getter);
        assert_eq!(
            e.account,
            Some(account_17),
            "第 17 个账户 (account) 应被填充，gRPC/RPC 路径均走 fill_trade_accounts"
        );
    }
}
