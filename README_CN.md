<div align="center">
    <h1>⚡ Sol Parser SDK</h1>
    <h3><em>超低延迟的 Solana DEX 事件解析器（SIMD 优化）</em></h3>
</div>

<p align="center">
    <strong>高性能 Rust 库，提供微秒级延迟的 Solana DEX 事件解析</strong>
</p>

<p align="center">
    <a href="https://crates.io/crates/sol-parser-sdk">
        <img src="https://img.shields.io/crates/v/sol-parser-sdk.svg" alt="Crates.io">
    </a>
    <a href="https://docs.rs/sol-parser-sdk">
        <img src="https://docs.rs/sol-parser-sdk/badge.svg" alt="Documentation">
    </a>
    <a href="https://github.com/0xfnzero/solana-streamer/blob/main/LICENSE">
        <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License">
    </a>
</p>

<p align="center">
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana">
    <img src="https://img.shields.io/badge/SIMD-FF6B6B?style=for-the-badge&logo=intel&logoColor=white" alt="SIMD">
    <img src="https://img.shields.io/badge/gRPC-4285F4?style=for-the-badge&logo=grpc&logoColor=white" alt="gRPC">
</p>

<p align="center">
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README_CN.md">中文</a> |
    <a href="https://github.com/0xfnzero/sol-parser-sdk/blob/main/README.md">English</a> |
    <a href="https://fnzero.dev/">Website</a> |
    <a href="https://t.me/fnzero_group">Telegram</a> |
    <a href="https://discord.gg/vuazbGkqQE">Discord</a>
</p>

---

## 📊 性能亮点

### ⚡ 超低延迟
- **10-20μs** 解析延迟（Release 模式）
- **零拷贝** 栈缓冲区解析
- **SIMD 加速** 模式匹配（memchr）
- **无锁队列** ArrayQueue 事件传递

### 🎚️ 灵活的顺序模式
| 模式 | 延迟 | 说明 |
|------|---------|-------------|
| **Unordered** | 10-20μs | 立即输出，超低延迟 |
| **MicroBatch** | 50-200μs | 微批次排序，时间窗口内排序 |
| **StreamingOrdered** | 0.1-5ms | 流式排序，连续序列立即释放 |
| **Ordered** | 1-50ms | 完整 slot 排序，等待整个 slot 完成 |

### 🚀 优化特性
- ✅ **零堆分配** 热路径无堆分配
- ✅ **SIMD 模式匹配** 所有协议检测 SIMD 加速
- ✅ **静态预编译查找器** 字符串搜索零开销
- ✅ **激进内联** 关键函数强制内联
- ✅ **事件类型过滤** 精准解析目标事件
- ✅ **条件 Create 检测** 仅在需要时检测
- ✅ **多种顺序模式** 延迟与顺序的灵活平衡

---

## 🔥 快速开始

### 安装

克隆仓库：

```bash
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

在 `Cargo.toml` 中添加：

```toml
[dependencies]
# 默认：Borsh 解析器
sol-parser-sdk = { path = "../sol-parser-sdk" }

# 或：零拷贝解析器（最高性能）
sol-parser-sdk = { path = "../sol-parser-sdk", default-features = false, features = ["parse-zero-copy"] }
```

### 使用 crates.io

```toml
# 在 Cargo.toml 中添加
sol-parser-sdk = "0.2.2"
```

或使用零拷贝解析器（最高性能）：

```toml
sol-parser-sdk = { version = "0.2.2", default-features = false, features = ["parse-zero-copy"] }
```

### 性能测试

使用优化示例测试解析延迟：

```bash
# PumpFun 详细性能指标
cargo run --example pumpfun_with_metrics --release

# PumpSwap 详细性能指标（单事件明细 + 每 10 秒统计）
cargo run --example pumpswap_with_metrics --release


# PumpSwap 超低延迟测试
cargo run --example pumpswap_low_latency --release

# PumpSwap 事件 + MicroBatch 有序模式
cargo run --example pumpswap_ordered --release

# 预期输出：
# gRPC接收时间: 1234567890 μs
# 事件接收时间: 1234567900 μs
# 延迟时间: 10 μs  <-- 超低延迟！
```

### 示例列表

| 示例 | 说明 | 命令 |
|---------|-------------|---------|
| **PumpFun** | | |
| `pumpfun_with_metrics` | PumpFun 事件解析 + 详细性能指标 | `cargo run --example pumpfun_with_metrics --release` |
| `pumpfun_trade_filter` | PumpFun 交易类型过滤（Buy/Sell/BuyExactSolIn），无序模式 | `cargo run --example pumpfun_trade_filter --release` |
| `pumpfun_trade_filter_ordered` | PumpFun 交易过滤 + StreamingOrdered 有序模式 | `cargo run --example pumpfun_trade_filter_ordered --release` |
| `pumpfun_quick_test` | PumpFun 快速连接测试（前 10 个事件） | `cargo run --example pumpfun_quick_test --release` |
| `parse_pump_tx` | 按签名从 RPC 解析 PumpFun 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` |
| `debug_pump_tx` | 调试 PumpFun 交易结构与内部指令 | `cargo run --example debug_pump_tx --release` |
| **PumpSwap** | | |
| `pumpswap_with_metrics` | PumpSwap 事件 + 单事件与 10 秒性能统计 | `cargo run --example pumpswap_with_metrics --release` |
| `pumpswap_low_latency` | PumpSwap 超低延迟（无序，完整事件数据） | `cargo run --example pumpswap_low_latency --release` |
| `pumpswap_ordered` | PumpSwap 买/卖/创建池 + MicroBatch 有序 | `cargo run --example pumpswap_ordered --release` |
| `parse_pumpswap_tx` | 按签名从 RPC 解析 PumpSwap 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` |
| `debug_pumpswap_tx` | 调试 PumpSwap 交易与 gRPC 转换 | `cargo run --example debug_pumpswap_tx --release` |
| **Meteora DAMM** | | |
| `meteora_damm_grpc` | Meteora DAMM V2 gRPC（Swap/AddLiquidity/RemoveLiquidity/CreatePosition/ClosePosition） | `cargo run --example meteora_damm_grpc --release` |
| `parse_meteora_damm_tx` | 按签名从 RPC 解析 Meteora DAMM 交易 | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` |
| **账户订阅** | | |
| `token_balance_listen` | 订阅单个 token 账户余额变化 | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` |
| `nonce_listen` | 订阅 nonce 账户状态变化 | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` |
| `token_decimals_listen` | 订阅 mint 账户（TokenInfo：decimals/supply） | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` |
| `pumpswap_pool_account_listen` | 通过 memcmp 订阅 PumpSwap 池账户（如 offset 32 的 mint） | `cargo run --example pumpswap_pool_account_listen --release` |
| `mint_all_ata_account_listen` | 订阅一个或多个 mint 的全部 ATA（memcmp offset 0） | `cargo run --example mint_all_ata_account_listen --release` |
| **工具** | | |
| `dynamic_subscription` | 运行时更新交易/账户过滤器（无需重连） | `cargo run --example dynamic_subscription --release` |
| `test_account_filling` | 调试 PumpSwap 账户填充（RPC + 账户解析） | `cargo run --example test_account_filling --release` |

### 基本用法

```rust
use sol_parser_sdk::grpc::{YellowstoneGrpc, ClientConfig, OrderMode, EventTypeFilter, EventType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 gRPC 客户端（默认 Unordered 模式）
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;
    
    // 或使用自定义配置启用有序模式
    let config = ClientConfig {
        order_mode: OrderMode::MicroBatch,  // 低延迟 + 有序
        micro_batch_us: 100,                // 100μs 批次窗口
        ..ClientConfig::default()
    };
    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    // 仅过滤 PumpFun Trade 事件（超快路径）
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunTrade
    ]);

    // 订阅并获取无锁队列
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    ).await?;

    // 最小延迟消费事件
    tokio::spawn(async move {
        let mut spin_count = 0;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                // 处理事件（10-20μs 延迟！）
                println!("{:?}", event);
            } else {
                // 混合自旋等待策略
                spin_count += 1;
                if spin_count < 1000 {
                    std::hint::spin_loop();
                } else {
                    tokio::task::yield_now().await;
                    spin_count = 0;
                }
            }
        }
    });

    Ok(())
}
```

---

## 🏗️ 支持的协议

### DEX 协议
- ✅ **PumpFun** - Meme 币交易（超快零拷贝路径）
- ✅ **PumpSwap** - PumpFun 交换协议
- ✅ **Raydium AMM V4** - 自动做市商
- ✅ **Raydium CLMM** - 集中流动性做市
- ✅ **Raydium CPMM** - 集中池做市
- ✅ **Orca Whirlpool** - 集中流动性 AMM
- ✅ **Meteora AMM** - 动态 AMM
- ✅ **Meteora DAMM** - 动态 AMM V2
- ✅ **Meteora DLMM** - 动态流动性做市
- ✅ **Bonk Launchpad** - 代币发射平台

### 事件类型
每个协议支持：
- 📈 **交易/兑换事件** - 买入/卖出交易
- 💧 **流动性事件** - 存款/提款
- 🏊 **池事件** - 池创建/初始化
- 🎯 **仓位事件** - 开仓/平仓（CLMM）

---

## ⚡ 性能特性

### 零拷贝解析
```rust
// PumpFun Trade 使用 512 字节栈缓冲区
const MAX_DECODE_SIZE: usize = 512;
let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

// 直接解码到栈，无堆分配
general_purpose::STANDARD
    .decode_slice(data_part.as_bytes(), &mut decode_buf)
    .ok()?;
```

### SIMD 模式匹配
```rust
// 预编译 SIMD 查找器（初始化一次）
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));

// 比 .contains() 快 3-10 倍
if PUMPFUN_FINDER.find(log_bytes).is_some() {
    return LogType::PumpFun;
}
```

### 事件类型过滤
```rust
// 单一事件类型超快路径
if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
    if log_type == LogType::PumpFun {
        return parse_pumpfun_trade(  // 零拷贝路径
            log, signature, slot, block_time, grpc_recv_us, is_created_buy
        );
    }
}
```

### 无锁队列
```rust
// 100,000 容量的 ArrayQueue
let queue = Arc::new(ArrayQueue::<DexEvent>::new(100_000));

// 非阻塞 push/pop（无互斥锁开销）
let _ = queue.push(event);
if let Some(event) = queue.pop() {
    // 处理事件
}
```

---

## 🎯 事件过滤

通过过滤特定事件减少处理开销：

### 示例：交易机器人
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunTrade,
    EventType::RaydiumAmmV4Swap,
    EventType::RaydiumClmmSwap,
    EventType::OrcaWhirlpoolSwap,
]);
```

### 示例：池监控
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunCreate,
    EventType::RaydiumClmmCreatePool,
    EventType::OrcaWhirlpoolInitialize,
]);
```

**性能影响：**
- 减少 60-80% 的处理开销
- 降低内存使用
- 减少网络带宽

---

## 🔧 高级功能

### Create+Buy 检测
自动检测代币创建后立即购买的交易：

```rust
// 检测 "Program data: GB7IKAUcB3c..." 模式
let has_create = detect_pumpfun_create(logs);

// 在 Trade 事件上设置 is_created_buy 标志
if has_create {
    trade_event.is_created_buy = true;
}
```

### 动态订阅
无需重连即可更新过滤器：

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### 顺序模式
根据场景选择延迟与顺序的平衡：

```rust
use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

// 超低延迟（无顺序保证）
let config = ClientConfig {
    order_mode: OrderMode::Unordered,
    ..ClientConfig::default()
};

// 低延迟微批次排序（50-200μs）
let config = ClientConfig {
    order_mode: OrderMode::MicroBatch,
    micro_batch_us: 100,  // 100μs 批次窗口
    ..ClientConfig::default()
};

// 流式排序，连续序列立即释放（0.1-5ms）
let config = ClientConfig {
    order_mode: OrderMode::StreamingOrdered,
    order_timeout_ms: 50,  // 不完整序列超时
    ..ClientConfig::default()
};

// 完整 slot 排序（1-50ms，等待整个 slot）
let config = ClientConfig {
    order_mode: OrderMode::Ordered,
    order_timeout_ms: 100,
    ..ClientConfig::default()
};
```

### 性能指标
```rust
let config = ClientConfig {
    enable_metrics: true,
    ..ClientConfig::default()
};

let grpc = YellowstoneGrpc::new_with_config(endpoint, token, config)?;
```

---

## 📁 项目结构

```
src/
├── core/
│   └── events.rs          # 事件定义
├── grpc/
│   ├── client.rs          # Yellowstone gRPC 客户端
│   ├── buffers.rs         # SlotBuffer 和 MicroBatchBuffer
│   └── types.rs           # OrderMode、ClientConfig、过滤器
├── logs/
│   ├── optimized_matcher.rs  # SIMD 日志检测
│   ├── zero_copy_parser.rs   # 零拷贝解析
│   ├── pumpfun.rs         # PumpFun 解析器
│   ├── raydium_*.rs       # Raydium 解析器
│   ├── orca_*.rs          # Orca 解析器
│   └── meteora_*.rs       # Meteora 解析器
├── instr/
│   └── *.rs               # 指令解析器
├── warmup/
│   └── mod.rs             # 解析器预热（自动调用）
└── lib.rs
```

---

## 🚀 优化技术

### 1. **SIMD 字符串匹配**
- 所有 `.contains()` 替换为 `memmem::Finder`
- 性能提升 3-10 倍
- 预编译静态查找器

### 2. **零拷贝解析**
- 栈分配缓冲区（512 字节）
- 热路径无堆分配
- 内联辅助函数

### 3. **事件类型过滤**
- 协议级别早期过滤
- 条件 Create 检测
- 单类型超快路径

### 4. **无锁队列**
- ArrayQueue（100K 容量）
- 自旋等待混合策略
- 无互斥锁开销

### 5. **激进内联**
```rust
#[inline(always)]
fn read_u64_le_inline(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 <= data.len() {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[offset..offset + 8]);
        Some(u64::from_le_bytes(bytes))
    } else {
        None
    }
}
```

---

## 📊 性能基准

### 解析延迟（Release 模式）
| 协议 | 平均延迟 | 最小 | 最大 |
|----------|-------------|-----|-----|
| PumpFun Trade（零拷贝） | 10-15μs | 8μs | 20μs |
| Raydium AMM V4 Swap | 15-20μs | 12μs | 25μs |
| Orca Whirlpool Swap | 15-20μs | 12μs | 25μs |

### SIMD 模式匹配
| 操作 | 优化前（contains） | 优化后（SIMD） | 提升 |
|-----------|------------------|--------------|---------|
| 协议检测 | 50-100ns | 10-20ns | 3-10x |
| Create 事件检测 | 150ns | 30ns | 5x |

---

## 📄 许可证

MIT License

## 📞 联系方式

- **仓库**: https://github.com/0xfnzero/solana-streamer
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE

---

## ⚠️ 性能建议

1. **使用事件过滤** - 源头过滤可获得 60-80% 性能提升
2. **Release 模式运行** - `cargo build --release` 获得完整优化
3. **使用 sudo 测试** - `sudo cargo run --example basic --release` 获得精确计时
4. **监控延迟** - 生产环境检查 `grpc_recv_us` 和队列延迟
5. **调整队列大小** - 根据吞吐量调整 ArrayQueue 容量
6. **自旋等待策略** - 根据使用场景调整自旋计数（默认：1000）

## 🔬 开发

```bash
# 运行测试
cargo test

# 构建 release 二进制
cargo build --release

# 生成文档
cargo doc --open
```
