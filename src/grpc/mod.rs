//! gRPC 模块 - 支持gRPC订阅、事件过滤、账号过滤
//!
//! 这个模块提供了完整的Solana DEX事件gRPC流式处理功能，包括：
//! - gRPC连接和订阅管理
//! - 事件类型过滤
//! - 账户和交易过滤
//! - 多协议支持（PumpFun, Bonk, Raydium等）

pub mod arpc_proto;
pub mod buffers;
pub mod client;
pub mod config;
pub mod deduper;
pub mod event_parser;
pub mod filter;
pub mod instruction_parser; // 增强的 instruction 解析器
pub mod program_ids;
pub mod types;

// 重新导出主要API
pub use client::{CorvusArpcV2Client, YellowstoneGrpcClient};
pub use deduper::TxDeduper;
pub use types::{
    account_filter_memcmp, AccountFilter, ClientConfig, CommitmentMode, EventType as StreamingEventType, EventTypeFilter,
    OrderMode, Protocol, SlotFilter, TransactionFilter,
};

// 事件解析器重新导出
pub use event_parser::*;

// 兼容性别名
pub use StreamingEventType as EventType;
