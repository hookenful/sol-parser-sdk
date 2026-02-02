//! 所有协议的 Inner Instruction 解析器统一入口
//!
//! 采用简洁高效的实现方式，所有协议共享通用工具函数
//!
//! ## 解析器插件系统
//!
//! 所有协议支持两种可插拔的解析器实现：
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

use crate::core::events::*;
use crate::instr::inner_common::*;
use solana_sdk::pubkey::Pubkey;


// ============================================================================
// Raydium CPMM
// ============================================================================

pub mod raydium_cpmm {
    use super::*;

    pub mod discriminators {
        pub const SWAP_BASE_IN: [u8; 16] = [143, 190, 90, 218, 196, 30, 51, 222, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const SWAP_BASE_OUT: [u8; 16] = [55, 217, 98, 86, 163, 74, 180, 173, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const CREATE_POOL: [u8; 16] = [233, 146, 209, 142, 207, 104, 64, 188, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const DEPOSIT: [u8; 16] = [242, 35, 198, 137, 82, 225, 242, 182, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const WITHDRAW: [u8; 16] = [183, 18, 70, 156, 148, 109, 161, 34, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::SWAP_BASE_IN | &discriminators::SWAP_BASE_OUT => parse_swap(data, metadata),
            &discriminators::DEPOSIT => parse_deposit(data, metadata),
            &discriminators::WITHDRAW => parse_withdraw(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Swap 事件解析器
    // ============================================================================

    /// 解析 Swap 事件（统一入口）
    #[inline(always)]
    fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_swap_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_swap_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Swap 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // pool_id: Pubkey (32 bytes)
        // input_amount: u64 (8 bytes)
        // output_amount: u64 (8 bytes)
        // Total: 48 bytes
        const EVENT_SIZE: usize = 32 + 8 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumCpmmSwapEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Swap 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8) {
                return None;
            }
            let pool = read_pubkey_unchecked(data, 0);
            let input_amount = read_u64_unchecked(data, 32);
            let output_amount = read_u64_unchecked(data, 40);
            Some(DexEvent::RaydiumCpmmSwap(RaydiumCpmmSwapEvent {
                metadata,
                pool_id: pool,
                input_amount,
                output_amount,
                input_vault_before: 0,
                output_vault_before: 0,
                input_transfer_fee: 0,
                output_transfer_fee: 0,
                base_input: true,
            }))
        }
    }

    // ============================================================================
    // Deposit 事件解析器
    // ============================================================================

    /// 解析 Deposit 事件（统一入口）
    #[inline(always)]
    fn parse_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_deposit_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_deposit_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Deposit 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_deposit_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // pool: Pubkey (32 bytes)
        // token0_amount: u64 (8 bytes)
        // token1_amount: u64 (8 bytes)
        // lp_token_amount: u64 (8 bytes)
        // Total: 56 bytes
        const EVENT_SIZE: usize = 32 + 8 + 8 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumCpmmDepositEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Deposit 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_deposit_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8 + 8) {
                return None;
            }
            let pool = read_pubkey_unchecked(data, 0);
            let token0_amount = read_u64_unchecked(data, 32);
            let token1_amount = read_u64_unchecked(data, 40);
            let lp_token_amount = read_u64_unchecked(data, 48);
            Some(DexEvent::RaydiumCpmmDeposit(RaydiumCpmmDepositEvent {
                metadata,
                pool,
                lp_token_amount,
                token0_amount,
                token1_amount,
                user: Pubkey::default(),
            }))
        }
    }

    // ============================================================================
    // Withdraw 事件解析器
    // ============================================================================

    /// 解析 Withdraw 事件（统一入口）
    #[inline(always)]
    fn parse_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_withdraw_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_withdraw_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Withdraw 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_withdraw_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // pool: Pubkey (32 bytes)
        // lp_token_amount: u64 (8 bytes)
        // token0_amount: u64 (8 bytes)
        // token1_amount: u64 (8 bytes)
        // Total: 56 bytes
        const EVENT_SIZE: usize = 32 + 8 + 8 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumCpmmWithdrawEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Withdraw 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_withdraw_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8 + 8) {
                return None;
            }
            let pool = read_pubkey_unchecked(data, 0);
            let lp_token_amount = read_u64_unchecked(data, 32);
            let token0_amount = read_u64_unchecked(data, 40);
            let token1_amount = read_u64_unchecked(data, 48);
            Some(DexEvent::RaydiumCpmmWithdraw(RaydiumCpmmWithdrawEvent {
                metadata,
                pool,
                lp_token_amount,
                token0_amount,
                token1_amount,
                user: Pubkey::default(),
            }))
        }
    }
}

// ============================================================================
// Raydium AMM V4
// ============================================================================

pub mod raydium_amm {
    use super::*;

    pub mod discriminators {
        pub const SWAP_BASE_IN: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 9, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const SWAP_BASE_OUT: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 11, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const DEPOSIT: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 3, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const WITHDRAW: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 4, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const INITIALIZE2: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 1, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::SWAP_BASE_IN | &discriminators::SWAP_BASE_OUT => parse_swap(data, metadata),
            &discriminators::DEPOSIT => parse_deposit(data, metadata),
            &discriminators::WITHDRAW => parse_withdraw(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Swap 事件解析器
    // ============================================================================

    /// 解析 Swap 事件（统一入口）
    #[inline(always)]
    fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_swap_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_swap_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Swap 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // amm: Pubkey (32 bytes)
        // amount_in: u64 (8 bytes)
        // amount_out: u64 (8 bytes)
        // Total: 48 bytes
        const EVENT_SIZE: usize = 32 + 8 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumAmmV4SwapEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Swap 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8) {
                return None;
            }
            let amm = read_pubkey_unchecked(data, 0);
            let amount_in = read_u64_unchecked(data, 32);
            let amount_out = read_u64_unchecked(data, 40);
            Some(DexEvent::RaydiumAmmV4Swap(RaydiumAmmV4SwapEvent {
                metadata,
                amm,
                amount_in,
                amount_out,
                minimum_amount_out: 0,
                max_amount_in: 0,
                token_program: Pubkey::default(),
                amm_authority: Pubkey::default(),
                amm_open_orders: Pubkey::default(),
                amm_target_orders: None,
                pool_coin_token_account: Pubkey::default(),
                pool_pc_token_account: Pubkey::default(),
                serum_program: Pubkey::default(),
                serum_market: Pubkey::default(),
                serum_bids: Pubkey::default(),
                serum_asks: Pubkey::default(),
                serum_event_queue: Pubkey::default(),
                serum_coin_vault_account: Pubkey::default(),
                serum_pc_vault_account: Pubkey::default(),
                serum_vault_signer: Pubkey::default(),
                user_source_token_account: Pubkey::default(),
                user_destination_token_account: Pubkey::default(),
                user_source_owner: Pubkey::default(),
            }))
        }
    }

    // ============================================================================
    // Deposit 事件解析器
    // ============================================================================

    /// 解析 Deposit 事件（统一入口）
    #[inline(always)]
    fn parse_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_deposit_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_deposit_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Deposit 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_deposit_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // amm: Pubkey (32 bytes)
        // max_coin_amount: u64 (8 bytes)
        // max_pc_amount: u64 (8 bytes)
        // Total: 48 bytes
        const EVENT_SIZE: usize = 32 + 8 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumAmmV4DepositEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Deposit 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_deposit_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8) {
                return None;
            }
            let amm = read_pubkey_unchecked(data, 0);
            let max_coin_amount = read_u64_unchecked(data, 32);
            let max_pc_amount = read_u64_unchecked(data, 40);
            Some(DexEvent::RaydiumAmmV4Deposit(RaydiumAmmV4DepositEvent {
                metadata,
                amm,
                max_coin_amount,
                max_pc_amount,
                base_side: 0,
                token_program: Pubkey::default(),
                amm_authority: Pubkey::default(),
                amm_open_orders: Pubkey::default(),
                amm_target_orders: Pubkey::default(),
                lp_mint_address: Pubkey::default(),
                pool_coin_token_account: Pubkey::default(),
                pool_pc_token_account: Pubkey::default(),
                serum_market: Pubkey::default(),
                serum_event_queue: Pubkey::default(),
                user_coin_token_account: Pubkey::default(),
                user_pc_token_account: Pubkey::default(),
                user_lp_token_account: Pubkey::default(),
                user_owner: Pubkey::default(),
            }))
        }
    }

    // ============================================================================
    // Withdraw 事件解析器
    // ============================================================================

    /// 解析 Withdraw 事件（统一入口）
    #[inline(always)]
    fn parse_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        {
            parse_withdraw_borsh(data, metadata)
        }

        #[cfg(feature = "parse-zero-copy")]
        {
            parse_withdraw_zero_copy(data, metadata)
        }
    }

    /// Borsh 反序列化解析器 - Withdraw 事件
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_withdraw_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构:
        // amm: Pubkey (32 bytes)
        // amount: u64 (8 bytes)
        // Total: 40 bytes
        const EVENT_SIZE: usize = 32 + 8;

        if data.len() < EVENT_SIZE {
            return None;
        }

        let event = borsh::from_slice::<RaydiumAmmV4WithdrawEvent>(&data[..EVENT_SIZE]).ok()?;

        Some(DexEvent::RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent {
            metadata,
            ..event
        }))
    }

    /// 零拷贝解析器 - Withdraw 事件
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_withdraw_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8) {
                return None;
            }
            let amm = read_pubkey_unchecked(data, 0);
            let amount = read_u64_unchecked(data, 32);
            Some(DexEvent::RaydiumAmmV4Withdraw(RaydiumAmmV4WithdrawEvent {
                metadata,
                amm,
                amount,
                token_program: Pubkey::default(),
                amm_authority: Pubkey::default(),
                amm_open_orders: Pubkey::default(),
                amm_target_orders: Pubkey::default(),
                lp_mint_address: Pubkey::default(),
                pool_coin_token_account: Pubkey::default(),
                pool_pc_token_account: Pubkey::default(),
                pool_withdraw_queue: Pubkey::default(),
                pool_temp_lp_token_account: Pubkey::default(),
                serum_program: Pubkey::default(),
                serum_market: Pubkey::default(),
                serum_bids: Pubkey::default(),
                serum_asks: Pubkey::default(),
                serum_event_queue: Pubkey::default(),
                serum_coin_vault_account: Pubkey::default(),
                serum_pc_vault_account: Pubkey::default(),
                serum_vault_signer: Pubkey::default(),
                user_lp_token_account: Pubkey::default(),
                user_coin_token_account: Pubkey::default(),
                user_pc_token_account: Pubkey::default(),
                user_owner: Pubkey::default(),
            }))
        }
    }
}

// ============================================================================
// Orca Whirlpool
// ============================================================================

pub mod orca {
    //! Orca Whirlpool Inner Instruction 解析器
    //!
    //! ## 解析器插件系统
    //!
    //! 支持两种可插拔的解析器实现：
    //!
    //! ### 1. Borsh 反序列化解析器（默认，推荐）
    //! - **启用**: `cargo build --features parse-borsh` （默认）
    //! - 特点：类型安全、代码简洁、易于维护
    //!
    //! ### 2. 零拷贝解析器（高性能）
    //! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
    //! - 特点：最高性能、零内存分配、直接读取内存

    use super::*;

    pub mod discriminators {
        pub const TRADED: [u8; 16] = [225, 202, 73, 175, 147, 43, 160, 150, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const LIQUIDITY_INCREASED: [u8; 16] = [30, 7, 144, 181, 102, 254, 155, 161, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const LIQUIDITY_DECREASED: [u8; 16] = [166, 1, 36, 71, 112, 202, 181, 171, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const POOL_INITIALIZED: [u8; 16] = [100, 118, 173, 87, 12, 198, 254, 229, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::TRADED => parse_swap(data, metadata),
            &discriminators::LIQUIDITY_INCREASED => parse_liquidity_increased(data, metadata),
            &discriminators::LIQUIDITY_DECREASED => parse_liquidity_decreased(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Swap Event (Traded)
    // ============================================================================

    /// 解析 Swap 事件（统一入口）
    #[inline(always)]
    fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_swap_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_swap_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：whirlpool(32) + input_amount(8) + output_amount(8) + a_to_b(1) = 49 bytes
        const SWAP_EVENT_SIZE: usize = 32 + 8 + 8 + 1;
        if data.len() < SWAP_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<OrcaWhirlpoolSwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::OrcaWhirlpoolSwap(event))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8 + 1) { return None; }
            let whirlpool = read_pubkey_unchecked(data, 0);
            let input_amount = read_u64_unchecked(data, 32);
            let output_amount = read_u64_unchecked(data, 40);
            let a_to_b = read_bool_unchecked(data, 48);
            Some(DexEvent::OrcaWhirlpoolSwap(OrcaWhirlpoolSwapEvent {
                metadata, whirlpool, input_amount, output_amount, a_to_b,
                pre_sqrt_price: 0, post_sqrt_price: 0,
                input_transfer_fee: 0, output_transfer_fee: 0,
                lp_fee: 0, protocol_fee: 0,
            }))
        }
    }

    // ============================================================================
    // LiquidityIncreased Event
    // ============================================================================

    /// 解析 LiquidityIncreased 事件（统一入口）
    #[inline(always)]
    fn parse_liquidity_increased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_liquidity_increased_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_liquidity_increased_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_liquidity_increased_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：whirlpool(32) + liquidity(16) + token_a_amount(8) + token_b_amount(8) = 64 bytes
        const LIQUIDITY_EVENT_SIZE: usize = 32 + 16 + 8 + 8;
        if data.len() < LIQUIDITY_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<OrcaWhirlpoolLiquidityIncreasedEvent>(&data[..LIQUIDITY_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(event))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_liquidity_increased_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 16 + 8 + 8) { return None; }
            let whirlpool = read_pubkey_unchecked(data, 0);
            let liquidity = read_u128_unchecked(data, 32);
            let token_a_amount = read_u64_unchecked(data, 48);
            let token_b_amount = read_u64_unchecked(data, 56);
            Some(DexEvent::OrcaWhirlpoolLiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent {
                metadata, whirlpool, liquidity, token_a_amount, token_b_amount,
                position: Pubkey::default(), tick_lower_index: 0, tick_upper_index: 0,
                token_a_transfer_fee: 0, token_b_transfer_fee: 0,
            }))
        }
    }

    // ============================================================================
    // LiquidityDecreased Event
    // ============================================================================

    /// 解析 LiquidityDecreased 事件（统一入口）
    #[inline(always)]
    fn parse_liquidity_decreased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_liquidity_decreased_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_liquidity_decreased_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_liquidity_decreased_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：whirlpool(32) + liquidity(16) + token_a_amount(8) + token_b_amount(8) = 64 bytes
        const LIQUIDITY_EVENT_SIZE: usize = 32 + 16 + 8 + 8;
        if data.len() < LIQUIDITY_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<OrcaWhirlpoolLiquidityDecreasedEvent>(&data[..LIQUIDITY_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(event))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_liquidity_decreased_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 16 + 8 + 8) { return None; }
            let whirlpool = read_pubkey_unchecked(data, 0);
            let liquidity = read_u128_unchecked(data, 32);
            let token_a_amount = read_u64_unchecked(data, 48);
            let token_b_amount = read_u64_unchecked(data, 56);
            Some(DexEvent::OrcaWhirlpoolLiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent {
                metadata, whirlpool, liquidity, token_a_amount, token_b_amount,
                position: Pubkey::default(), tick_lower_index: 0, tick_upper_index: 0,
                token_a_transfer_fee: 0, token_b_transfer_fee: 0,
            }))
        }
    }
}

// ============================================================================
// Meteora AMM
// ============================================================================

pub mod meteora_amm {
    use super::*;

    pub mod discriminators {
        pub const SWAP: [u8; 16] = [81, 108, 227, 190, 205, 208, 10, 196, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const ADD_LIQUIDITY: [u8; 16] = [31, 94, 125, 90, 227, 52, 61, 186, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const REMOVE_LIQUIDITY: [u8; 16] = [116, 244, 97, 232, 103, 31, 152, 58, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const POOL_CREATED: [u8; 16] = [202, 44, 41, 88, 104, 220, 157, 82, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            match disc {
                &discriminators::SWAP => {
                    if !check_length(data, 8 + 8) { return None; }
                    let in_amount = read_u64_unchecked(data, 0);
                    let out_amount = read_u64_unchecked(data, 8);
                    Some(DexEvent::MeteoraPoolsSwap(MeteoraPoolsSwapEvent {
                        metadata, in_amount, out_amount, trade_fee: 0, admin_fee: 0, host_fee: 0,
                    }))
                }
                &discriminators::ADD_LIQUIDITY => {
                    if !check_length(data, 8 + 8 + 8) { return None; }
                    let lp_mint_amount = read_u64_unchecked(data, 0);
                    let token_a_amount = read_u64_unchecked(data, 8);
                    let token_b_amount = read_u64_unchecked(data, 16);
                    Some(DexEvent::MeteoraPoolsAddLiquidity(MeteoraPoolsAddLiquidityEvent {
                        metadata, lp_mint_amount, token_a_amount, token_b_amount,
                    }))
                }
                &discriminators::REMOVE_LIQUIDITY => {
                    if !check_length(data, 8 + 8 + 8) { return None; }
                    let lp_unmint_amount = read_u64_unchecked(data, 0);
                    let token_a_out_amount = read_u64_unchecked(data, 8);
                    let token_b_out_amount = read_u64_unchecked(data, 16);
                    Some(DexEvent::MeteoraPoolsRemoveLiquidity(MeteoraPoolsRemoveLiquidityEvent {
                        metadata, lp_unmint_amount, token_a_out_amount, token_b_out_amount,
                    }))
                }
                _ => None,
            }
        }
    }
}

// ============================================================================
// Meteora DAMM V2
// ============================================================================

pub mod meteora_damm {
    //! Meteora DAMM V2 Inner Instruction 解析器
    //!
    //! ## 解析器插件系统
    //!
    //! 支持两种可插拔的解析器实现：
    //!
    //! ### 1. Borsh 反序列化解析器（默认，推荐）
    //! - **启用**: `cargo build --features parse-borsh` （默认）
    //! - 特点：类型安全、代码简洁、易于维护
    //!
    //! ### 2. 零拷贝解析器（高性能）
    //! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
    //! - 特点：最高性能、零内存分配、直接读取内存

    use super::*;

    pub mod discriminators {
        pub const SWAP: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 27, 60, 21, 213, 138, 170, 187, 147];
        pub const SWAP2: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 189, 66, 51, 168, 38, 80, 117, 153];
        pub const ADD_LIQUIDITY: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 175, 242, 8, 157, 30, 247, 185, 169];
        pub const REMOVE_LIQUIDITY: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 87, 46, 88, 98, 175, 96, 34, 91];
        pub const CREATE_POSITION: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 156, 15, 119, 198, 29, 181, 221, 55];
        pub const CLOSE_POSITION: [u8; 16] = [228, 69, 165, 46, 81, 203, 154, 29, 20, 145, 144, 68, 143, 142, 214, 178];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::SWAP => parse_swap(data, metadata),
            &discriminators::SWAP2 => parse_swap2(data, metadata),
            &discriminators::ADD_LIQUIDITY => parse_add_liquidity(data, metadata),
            &discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity(data, metadata),
            &discriminators::CREATE_POSITION => parse_create_position(data, metadata),
            &discriminators::CLOSE_POSITION => parse_close_position(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Swap Event
    // ============================================================================

    /// 解析 Swap 事件（统一入口）
    #[inline(always)]
    fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_swap_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_swap_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool(32) + amount_in(8) + output_amount(8) = 48 bytes
        const SWAP_EVENT_SIZE: usize = 32 + 8 + 8;
        if data.len() < SWAP_EVENT_SIZE { return None; }

        let event = borsh::from_slice::<MeteoraDammV2SwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;
        Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent { metadata, ..event }))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 8 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let amount_in = read_u64_unchecked(data, 32);
            let output_amount = read_u64_unchecked(data, 40);
            Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
                metadata, pool, amount_in, output_amount, ..Default::default()
            }))
        }
    }

    // ============================================================================
    // Swap2 Event
    // ============================================================================

    /// 解析 Swap2 事件（统一入口）
    #[inline(always)]
    fn parse_swap2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_swap2_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_swap2_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 for Swap2
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap2_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // Swap2 事件结构：
        // pool(32) + config(32) + trade_direction(1) + has_referral(1) +
        // amount_0(8) + amount_1(8) + swap_mode(1) +
        // included_fee_input_amount(8) + excluded_fee_input_amount(8) + amount_left(8) +
        // output_amount(8) + next_sqrt_price(16) +
        // trading_fee(8) + protocol_fee(8) + referral_fee(8) +
        // quote_reserve_amount(8) + migration_threshold(8) + current_timestamp(8)
        // = 32 + 32 + 1 + 1 + 8 + 8 + 1 + 8 + 8 + 8 + 8 + 16 + 8 + 8 + 8 + 8 + 8 + 8 = 177 bytes
        const SWAP2_EVENT_MIN_SIZE: usize = 177;
        if data.len() < SWAP2_EVENT_MIN_SIZE { return None; }

        let mut offset = 0;

        // 使用 unsafe 读取以提高性能
        unsafe {
            let pool = read_pubkey_unchecked(data, offset);
            offset += 32;

            let _config = read_pubkey_unchecked(data, offset);
            offset += 32;

            let trade_direction = read_u8_unchecked(data, offset);
            offset += 1;

            let has_referral = read_bool_unchecked(data, offset);
            offset += 1;

            let amount_0 = read_u64_unchecked(data, offset);
            offset += 8;

            let amount_1 = read_u64_unchecked(data, offset);
            offset += 8;

            let swap_mode = read_u8_unchecked(data, offset);
            offset += 1;

            let included_fee_input_amount = read_u64_unchecked(data, offset);
            offset += 8;

            let _excluded_fee_input_amount = read_u64_unchecked(data, offset);
            offset += 8;

            let _amount_left = read_u64_unchecked(data, offset);
            offset += 8;

            let output_amount = read_u64_unchecked(data, offset);
            offset += 8;

            let next_sqrt_price = read_u128_unchecked(data, offset);
            offset += 16;

            let lp_fee = read_u64_unchecked(data, offset);
            offset += 8;

            let protocol_fee = read_u64_unchecked(data, offset);
            offset += 8;

            let referral_fee = read_u64_unchecked(data, offset);
            offset += 8;

            let _quote_reserve_amount = read_u64_unchecked(data, offset);
            offset += 8;

            let _migration_threshold = read_u64_unchecked(data, offset);
            offset += 8;

            let current_timestamp = read_u64_unchecked(data, offset);

            // 根据 swap_mode 确定 amount_in 和 minimum_amount_out
            let (amount_in, minimum_amount_out) = if swap_mode == 0 {
                (amount_0, amount_1)
            } else {
                (amount_1, amount_0)
            };

            Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
                metadata,
                pool,
                trade_direction,
                has_referral,
                amount_in,
                minimum_amount_out,
                output_amount,
                next_sqrt_price,
                lp_fee,
                protocol_fee,
                partner_fee: 0,
                referral_fee,
                actual_amount_in: included_fee_input_amount,
                current_timestamp,
                ..Default::default()
            }))
        }
    }

    /// 零拷贝解析器 for Swap2
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap2_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // Swap2 事件结构：
        // pool(32) + config(32) + trade_direction(1) + has_referral(1) +
        // amount_0(8) + amount_1(8) + swap_mode(1) +
        // included_fee_input_amount(8) + excluded_fee_input_amount(8) + amount_left(8) +
        // output_amount(8) + next_sqrt_price(16) +
        // trading_fee(8) + protocol_fee(8) + referral_fee(8) +
        // quote_reserve_amount(8) + migration_threshold(8) + current_timestamp(8)
        const SWAP2_EVENT_MIN_SIZE: usize = 177;

        unsafe {
            if !check_length(data, SWAP2_EVENT_MIN_SIZE) { return None; }

            let pool = read_pubkey_unchecked(data, 0);
            let trade_direction = read_u8_unchecked(data, 64);
            let has_referral = read_bool_unchecked(data, 65);
            let amount_0 = read_u64_unchecked(data, 66);
            let amount_1 = read_u64_unchecked(data, 74);
            let swap_mode = read_u8_unchecked(data, 82);
            let included_fee_input_amount = read_u64_unchecked(data, 83);
            let output_amount = read_u64_unchecked(data, 107);
            let next_sqrt_price = read_u128_unchecked(data, 115);
            let lp_fee = read_u64_unchecked(data, 131);
            let protocol_fee = read_u64_unchecked(data, 139);
            let referral_fee = read_u64_unchecked(data, 147);
            let current_timestamp = read_u64_unchecked(data, 169);

            // 根据 swap_mode 确定 amount_in 和 minimum_amount_out
            let (amount_in, minimum_amount_out) = if swap_mode == 0 {
                (amount_0, amount_1)
            } else {
                (amount_1, amount_0)
            };

            Some(DexEvent::MeteoraDammV2Swap(MeteoraDammV2SwapEvent {
                metadata,
                pool,
                trade_direction,
                has_referral,
                amount_in,
                minimum_amount_out,
                output_amount,
                next_sqrt_price,
                lp_fee,
                protocol_fee,
                partner_fee: 0,
                referral_fee,
                actual_amount_in: included_fee_input_amount,
                current_timestamp,
                ..Default::default()
            }))
        }
    }

    // ============================================================================
    // AddLiquidity Event
    // ============================================================================

    /// 解析 AddLiquidity 事件（统一入口）
    #[inline(always)]
    fn parse_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_add_liquidity_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_add_liquidity_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_add_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool(32) + position(32) + owner(32) + token_a_amount(8) + token_b_amount(8) = 112 bytes
        const ADD_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
        if data.len() < ADD_LIQUIDITY_EVENT_SIZE { return None; }

        let event = borsh::from_slice::<MeteoraDammV2AddLiquidityEvent>(&data[..ADD_LIQUIDITY_EVENT_SIZE]).ok()?;
        Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent { metadata, ..event }))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_add_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 8 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let position = read_pubkey_unchecked(data, 32);
            let owner = read_pubkey_unchecked(data, 64);
            let token_a_amount = read_u64_unchecked(data, 96);
            let token_b_amount = read_u64_unchecked(data, 104);
            Some(DexEvent::MeteoraDammV2AddLiquidity(MeteoraDammV2AddLiquidityEvent {
                metadata, pool, position, owner, token_a_amount, token_b_amount,
                liquidity_delta: 0, token_a_amount_threshold: 0, token_b_amount_threshold: 0,
                total_amount_a: 0, total_amount_b: 0,
            }))
        }
    }

    // ============================================================================
    // RemoveLiquidity Event
    // ============================================================================

    /// 解析 RemoveLiquidity 事件（统一入口）
    #[inline(always)]
    fn parse_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_remove_liquidity_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_remove_liquidity_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_remove_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool(32) + position(32) + owner(32) + token_a_amount(8) + token_b_amount(8) = 112 bytes
        const REMOVE_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
        if data.len() < REMOVE_LIQUIDITY_EVENT_SIZE { return None; }

        let event = borsh::from_slice::<MeteoraDammV2RemoveLiquidityEvent>(&data[..REMOVE_LIQUIDITY_EVENT_SIZE]).ok()?;
        Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent { metadata, ..event }))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_remove_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 8 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let position = read_pubkey_unchecked(data, 32);
            let owner = read_pubkey_unchecked(data, 64);
            let token_a_amount = read_u64_unchecked(data, 96);
            let token_b_amount = read_u64_unchecked(data, 104);
            Some(DexEvent::MeteoraDammV2RemoveLiquidity(MeteoraDammV2RemoveLiquidityEvent {
                metadata, pool, position, owner, token_a_amount, token_b_amount,
                liquidity_delta: 0, token_a_amount_threshold: 0, token_b_amount_threshold: 0,
            }))
        }
    }

    // ============================================================================
    // CreatePosition Event
    // ============================================================================

    /// 解析 CreatePosition 事件（统一入口）
    #[inline(always)]
    fn parse_create_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_create_position_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_create_position_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_create_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool(32) + owner(32) + position(32) + position_nft_mint(32) = 128 bytes
        const CREATE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 32;
        if data.len() < CREATE_POSITION_EVENT_SIZE { return None; }

        let event = borsh::from_slice::<MeteoraDammV2CreatePositionEvent>(&data[..CREATE_POSITION_EVENT_SIZE]).ok()?;
        Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent { metadata, ..event }))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_create_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 32) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let owner = read_pubkey_unchecked(data, 32);
            let position = read_pubkey_unchecked(data, 64);
            let position_nft_mint = read_pubkey_unchecked(data, 96);
            Some(DexEvent::MeteoraDammV2CreatePosition(MeteoraDammV2CreatePositionEvent {
                metadata, pool, owner, position, position_nft_mint,
            }))
        }
    }

    // ============================================================================
    // ClosePosition Event
    // ============================================================================

    /// 解析 ClosePosition 事件（统一入口）
    #[inline(always)]
    fn parse_close_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_close_position_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_close_position_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_close_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool(32) + owner(32) + position(32) + position_nft_mint(32) = 128 bytes
        const CLOSE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 32;
        if data.len() < CLOSE_POSITION_EVENT_SIZE { return None; }

        let event = borsh::from_slice::<MeteoraDammV2ClosePositionEvent>(&data[..CLOSE_POSITION_EVENT_SIZE]).ok()?;
        Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent { metadata, ..event }))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_close_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 32) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let owner = read_pubkey_unchecked(data, 32);
            let position = read_pubkey_unchecked(data, 64);
            let position_nft_mint = read_pubkey_unchecked(data, 96);
            Some(DexEvent::MeteoraDammV2ClosePosition(MeteoraDammV2ClosePositionEvent {
                metadata, pool, owner, position, position_nft_mint,
            }))
        }
    }
}

// ============================================================================
// Bonk (Raydium Launchpad)
// ============================================================================

pub mod bonk {
    //! Bonk (Raydium Launchpad) Inner Instruction 解析器
    //!
    //! ## 解析器插件系统
    //!
    //! 支持两种可插拔的解析器实现：
    //!
    //! ### 1. Borsh 反序列化解析器（默认，推荐）
    //! - **启用**: `cargo build --features parse-borsh` （默认）
    //! - 特点：类型安全、代码简洁、易于维护
    //!
    //! ### 2. 零拷贝解析器（高性能）
    //! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
    //! - 特点：最高性能、零内存分配、直接读取内存

    use super::*;

    pub mod discriminators {
        pub const POOL_CREATE: [u8; 16] = [100, 50, 200, 150, 75, 120, 90, 30, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const TRADE: [u8; 16] = [80, 120, 100, 200, 150, 75, 60, 40, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const MIGRATE: [u8; 16] = [90, 130, 110, 210, 160, 85, 70, 50, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::TRADE => parse_trade(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Trade Event
    // ============================================================================

    /// 解析 Trade 事件（统一入口）
    #[inline(always)]
    fn parse_trade(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_trade_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_trade_zero_copy(data, metadata) }
    }

    /// Borsh 解析器
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_trade_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // 数据结构：pool_state(32) + user(32) + amount_in(8) + amount_out(8) + is_buy(1) = 81 bytes
        const TRADE_EVENT_SIZE: usize = 32 + 32 + 8 + 8 + 1;
        if data.len() < TRADE_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<BonkTradeEvent>(&data[..TRADE_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        event.trade_direction = if event.is_buy { TradeDirection::Buy } else { TradeDirection::Sell };
        event.exact_in = true;
        Some(DexEvent::BonkTrade(event))
    }

    /// 零拷贝解析器
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_trade_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 8 + 8 + 1) { return None; }
            let pool_state = read_pubkey_unchecked(data, 0);
            let user = read_pubkey_unchecked(data, 32);
            let amount_in = read_u64_unchecked(data, 64);
            let amount_out = read_u64_unchecked(data, 72);
            let is_buy = read_bool_unchecked(data, 80);
            Some(DexEvent::BonkTrade(BonkTradeEvent {
                metadata, pool_state, user, amount_in, amount_out, is_buy,
                trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
                exact_in: true,
            }))
        }
    }
}

// ============================================================================
// Meteora DLMM
// ============================================================================

pub mod meteora_dlmm {
    //! Meteora DLMM Inner Instruction 解析器
    //!
    //! ## 解析器插件系统
    //!
    //! 支持两种可插拔的解析器实现：
    //!
    //! ### 1. Borsh 反序列化解析器（默认，推荐）
    //! - **启用**: `cargo build --features parse-borsh` （默认）
    //! - 特点：类型安全、代码简洁、易于维护
    //!
    //! ### 2. 零拷贝解析器（高性能）
    //! - **启用**: `cargo build --features parse-zero-copy --no-default-features`
    //! - 特点：最高性能、零内存分配、直接读取内存

    use super::*;

    pub mod discriminators {
        // 16-byte discriminators: 8-byte event hash + 8-byte magic
        pub const SWAP: [u8; 16] = [143, 190, 90, 218, 196, 30, 51, 222, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const ADD_LIQUIDITY: [u8; 16] = [181, 157, 89, 67, 143, 182, 52, 72, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const REMOVE_LIQUIDITY: [u8; 16] = [80, 85, 209, 72, 24, 206, 35, 178, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const INITIALIZE_POOL: [u8; 16] = [95, 180, 10, 172, 84, 174, 232, 40, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const INITIALIZE_BIN_ARRAY: [u8; 16] = [11, 18, 155, 194, 33, 115, 238, 119, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const CREATE_POSITION: [u8; 16] = [123, 233, 11, 43, 146, 180, 97, 119, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const CLOSE_POSITION: [u8; 16] = [94, 168, 102, 45, 59, 122, 137, 54, 155, 167, 108, 32, 122, 76, 173, 64];
        pub const CLAIM_FEE: [u8; 16] = [152, 70, 208, 111, 104, 91, 44, 1, 155, 167, 108, 32, 122, 76, 173, 64];
    }

    /// 主入口：根据 discriminator 解析事件
    #[inline]
    pub fn parse(disc: &[u8; 16], data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        match disc {
            &discriminators::SWAP => parse_swap(data, metadata),
            &discriminators::ADD_LIQUIDITY => parse_add_liquidity(data, metadata),
            &discriminators::REMOVE_LIQUIDITY => parse_remove_liquidity(data, metadata),
            &discriminators::INITIALIZE_POOL => parse_initialize_pool(data, metadata),
            &discriminators::INITIALIZE_BIN_ARRAY => parse_initialize_bin_array(data, metadata),
            &discriminators::CREATE_POSITION => parse_create_position(data, metadata),
            &discriminators::CLOSE_POSITION => parse_close_position(data, metadata),
            &discriminators::CLAIM_FEE => parse_claim_fee(data, metadata),
            _ => None,
        }
    }

    // ============================================================================
    // Swap Event
    // ============================================================================

    /// 解析 Swap 事件（统一入口）
    #[inline(always)]
    fn parse_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_swap_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_swap_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Swap
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_swap_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + from(32) + start_bin_id(4) + end_bin_id(4) + amount_in(8) + amount_out(8) + swap_for_y(1) + fee(8) + protocol_fee(8) + fee_bps(16) + host_fee(8) = 129 bytes
        const SWAP_EVENT_SIZE: usize = 32 + 32 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 16 + 8;
        if data.len() < SWAP_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmSwapEvent>(&data[..SWAP_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmSwap(event))
    }

    /// 零拷贝解析器 - Swap
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_swap_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 16 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let from = read_pubkey_unchecked(data, 32);
            let start_bin_id = read_i32_unchecked(data, 64);
            let end_bin_id = read_i32_unchecked(data, 68);
            let amount_in = read_u64_unchecked(data, 72);
            let amount_out = read_u64_unchecked(data, 80);
            let swap_for_y = read_bool_unchecked(data, 88);
            let fee = read_u64_unchecked(data, 89);
            let protocol_fee = read_u64_unchecked(data, 97);
            let fee_bps = read_u128_unchecked(data, 105);
            let host_fee = read_u64_unchecked(data, 121);
            Some(DexEvent::MeteoraDlmmSwap(MeteoraDlmmSwapEvent {
                metadata, pool, from, start_bin_id, end_bin_id, amount_in, amount_out,
                swap_for_y, fee, protocol_fee, fee_bps, host_fee,
            }))
        }
    }

    // ============================================================================
    // Add Liquidity Event
    // ============================================================================

    /// 解析 Add Liquidity 事件（统一入口）
    #[inline(always)]
    fn parse_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_add_liquidity_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_add_liquidity_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Add Liquidity
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_add_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + from(32) + position(32) + amounts[2](16) + active_bin_id(4) = 116 bytes
        const ADD_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 16 + 4;
        if data.len() < ADD_LIQUIDITY_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmAddLiquidityEvent>(&data[..ADD_LIQUIDITY_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmAddLiquidity(event))
    }

    /// 零拷贝解析器 - Add Liquidity
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_add_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 16 + 4) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let from = read_pubkey_unchecked(data, 32);
            let position = read_pubkey_unchecked(data, 64);
            let amount_0 = read_u64_unchecked(data, 96);
            let amount_1 = read_u64_unchecked(data, 104);
            let active_bin_id = read_i32_unchecked(data, 112);
            Some(DexEvent::MeteoraDlmmAddLiquidity(MeteoraDlmmAddLiquidityEvent {
                metadata, pool, from, position, amounts: [amount_0, amount_1], active_bin_id,
            }))
        }
    }

    // ============================================================================
    // Remove Liquidity Event
    // ============================================================================

    /// 解析 Remove Liquidity 事件（统一入口）
    #[inline(always)]
    fn parse_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_remove_liquidity_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_remove_liquidity_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Remove Liquidity
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_remove_liquidity_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + from(32) + position(32) + amounts[2](16) + active_bin_id(4) = 116 bytes
        const REMOVE_LIQUIDITY_EVENT_SIZE: usize = 32 + 32 + 32 + 16 + 4;
        if data.len() < REMOVE_LIQUIDITY_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmRemoveLiquidityEvent>(&data[..REMOVE_LIQUIDITY_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmRemoveLiquidity(event))
    }

    /// 零拷贝解析器 - Remove Liquidity
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_remove_liquidity_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 16 + 4) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let from = read_pubkey_unchecked(data, 32);
            let position = read_pubkey_unchecked(data, 64);
            let amount_0 = read_u64_unchecked(data, 96);
            let amount_1 = read_u64_unchecked(data, 104);
            let active_bin_id = read_i32_unchecked(data, 112);
            Some(DexEvent::MeteoraDlmmRemoveLiquidity(MeteoraDlmmRemoveLiquidityEvent {
                metadata, pool, from, position, amounts: [amount_0, amount_1], active_bin_id,
            }))
        }
    }

    // ============================================================================
    // Initialize Pool Event
    // ============================================================================

    /// 解析 Initialize Pool 事件（统一入口）
    #[inline(always)]
    fn parse_initialize_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_initialize_pool_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_initialize_pool_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Initialize Pool
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_initialize_pool_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + creator(32) + active_bin_id(4) + bin_step(2) = 70 bytes
        const INITIALIZE_POOL_EVENT_SIZE: usize = 32 + 32 + 4 + 2;
        if data.len() < INITIALIZE_POOL_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmInitializePoolEvent>(&data[..INITIALIZE_POOL_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmInitializePool(event))
    }

    /// 零拷贝解析器 - Initialize Pool
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_initialize_pool_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 4 + 2) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let creator = read_pubkey_unchecked(data, 32);
            let active_bin_id = read_i32_unchecked(data, 64);
            let bin_step = read_u16_unchecked(data, 68);
            Some(DexEvent::MeteoraDlmmInitializePool(MeteoraDlmmInitializePoolEvent {
                metadata, pool, creator, active_bin_id, bin_step,
            }))
        }
    }

    // ============================================================================
    // Initialize Bin Array Event
    // ============================================================================

    /// 解析 Initialize Bin Array 事件（统一入口）
    #[inline(always)]
    fn parse_initialize_bin_array(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_initialize_bin_array_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_initialize_bin_array_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Initialize Bin Array
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_initialize_bin_array_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + bin_array(32) + index(8) = 72 bytes
        const INITIALIZE_BIN_ARRAY_EVENT_SIZE: usize = 32 + 32 + 8;
        if data.len() < INITIALIZE_BIN_ARRAY_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmInitializeBinArrayEvent>(&data[..INITIALIZE_BIN_ARRAY_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmInitializeBinArray(event))
    }

    /// 零拷贝解析器 - Initialize Bin Array
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_initialize_bin_array_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let bin_array = read_pubkey_unchecked(data, 32);
            let index = read_i64_unchecked(data, 64);
            Some(DexEvent::MeteoraDlmmInitializeBinArray(MeteoraDlmmInitializeBinArrayEvent {
                metadata, pool, bin_array, index,
            }))
        }
    }

    // ============================================================================
    // Create Position Event
    // ============================================================================

    /// 解析 Create Position 事件（统一入口）
    #[inline(always)]
    fn parse_create_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_create_position_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_create_position_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Create Position
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_create_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + position(32) + owner(32) + lower_bin_id(4) + width(4) = 104 bytes
        const CREATE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32 + 4 + 4;
        if data.len() < CREATE_POSITION_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmCreatePositionEvent>(&data[..CREATE_POSITION_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmCreatePosition(event))
    }

    /// 零拷贝解析器 - Create Position
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_create_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 4 + 4) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let position = read_pubkey_unchecked(data, 32);
            let owner = read_pubkey_unchecked(data, 64);
            let lower_bin_id = read_i32_unchecked(data, 96);
            let width = read_u32_unchecked(data, 100);
            Some(DexEvent::MeteoraDlmmCreatePosition(MeteoraDlmmCreatePositionEvent {
                metadata, pool, position, owner, lower_bin_id, width,
            }))
        }
    }

    // ============================================================================
    // Close Position Event
    // ============================================================================

    /// 解析 Close Position 事件（统一入口）
    #[inline(always)]
    fn parse_close_position(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_close_position_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_close_position_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Close Position
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_close_position_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + position(32) + owner(32) = 96 bytes
        const CLOSE_POSITION_EVENT_SIZE: usize = 32 + 32 + 32;
        if data.len() < CLOSE_POSITION_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmClosePositionEvent>(&data[..CLOSE_POSITION_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmClosePosition(event))
    }

    /// 零拷贝解析器 - Close Position
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_close_position_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let position = read_pubkey_unchecked(data, 32);
            let owner = read_pubkey_unchecked(data, 64);
            Some(DexEvent::MeteoraDlmmClosePosition(MeteoraDlmmClosePositionEvent {
                metadata, pool, position, owner,
            }))
        }
    }

    // ============================================================================
    // Claim Fee Event
    // ============================================================================

    /// 解析 Claim Fee 事件（统一入口）
    #[inline(always)]
    fn parse_claim_fee(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        #[cfg(feature = "parse-borsh")]
        { parse_claim_fee_borsh(data, metadata) }

        #[cfg(feature = "parse-zero-copy")]
        { parse_claim_fee_zero_copy(data, metadata) }
    }

    /// Borsh 解析器 - Claim Fee
    #[cfg(feature = "parse-borsh")]
    #[inline(always)]
    fn parse_claim_fee_borsh(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        // pool(32) + position(32) + owner(32) + fee_x(8) + fee_y(8) = 112 bytes
        const CLAIM_FEE_EVENT_SIZE: usize = 32 + 32 + 32 + 8 + 8;
        if data.len() < CLAIM_FEE_EVENT_SIZE { return None; }

        let mut event = borsh::from_slice::<MeteoraDlmmClaimFeeEvent>(&data[..CLAIM_FEE_EVENT_SIZE]).ok()?;
        event.metadata = metadata;
        Some(DexEvent::MeteoraDlmmClaimFee(event))
    }

    /// 零拷贝解析器 - Claim Fee
    #[cfg(feature = "parse-zero-copy")]
    #[inline(always)]
    fn parse_claim_fee_zero_copy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
        unsafe {
            if !check_length(data, 32 + 32 + 32 + 8 + 8) { return None; }
            let pool = read_pubkey_unchecked(data, 0);
            let position = read_pubkey_unchecked(data, 32);
            let owner = read_pubkey_unchecked(data, 64);
            let fee_x = read_u64_unchecked(data, 96);
            let fee_y = read_u64_unchecked(data, 104);
            Some(DexEvent::MeteoraDlmmClaimFee(MeteoraDlmmClaimFeeEvent {
                metadata, pool, position, owner, fee_x, fee_y,
            }))
        }
    }
}
