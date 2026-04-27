//! 轻量级事件合并机制 - 零拷贝高性能实现
//!
//! 将 inner instruction 事件数据合并到主 instruction 事件中
//! 设计原则:
//! - 只合并必要的字段
//! - 保持零拷贝特性
//! - 内联优化，最小化开销

use crate::core::events::*;

/// 合并 instruction 事件和 inner instruction 事件
///
/// # 设计
/// - Inner instruction 包含完整的交易数据（来自程序日志）
/// - Instruction 包含账户上下文（来自指令本身）
/// - 合并后的事件包含两者的完整信息
///
/// # 性能
/// - 内联优化，编译器会将其优化为直接赋值
/// - 零堆分配
/// - 预期开销 < 10ns
#[inline(always)]
pub fn merge_events(base: &mut DexEvent, inner: DexEvent) {
    use DexEvent::*;

    match (base, inner) {
        // ========== PumpFun 系列 ==========
        (PumpFunTrade(b), PumpFunTrade(i))
        | (PumpFunTrade(b), PumpFunBuy(i))
        | (PumpFunTrade(b), PumpFunSell(i))
        | (PumpFunTrade(b), PumpFunBuyExactSolIn(i))
        | (PumpFunBuy(b), PumpFunTrade(i))
        | (PumpFunBuy(b), PumpFunBuy(i))
        | (PumpFunSell(b), PumpFunTrade(i))
        | (PumpFunSell(b), PumpFunSell(i))
        | (PumpFunBuyExactSolIn(b), PumpFunTrade(i))
        | (PumpFunBuyExactSolIn(b), PumpFunBuyExactSolIn(i)) => merge_pumpfun_trade(b, i),

        (PumpFunCreate(b), PumpFunCreate(i)) => merge_pumpfun_create(b, i),
        (PumpFunCreateV2(b), PumpFunCreateV2(i)) => merge_generic(b, i),
        (PumpFunMigrate(b), PumpFunMigrate(i)) => merge_pumpfun_migrate(b, i),

        // ========== PumpSwap 系列 ==========
        (PumpSwapBuy(b), PumpSwapBuy(i)) => merge_generic(b, i),
        (PumpSwapSell(b), PumpSwapSell(i)) => merge_generic(b, i),
        (PumpSwapCreatePool(b), PumpSwapCreatePool(i)) => merge_generic(b, i),
        (PumpSwapLiquidityAdded(b), PumpSwapLiquidityAdded(i)) => merge_generic(b, i),
        (PumpSwapLiquidityRemoved(b), PumpSwapLiquidityRemoved(i)) => merge_generic(b, i),

        // ========== Raydium CLMM 系列 ==========
        (RaydiumClmmSwap(b), RaydiumClmmSwap(i)) => merge_generic(b, i),
        (RaydiumClmmIncreaseLiquidity(b), RaydiumClmmIncreaseLiquidity(i)) => merge_generic(b, i),
        (RaydiumClmmDecreaseLiquidity(b), RaydiumClmmDecreaseLiquidity(i)) => merge_generic(b, i),
        (RaydiumClmmCreatePool(b), RaydiumClmmCreatePool(i)) => merge_generic(b, i),
        (RaydiumClmmCollectFee(b), RaydiumClmmCollectFee(i)) => merge_generic(b, i),

        // ========== Raydium CPMM 系列 ==========
        (RaydiumCpmmSwap(b), RaydiumCpmmSwap(i)) => merge_generic(b, i),
        (RaydiumCpmmDeposit(b), RaydiumCpmmDeposit(i)) => merge_generic(b, i),
        (RaydiumCpmmWithdraw(b), RaydiumCpmmWithdraw(i)) => merge_generic(b, i),

        // ========== Raydium AMM V4 系列 ==========
        (RaydiumAmmV4Swap(b), RaydiumAmmV4Swap(i)) => merge_generic(b, i),
        (RaydiumAmmV4Deposit(b), RaydiumAmmV4Deposit(i)) => merge_generic(b, i),
        (RaydiumAmmV4Withdraw(b), RaydiumAmmV4Withdraw(i)) => merge_generic(b, i),

        // ========== Orca Whirlpool 系列 ==========
        (OrcaWhirlpoolSwap(b), OrcaWhirlpoolSwap(i)) => merge_generic(b, i),
        (OrcaWhirlpoolLiquidityIncreased(b), OrcaWhirlpoolLiquidityIncreased(i)) => {
            merge_generic(b, i)
        }
        (OrcaWhirlpoolLiquidityDecreased(b), OrcaWhirlpoolLiquidityDecreased(i)) => {
            merge_generic(b, i)
        }

        // ========== Meteora Pools (AMM) 系列 ==========
        (MeteoraPoolsSwap(b), MeteoraPoolsSwap(i)) => merge_generic(b, i),
        (MeteoraPoolsAddLiquidity(b), MeteoraPoolsAddLiquidity(i)) => merge_generic(b, i),
        (MeteoraPoolsRemoveLiquidity(b), MeteoraPoolsRemoveLiquidity(i)) => merge_generic(b, i),

        // ========== Meteora DAMM V2 系列 ==========
        (MeteoraDammV2Swap(b), MeteoraDammV2Swap(i)) => merge_generic(b, i),
        (MeteoraDammV2AddLiquidity(b), MeteoraDammV2AddLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2RemoveLiquidity(b), MeteoraDammV2RemoveLiquidity(i)) => merge_generic(b, i),
        (MeteoraDammV2CreatePosition(b), MeteoraDammV2CreatePosition(i)) => merge_generic(b, i),
        (MeteoraDammV2ClosePosition(b), MeteoraDammV2ClosePosition(i)) => merge_generic(b, i),

        // ========== Bonk 系列 ==========
        (BonkTrade(b), BonkTrade(i)) => merge_generic(b, i),

        // 其他组合不需要合并（类型不匹配）
        _ => {}
    }
}

/// 通用合并函数 - 对于大多数事件，inner instruction 包含完整数据
///
/// 这个函数简单地用 inner 的数据覆盖 base，因为：
/// - Inner instruction 来自程序日志，包含完整的交易数据
/// - Instruction 主要提供账户上下文
/// - 对于大多数协议，inner instruction 的数据已经足够完整
#[inline(always)]
fn merge_generic<T>(base: &mut T, inner: T) {
    *base = inner;
}

// ============================================================================
// PumpFun 事件合并实现
// ============================================================================

/// 合并 PumpFun Trade 事件
///
/// 合并策略:
/// - Inner instruction 提供: 交易数据（amount, reserves, fees 等）
/// - Instruction 提供: 账户上下文（bonding_curve, associated_bonding_curve 等）
/// - 合并后: 完整的交易事件
#[inline(always)]
fn merge_pumpfun_trade(base: &mut PumpFunTradeEvent, inner: PumpFunTradeEvent) {
    // 从 inner instruction 合并交易核心数据
    base.mint = inner.mint;
    base.sol_amount = inner.sol_amount;
    base.token_amount = inner.token_amount;
    base.is_buy = inner.is_buy;
    base.user = inner.user;
    base.timestamp = inner.timestamp;
    base.virtual_sol_reserves = inner.virtual_sol_reserves;
    base.virtual_token_reserves = inner.virtual_token_reserves;
    base.real_sol_reserves = inner.real_sol_reserves;
    base.real_token_reserves = inner.real_token_reserves;
    base.fee_recipient = inner.fee_recipient;
    base.fee_basis_points = inner.fee_basis_points;
    base.fee = inner.fee;
    base.creator = inner.creator;
    base.creator_fee_basis_points = inner.creator_fee_basis_points;
    base.creator_fee = inner.creator_fee;

    // 可选字段
    base.track_volume = inner.track_volume;
    base.total_unclaimed_tokens = inner.total_unclaimed_tokens;
    base.total_claimed_tokens = inner.total_claimed_tokens;
    base.current_sol_volume = inner.current_sol_volume;
    base.last_update_timestamp = inner.last_update_timestamp;
    base.ix_name = inner.ix_name;
    base.is_created_buy = inner.is_created_buy;
    base.mayhem_mode = inner.mayhem_mode;
    base.cashback_fee_basis_points = inner.cashback_fee_basis_points;
    base.cashback = inner.cashback;
    base.is_cashback_coin = inner.is_cashback_coin;

    // 保留 base 的账户上下文字段（bonding_curve, associated_bonding_curve 等）
    // 这些字段来自 instruction，不被 inner instruction 覆盖
}

/// 合并 PumpFun Create 事件
#[inline(always)]
fn merge_pumpfun_create(base: &mut PumpFunCreateTokenEvent, inner: PumpFunCreateTokenEvent) {
    // Inner instruction 包含完整的 create 数据
    base.name = inner.name;
    base.symbol = inner.symbol;
    base.uri = inner.uri;
    base.mint = inner.mint;
    base.bonding_curve = inner.bonding_curve;
    base.user = inner.user;
    base.creator = inner.creator;
    base.timestamp = inner.timestamp;
    base.virtual_token_reserves = inner.virtual_token_reserves;
    base.virtual_sol_reserves = inner.virtual_sol_reserves;
    base.real_token_reserves = inner.real_token_reserves;
    base.token_total_supply = inner.token_total_supply;
    base.token_program = inner.token_program;
    base.is_mayhem_mode = inner.is_mayhem_mode;
}

/// 合并 PumpFun Migrate 事件
#[inline(always)]
fn merge_pumpfun_migrate(base: &mut PumpFunMigrateEvent, inner: PumpFunMigrateEvent) {
    // Inner instruction 包含完整的 migrate 数据
    base.user = inner.user;
    base.mint = inner.mint;
    base.mint_amount = inner.mint_amount;
    base.sol_amount = inner.sol_amount;
    base.pool_migration_fee = inner.pool_migration_fee;
    base.bonding_curve = inner.bonding_curve;
    base.timestamp = inner.timestamp;
    base.pool = inner.pool;
}

// ============================================================================
// 工具函数
// ============================================================================

/// 判断两个事件是否可以合并
///
/// 合并条件:
/// 1. 都是同一个协议的事件
/// 2. 事件类型兼容（例如 Trade 和 Buy 可以合并）
/// 3. 来自同一个交易（signature 相同）
#[inline(always)]
pub fn can_merge(base: &DexEvent, inner: &DexEvent) -> bool {
    // 检查 signature 是否相同
    if base.metadata().signature != inner.metadata().signature {
        return false;
    }

    // 检查事件类型是否兼容
    match (base, inner) {
        // PumpFun Trade 系列事件可以互相合并
        (DexEvent::PumpFunTrade(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunBuy(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunSell(_))
        | (DexEvent::PumpFunTrade(_), DexEvent::PumpFunBuyExactSolIn(_))
        | (DexEvent::PumpFunBuy(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunBuy(_), DexEvent::PumpFunBuy(_))
        | (DexEvent::PumpFunSell(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunSell(_), DexEvent::PumpFunSell(_))
        | (DexEvent::PumpFunBuyExactSolIn(_), DexEvent::PumpFunTrade(_))
        | (DexEvent::PumpFunBuyExactSolIn(_), DexEvent::PumpFunBuyExactSolIn(_)) => true,

        // PumpFun Create / CreateV2 可以合并
        (DexEvent::PumpFunCreate(_), DexEvent::PumpFunCreate(_)) => true,
        (DexEvent::PumpFunCreateV2(_), DexEvent::PumpFunCreateV2(_)) => true,

        // PumpFun Migrate 可以合并
        (DexEvent::PumpFunMigrate(_), DexEvent::PumpFunMigrate(_)) => true,

        // 其他组合不支持合并
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{pubkey::Pubkey, signature::Signature};

    #[test]
    fn test_merge_pumpfun_trade() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        // Base event 来自 instruction（包含账户上下文）
        let mut base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            bonding_curve: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            ..Default::default()
        });

        // Inner event 来自 inner instruction（包含交易数据）
        let inner = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            mint: Pubkey::new_unique(),
            sol_amount: 1000,
            token_amount: 2000,
            is_buy: true,
            user: Pubkey::new_unique(),
            ..Default::default()
        });

        // 合并
        merge_events(&mut base, inner);

        // 验证合并结果
        if let DexEvent::PumpFunTrade(trade) = base {
            assert_eq!(trade.sol_amount, 1000);
            assert_eq!(trade.token_amount, 2000);
            assert!(trade.is_buy);
            // 账户上下文保留
            assert_ne!(trade.bonding_curve, Pubkey::default());
            assert_ne!(trade.associated_bonding_curve, Pubkey::default());
        } else {
            panic!("Expected PumpFunTrade event");
        }
    }

    #[test]
    fn test_can_merge() {
        let metadata = EventMetadata {
            signature: Signature::default(),
            slot: 100,
            tx_index: 1,
            block_time_us: 1000,
            grpc_recv_us: 2000,
            recent_blockhash: None,
        };

        let base = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ..Default::default()
        });

        let inner = DexEvent::PumpFunBuy(PumpFunTradeEvent {
            metadata: metadata.clone(),
            ..Default::default()
        });

        // 应该可以合并（同一个 signature，兼容类型）
        assert!(can_merge(&base, &inner));

        // 不同 signature 不能合并
        let different_sig = DexEvent::PumpFunTrade(PumpFunTradeEvent {
            metadata: EventMetadata { signature: Signature::new_unique(), ..metadata },
            ..Default::default()
        });

        assert!(!can_merge(&base, &different_sig));
    }
}
