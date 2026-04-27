use serde::{Deserialize, Serialize};
use yellowstone_grpc_proto::geyser::{
    subscribe_request_filter_accounts_filter::Filter as AccountsFilterOneof,
    subscribe_request_filter_accounts_filter_memcmp::Data as MemcmpDataOneof,
    SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
};

/// 事件输出顺序模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OrderMode {
    /// 无序模式：收到即输出，超低延迟 (10-20μs)
    #[default]
    Unordered,
    /// 有序模式：按 slot + tx_index 排序后输出
    /// 同一 slot 内的交易会等待收齐后按 tx_index 排序
    /// 延迟增加约 1-50ms（取决于 slot 内交易数量）
    Ordered,
    /// 流式有序模式：连续序列立即释放，低延迟 + 顺序保证
    /// 只要收到从 0 开始的连续 tx_index 序列，立即释放
    /// 延迟约 0.1-5ms，比 Ordered 低 5-50 倍
    StreamingOrdered,
    /// 微批次模式：极短时间窗口内收集事件，窗口结束后排序释放
    /// 窗口大小由 micro_batch_us 配置（默认 100μs）
    /// 延迟约 50-200μs，接近 Unordered 但保证顺序
    MicroBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CommitmentMode {
    #[default]
    Processed,
    Confirmed,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 是否启用性能监控
    pub enable_metrics: bool,
    /// 连接超时时间（毫秒）
    pub connection_timeout_ms: u64,
    /// 请求超时时间（毫秒）
    pub request_timeout_ms: u64,
    /// 是否启用TLS
    pub enable_tls: bool,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_concurrent_streams: u32,
    pub keep_alive_interval_ms: u64,
    pub keep_alive_timeout_ms: u64,
    pub buffer_size: usize,
    /// 事件输出顺序模式
    pub order_mode: OrderMode,
    /// 有序模式下，slot 超时时间（毫秒）
    /// 超过此时间未收到新 slot 信号，强制输出当前缓冲的事件
    pub order_timeout_ms: u64,
    /// MicroBatch 模式下的时间窗口大小（微秒）
    /// 默认 100μs，可根据网络状况调整
    pub micro_batch_us: u64,
    /// Yellowstone 订阅承诺级别
    pub commitment: CommitmentMode,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            connection_timeout_ms: 8000,
            request_timeout_ms: 15000,
            enable_tls: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            max_concurrent_streams: 100,
            keep_alive_interval_ms: 30000,
            keep_alive_timeout_ms: 5000,
            buffer_size: 8192,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 100,
            micro_batch_us: 100, // 100μs 默认窗口
            commitment: CommitmentMode::Processed,
        }
    }
}

impl ClientConfig {
    pub fn low_latency() -> Self {
        Self {
            enable_metrics: false,
            connection_timeout_ms: 5000,
            request_timeout_ms: 0,
            enable_tls: true,
            max_retries: 5,
            retry_delay_ms: 50,
            max_concurrent_streams: 200,
            keep_alive_interval_ms: 3000,
            keep_alive_timeout_ms: 2000,
            buffer_size: 16384,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 50,
            micro_batch_us: 50, // 50μs 更激进的窗口
            commitment: CommitmentMode::Processed,
        }
    }

    pub fn high_throughput() -> Self {
        Self {
            enable_metrics: true,
            connection_timeout_ms: 10000,
            request_timeout_ms: 30000,
            enable_tls: true,
            max_retries: 5,
            retry_delay_ms: 2000,
            max_concurrent_streams: 500,
            keep_alive_interval_ms: 60000,
            keep_alive_timeout_ms: 10000,
            buffer_size: 32768,
            order_mode: OrderMode::Unordered,
            order_timeout_ms: 200,
            micro_batch_us: 200, // 200μs 高吞吐模式
            commitment: CommitmentMode::Processed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionFilter {
    pub account_include: Vec<String>,
    pub account_exclude: Vec<String>,
    pub account_required: Vec<String>,
}

impl TransactionFilter {
    pub fn new() -> Self {
        Self {
            account_include: Vec::new(),
            account_exclude: Vec::new(),
            account_required: Vec::new(),
        }
    }

    pub fn include_account(mut self, account: impl Into<String>) -> Self {
        self.account_include.push(account.into());
        self
    }

    pub fn exclude_account(mut self, account: impl Into<String>) -> Self {
        self.account_exclude.push(account.into());
        self
    }

    pub fn require_account(mut self, account: impl Into<String>) -> Self {
        self.account_required.push(account.into());
        self
    }

    /// 从程序ID列表创建过滤器
    pub fn from_program_ids(program_ids: Vec<String>) -> Self {
        Self {
            account_include: program_ids,
            account_exclude: Vec::new(),
            account_required: Vec::new(),
        }
    }
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AccountFilter {
    pub account: Vec<String>,
    pub owner: Vec<String>,
    pub filters: Vec<SubscribeRequestFilterAccountsFilter>,
}

impl AccountFilter {
    pub fn new() -> Self {
        Self { account: Vec::new(), owner: Vec::new(), filters: Vec::new() }
    }

    pub fn add_account(mut self, account: impl Into<String>) -> Self {
        self.account.push(account.into());
        self
    }

    pub fn add_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner.push(owner.into());
        self
    }

    pub fn add_filter(mut self, filter: SubscribeRequestFilterAccountsFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// 从程序ID列表创建所有者过滤器
    pub fn from_program_owners(program_ids: Vec<String>) -> Self {
        Self { account: Vec::new(), owner: program_ids, filters: Vec::new() }
    }
}

impl Default for AccountFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a memcmp account filter for use in `AccountFilter::filters`.
/// ATA accounts have mint at offset 0; PumpSwap pool accounts often use offset 32 for mint/pubkey.
#[inline]
pub fn account_filter_memcmp(offset: u64, bytes: Vec<u8>) -> SubscribeRequestFilterAccountsFilter {
    SubscribeRequestFilterAccountsFilter {
        filter: Some(AccountsFilterOneof::Memcmp(SubscribeRequestFilterAccountsFilterMemcmp {
            offset,
            data: Some(MemcmpDataOneof::Bytes(bytes)),
        })),
    }
}

#[derive(Debug, Clone)]
pub struct AccountFilterData {
    pub memcmp: Option<AccountFilterMemcmp>,
    pub datasize: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AccountFilterMemcmp {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    PumpFun,
    PumpSwap,
    Bonk,
    RaydiumCpmm,
    RaydiumClmm,
    RaydiumAmmV4,
    MeteoraDammV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // Block events
    BlockMeta,

    // Bonk events
    BonkTrade,
    BonkPoolCreate,
    BonkMigrateAmm,

    // PumpFun events
    PumpFunTrade,         // All trade events (backward compatible)
    PumpFunBuy,           // Buy events only (filter by ix_name)
    PumpFunSell,          // Sell events only (filter by ix_name)
    PumpFunBuyExactSolIn, // BuyExactSolIn events only (filter by ix_name)
    PumpFunCreate,
    PumpFunCreateV2, // SPL-22 / Mayhem create
    PumpFunComplete,
    PumpFunMigrate,

    // PumpSwap events
    PumpSwapBuy,
    PumpSwapSell,
    PumpSwapCreatePool,
    PumpSwapLiquidityAdded,
    PumpSwapLiquidityRemoved,
    // PumpSwapPoolUpdated,
    // PumpSwapFeesClaimed,

    // Raydium CPMM events
    // RaydiumCpmmSwap,
    // RaydiumCpmmDeposit,
    // RaydiumCpmmWithdraw,
    // RaydiumCpmmInitialize,

    // Raydium CLMM events
    // RaydiumClmmSwap,
    // RaydiumClmmCreatePool,
    // RaydiumClmmOpenPosition,
    // RaydiumClmmClosePosition,
    // RaydiumClmmIncreaseLiquidity,
    // RaydiumClmmDecreaseLiquidity,
    // RaydiumClmmOpenPositionWithTokenExtNft,
    // RaydiumClmmCollectFee,

    // Raydium AMM V4 events
    // RaydiumAmmV4Swap,
    // RaydiumAmmV4Deposit,
    // RaydiumAmmV4Withdraw,
    // RaydiumAmmV4Initialize2,
    // RaydiumAmmV4WithdrawPnl,

    // Orca Whirlpool events
    // OrcaWhirlpoolSwap,
    // OrcaWhirlpoolLiquidityIncreased,
    // OrcaWhirlpoolLiquidityDecreased,
    // OrcaWhirlpoolPoolInitialized,

    // Meteora events
    // MeteoraPoolsSwap,
    // MeteoraPoolsAddLiquidity,
    // MeteoraPoolsRemoveLiquidity,
    // MeteoraPoolsBootstrapLiquidity,
    // MeteoraPoolsPoolCreated,
    // MeteoraPoolsSetPoolFees,

    // Meteora DAMM V2 events
    MeteoraDammV2Swap,
    MeteoraDammV2AddLiquidity,
    MeteoraDammV2RemoveLiquidity,
    // MeteoraDammV2InitializePool,
    MeteoraDammV2CreatePosition,
    MeteoraDammV2ClosePosition,
    // MeteoraDammV2ClaimPositionFee,
    // MeteoraDammV2InitializeReward,
    // MeteoraDammV2FundReward,
    // MeteoraDammV2ClaimReward,

    // Account events
    TokenAccount,
    NonceAccount,

    AccountPumpSwapGlobalConfig,
    AccountPumpSwapPool,
}

#[derive(Debug, Clone)]
pub struct EventTypeFilter {
    pub include_only: Option<Vec<EventType>>,
    pub exclude_types: Option<Vec<EventType>>,
}

impl EventTypeFilter {
    pub fn include_only(types: Vec<EventType>) -> Self {
        Self { include_only: Some(types), exclude_types: None }
    }

    pub fn exclude_types(types: Vec<EventType>) -> Self {
        Self { include_only: None, exclude_types: Some(types) }
    }

    pub fn should_include(&self, event_type: EventType) -> bool {
        if let Some(ref include_only) = self.include_only {
            // Direct match
            if include_only.contains(&event_type) {
                return true;
            }
            // Special case: PumpFunTrade discriminator is shared by Buy/Sell/BuyExactSolIn
            // If filter includes any of these specific types, allow PumpFunTrade through
            // (secondary filtering will happen after parsing)
            if event_type == EventType::PumpFunTrade {
                return include_only.iter().any(|t| {
                    matches!(
                        t,
                        EventType::PumpFunBuy
                            | EventType::PumpFunSell
                            | EventType::PumpFunBuyExactSolIn
                    )
                });
            }
            return false;
        }

        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.contains(&event_type);
        }

        true
    }

    #[inline]
    pub fn includes_pumpfun(&self) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.iter().any(|t| {
                matches!(
                    t,
                    EventType::PumpFunTrade
                        | EventType::PumpFunBuy
                        | EventType::PumpFunSell
                        | EventType::PumpFunBuyExactSolIn
                        | EventType::PumpFunCreate
                        | EventType::PumpFunCreateV2
                        | EventType::PumpFunComplete
                        | EventType::PumpFunMigrate
                )
            });
        }

        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.iter().any(|t| {
                matches!(
                    t,
                    EventType::PumpFunTrade
                        | EventType::PumpFunBuy
                        | EventType::PumpFunSell
                        | EventType::PumpFunBuyExactSolIn
                        | EventType::PumpFunCreate
                        | EventType::PumpFunCreateV2
                        | EventType::PumpFunComplete
                        | EventType::PumpFunMigrate
                )
            });
        }

        true
    }

    #[inline]
    pub fn includes_meteora_damm_v2(&self) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.iter().any(|t| {
                matches!(
                    t,
                    EventType::MeteoraDammV2Swap
                        | EventType::MeteoraDammV2AddLiquidity
                        | EventType::MeteoraDammV2CreatePosition
                        | EventType::MeteoraDammV2ClosePosition
                        | EventType::MeteoraDammV2RemoveLiquidity
                )
            });
        }
        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.iter().any(|t| {
                matches!(
                    t,
                    EventType::MeteoraDammV2Swap
                        | EventType::MeteoraDammV2AddLiquidity
                        | EventType::MeteoraDammV2CreatePosition
                        | EventType::MeteoraDammV2ClosePosition
                        | EventType::MeteoraDammV2RemoveLiquidity
                )
            });
        }
        true
    }

    /// Check if PumpSwap protocol events are included in the filter
    #[inline]
    pub fn includes_pumpswap(&self) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.iter().any(|t| {
                matches!(
                    t,
                    EventType::PumpSwapBuy
                        | EventType::PumpSwapSell
                        | EventType::PumpSwapCreatePool
                        | EventType::PumpSwapLiquidityAdded
                        | EventType::PumpSwapLiquidityRemoved
                )
            });
        }
        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.iter().any(|t| {
                matches!(
                    t,
                    EventType::PumpSwapBuy
                        | EventType::PumpSwapSell
                        | EventType::PumpSwapCreatePool
                        | EventType::PumpSwapLiquidityAdded
                        | EventType::PumpSwapLiquidityRemoved
                )
            });
        }
        true
    }

    /// Check if Raydium Launchpad (Bonk) events are included in the filter
    #[inline]
    pub fn includes_raydium_launchpad(&self) -> bool {
        if let Some(ref include_only) = self.include_only {
            return include_only.iter().any(|t| {
                matches!(
                    t,
                    EventType::BonkTrade | EventType::BonkPoolCreate | EventType::BonkMigrateAmm
                )
            });
        }
        if let Some(ref exclude_types) = self.exclude_types {
            return !exclude_types.iter().any(|t| {
                matches!(
                    t,
                    EventType::BonkTrade | EventType::BonkPoolCreate | EventType::BonkMigrateAmm
                )
            });
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct SlotFilter {
    pub min_slot: Option<u64>,
    pub max_slot: Option<u64>,
}

impl SlotFilter {
    pub fn new() -> Self {
        Self { min_slot: None, max_slot: None }
    }

    pub fn min_slot(mut self, slot: u64) -> Self {
        self.min_slot = Some(slot);
        self
    }

    pub fn max_slot(mut self, slot: u64) -> Self {
        self.max_slot = Some(slot);
        self
    }
}

impl Default for SlotFilter {
    fn default() -> Self {
        Self::new()
    }
}
