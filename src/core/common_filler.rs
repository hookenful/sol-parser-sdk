use crate::core::invoke_context::{InvokeContext, InvokeLookup};
use crate::{core::events::*, instr::read_bool};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{Transaction, TransactionStatusMeta};

#[inline]
fn set_pumpswap_is_pump_pool_from_fees_ix<L: InvokeLookup + ?Sized>(
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &L,
    is_pump_pool: &mut bool,
) {
    if let Some(invoke) = program_invokes
        .get_invokes(&crate::grpc::program_ids::PUMPSWAP_FEES_PROGRAM)
        .and_then(|v| v.last())
    {
        if let Some(data) = get_instruction_data(meta, transaction, invoke) {
            *is_pump_pool = read_bool(data, 9).unwrap_or_default();
        }
    }
}

#[inline]
fn fill_data_with_lookup<L: InvokeLookup + ?Sized>(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &L,
) {
    match event {
        DexEvent::PumpSwapBuy(ref mut e) => {
            set_pumpswap_is_pump_pool_from_fees_ix(
                meta,
                transaction,
                program_invokes,
                &mut e.is_pump_pool,
            );
        }
        DexEvent::PumpSwapSell(ref mut e) => {
            set_pumpswap_is_pump_pool_from_fees_ix(
                meta,
                transaction,
                program_invokes,
                &mut e.is_pump_pool,
            );
        }
        _ => {}
    }
}

pub fn fill_data(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &HashMap<Pubkey, Vec<(i32, i32)>>,
) {
    fill_data_with_lookup(event, meta, transaction, program_invokes);
}

#[inline]
pub(crate) fn fill_data_with_invoke_context(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
    program_invokes: &InvokeContext,
) {
    fill_data_with_lookup(event, meta, transaction, program_invokes);
}

/// Fill PumpFun user post-transaction balances without any RPC calls.
///
/// `token_balance` is the raw base-unit balance of the event's `associated_user` token account.
/// `sol_balance` is the lamport balance of the event's `user` account. A token account closed by
/// the transaction has a final balance of zero.
#[inline]
pub fn fill_token_balances(
    event: &mut DexEvent,
    meta: &TransactionStatusMeta,
    transaction: &Option<Transaction>,
) {
    let trade = match event {
        DexEvent::PumpFunTrade(e)
        | DexEvent::PumpFunBuy(e)
        | DexEvent::PumpFunSell(e)
        | DexEvent::PumpFunBuyExactSolIn(e) => e,
        _ => return,
    };

    if let Some(user_index) = account_index(transaction, meta, &trade.user) {
        trade.sol_balance = meta.post_balances.get(user_index).copied();
    }

    if let Some(balance) = post_token_balance(transaction, meta, &trade.associated_user) {
        trade.token_balance = balance;
    }
}

#[inline]
fn account_index(
    transaction: &Option<Transaction>,
    meta: &TransactionStatusMeta,
    account: &Pubkey,
) -> Option<usize> {
    if *account == Pubkey::default() {
        return None;
    }

    let message = transaction.as_ref()?.message.as_ref()?;
    message
        .account_keys
        .iter()
        .chain(meta.loaded_writable_addresses.iter())
        .chain(meta.loaded_readonly_addresses.iter())
        .position(|key| key.as_slice() == account.as_ref())
}

#[inline]
fn post_token_balance(
    transaction: &Option<Transaction>,
    meta: &TransactionStatusMeta,
    account: &Pubkey,
) -> Option<Option<u64>> {
    if *account == Pubkey::default() {
        return None;
    }

    let message = transaction.as_ref()?.message.as_ref()?;
    let static_len = message.account_keys.len();
    let writable_len = meta.loaded_writable_addresses.len();

    let matches_account = |balance: &yellowstone_grpc_proto::prelude::TokenBalance| {
        let index = balance.account_index as usize;
        let key = if index < static_len {
            message.account_keys.get(index)
        } else if index < static_len + writable_len {
            meta.loaded_writable_addresses.get(index - static_len)
        } else {
            meta.loaded_readonly_addresses.get(index - static_len - writable_len)
        };

        key.is_some_and(|key| key.as_slice() == account.as_ref())
    };

    if let Some(balance) = meta.post_token_balances.iter().find(|balance| matches_account(balance))
    {
        return Some(crate::grpc::transaction_meta::try_token_balance_raw_amount(balance));
    }

    meta.pre_token_balances.iter().any(matches_account).then_some(Some(0))
}

pub fn get_instruction_data<'a>(
    meta: &'a TransactionStatusMeta,
    transaction: &'a Option<Transaction>,
    index: &(i32, i32), // (outer_index, inner_index)
) -> Option<&'a [u8]> {
    let data = if index.1 >= 0 {
        meta.inner_instructions
            .iter()
            .find(|i| i.index == index.0 as u32)?
            .instructions
            .get(index.1 as usize)?
            .data
            .as_slice()
    } else {
        transaction.as_ref()?.message.as_ref()?.instructions.get(index.0 as usize)?.data.as_slice()
    };
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yellowstone_grpc_proto::prelude::{Message, TokenBalance, UiTokenAmount};

    fn token_balance(account_index: u32, amount: u64) -> TokenBalance {
        TokenBalance {
            account_index,
            ui_token_amount: Some(UiTokenAmount {
                amount: amount.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn pumpfun_event(user: Pubkey, associated_user: Pubkey) -> DexEvent {
        DexEvent::PumpFunBuy(PumpFunTradeEvent {
            user,
            associated_user,
            is_buy: true,
            ..Default::default()
        })
    }

    fn transaction(accounts: &[Pubkey]) -> Option<Transaction> {
        Some(Transaction {
            message: Some(Message {
                account_keys: accounts.iter().map(|key| key.to_bytes().to_vec()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    #[test]
    fn fills_pumpfun_user_post_transaction_balances() {
        let user = Pubkey::new_unique();
        let associated_user = Pubkey::new_unique();
        let mut event = pumpfun_event(user, associated_user);
        let meta = TransactionStatusMeta {
            pre_balances: vec![50_000, 1_000],
            post_balances: vec![40_000, 1_000],
            pre_token_balances: vec![token_balance(1, 10)],
            post_token_balances: vec![token_balance(1, 35)],
            ..Default::default()
        };

        fill_token_balances(&mut event, &meta, &transaction(&[user, associated_user]));

        let DexEvent::PumpFunBuy(trade) = event else { panic!("expected PumpFun buy") };
        assert_eq!(trade.sol_balance, Some(40_000));
        assert_eq!(trade.token_balance, Some(35));
    }

    #[test]
    fn missing_token_balance_side_means_zero_for_created_or_closed_account() {
        let user = Pubkey::new_unique();
        let associated_user = Pubkey::new_unique();
        let tx = transaction(&[user, associated_user]);

        let mut created = pumpfun_event(user, associated_user);
        let created_meta = TransactionStatusMeta {
            pre_balances: vec![50, 0],
            post_balances: vec![40, 1],
            post_token_balances: vec![token_balance(1, 25)],
            ..Default::default()
        };
        fill_token_balances(&mut created, &created_meta, &tx);
        let DexEvent::PumpFunBuy(created) = created else { unreachable!() };
        assert_eq!(created.token_balance, Some(25));

        let mut closed = pumpfun_event(user, associated_user);
        let closed_meta = TransactionStatusMeta {
            pre_balances: vec![40, 1],
            post_balances: vec![45, 0],
            pre_token_balances: vec![token_balance(1, 25)],
            ..Default::default()
        };
        fill_token_balances(&mut closed, &closed_meta, &tx);
        let DexEvent::PumpFunBuy(closed) = closed else { unreachable!() };
        assert_eq!(closed.token_balance, Some(0));
    }

    #[test]
    fn ignores_non_pumpfun_events() {
        let mut event = DexEvent::Error("unchanged".to_string());

        fill_token_balances(&mut event, &TransactionStatusMeta::default(), &None);

        assert!(matches!(event, DexEvent::Error(ref message) if message == "unchanged"));
    }

    #[test]
    fn resolves_user_token_account_from_loaded_writable_addresses() {
        let user = Pubkey::new_unique();
        let associated_user = Pubkey::new_unique();
        let mut event = pumpfun_event(user, associated_user);
        let tx = transaction(&[user]);
        let meta = TransactionStatusMeta {
            pre_balances: vec![50, 1],
            post_balances: vec![40, 1],
            loaded_writable_addresses: vec![associated_user.to_bytes().to_vec()],
            pre_token_balances: vec![token_balance(1, 10)],
            post_token_balances: vec![token_balance(1, 35)],
            ..Default::default()
        };

        fill_token_balances(&mut event, &meta, &tx);

        let DexEvent::PumpFunBuy(trade) = event else { unreachable!() };
        assert_eq!(trade.token_balance, Some(35));
    }

    #[test]
    fn ignores_out_of_range_token_balance_account_index() {
        let user = Pubkey::new_unique();
        let associated_user = Pubkey::new_unique();
        let mut event = pumpfun_event(user, associated_user);
        let meta = TransactionStatusMeta {
            pre_balances: vec![50, 1],
            post_balances: vec![40, 1],
            pre_token_balances: vec![token_balance(99, 10)],
            post_token_balances: vec![token_balance(99, 35)],
            ..Default::default()
        };

        fill_token_balances(&mut event, &meta, &transaction(&[user, associated_user]));

        let DexEvent::PumpFunBuy(trade) = event else { unreachable!() };
        assert_eq!(trade.token_balance, None);
    }

    #[test]
    fn malformed_post_token_amount_is_unknown_not_zero() {
        let user = Pubkey::new_unique();
        let associated_user = Pubkey::new_unique();
        let mut event = pumpfun_event(user, associated_user);
        let meta = TransactionStatusMeta {
            pre_balances: vec![50, 1],
            post_balances: vec![40, 1],
            post_token_balances: vec![TokenBalance {
                account_index: 1,
                ui_token_amount: Some(UiTokenAmount {
                    amount: "invalid".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        fill_token_balances(&mut event, &meta, &transaction(&[user, associated_user]));

        let DexEvent::PumpFunBuy(trade) = event else { unreachable!() };
        assert_eq!(trade.token_balance, None);
        assert_eq!(trade.sol_balance, Some(40));
    }
}
