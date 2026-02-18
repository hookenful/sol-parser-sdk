// 核心模块 - 扁平化结构
pub mod accounts; // 账户解析器
pub mod common;
pub mod core;
pub mod instr; // 指令解析器
pub mod logs; // 日志解析器
pub mod utils;
pub mod warmup; // 预热模块

// gRPC 模块 - 支持gRPC订阅和过滤
pub mod grpc;

// RPC 解析模块 - 支持直接从RPC解析交易
pub mod rpc_parser;

// 兼容性别名
pub mod parser {
    pub use crate::core::*;
}

// 重新导出主要API - 简化的单一入口解析器
pub use core::{
    parse_logs_only,
    parse_logs_streaming,
    // 主要解析函数
    parse_transaction_events,
    // 流式解析函数
    parse_transaction_events_streaming,
    parse_transaction_with_listener,
    parse_transaction_with_streaming_listener,
    // 事件类型
    DexEvent,
    // 事件监听器
    EventListener,
    EventMetadata,
    ParsedEvent,
    StreamingEventListener,
};

// 导出预热函数
pub use warmup::warmup_parser;

// 导出 RPC 解析函数
pub use rpc_parser::{
    convert_rpc_to_grpc, parse_rpc_transaction, parse_transaction_from_rpc, ParseError,
};
