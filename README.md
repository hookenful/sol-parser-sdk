<div align="center">
    <h1>⚡ Sol Parser SDK</h1>
    <h3><em>Ultra-low latency Solana DEX event parser with SIMD optimization</em></h3>
</div>

<p align="center">
    <strong>High-performance Rust library for parsing Solana DEX events with microsecond-level latency</strong>
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

## 📊 Performance Highlights

### ⚡ Ultra-Low Latency
- **10-20μs** parsing latency in release mode
- **Zero-copy** parsing with stack-allocated buffers
- **SIMD-accelerated** pattern matching (memchr)
- **Lock-free** ArrayQueue for event delivery

### 🎚️ Flexible Order Modes
| Mode | Latency | Description |
|------|---------|-------------|
| **Unordered** | 10-20μs | Immediate output, ultra-low latency |
| **MicroBatch** | 50-200μs | Micro-batch ordering with time window |
| **StreamingOrdered** | 0.1-5ms | Stream ordering with continuous sequence release |
| **Ordered** | 1-50ms | Full slot ordering, wait for complete slot |

### 🚀 Optimization Highlights
- ✅ **Zero heap allocation** for hot paths
- ✅ **SIMD pattern matching** for all protocol detection
- ✅ **Static pre-compiled finders** for string search
- ✅ **Inline functions** with aggressive optimization
- ✅ **Event type filtering** for targeted parsing
- ✅ **Conditional Create detection** (only when needed)
- ✅ **Multiple order modes** for latency vs ordering trade-off

---

## 🔥 Quick Start

### Installation

Clone the repository:

```bash
cd your_project_dir
git clone https://github.com/0xfnzero/sol-parser-sdk
```

Add to your `Cargo.toml`:

```toml
[dependencies]
# Default: Borsh parser
sol-parser-sdk = { path = "../sol-parser-sdk" }

# Or: Zero-copy parser (maximum performance)
sol-parser-sdk = { path = "../sol-parser-sdk", default-features = false, features = ["parse-zero-copy"] }
```

### Use crates.io

```toml
# Add to your Cargo.toml
sol-parser-sdk = "0.2.2"
```

Or with the zero-copy parser (maximum performance):

```toml
sol-parser-sdk = { version = "0.2.2", default-features = false, features = ["parse-zero-copy"] }
```

### Performance Testing

Test parsing latency with the optimized examples:

```bash
# PumpFun with detailed metrics (per-event + 10s stats)
cargo run --example pumpfun_with_metrics --release

# PumpSwap with detailed metrics (per-event + 10s stats)
cargo run --example pumpswap_with_metrics --release

# PumpSwap ultra-low latency test
cargo run --example pumpswap_low_latency --release

# PumpSwap with MicroBatch ordering
cargo run --example pumpswap_ordered --release

# Expected output:
# gRPC接收时间: 1234567890 μs
# 事件接收时间: 1234567900 μs
# 延迟时间: 10 μs  <-- Ultra-low latency!
```

### Examples

| Example | Description | Command |
|---------|-------------|---------|
| **PumpFun** | | |
| `pumpfun_with_metrics` | PumpFun event parsing with detailed performance metrics | `cargo run --example pumpfun_with_metrics --release` |
| `pumpfun_trade_filter` | PumpFun trade type filtering (Buy/Sell/BuyExactSolIn), Unordered mode | `cargo run --example pumpfun_trade_filter --release` |
| `pumpfun_trade_filter_ordered` | PumpFun trade filtering with StreamingOrdered mode | `cargo run --example pumpfun_trade_filter_ordered --release` |
| `pumpfun_quick_test` | Quick PumpFun connection test (first 10 events) | `cargo run --example pumpfun_quick_test --release` |
| `parse_pump_tx` | Parse a PumpFun transaction from RPC by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pump_tx --release` |
| `debug_pump_tx` | Debug PumpFun transaction structure and inner instructions | `cargo run --example debug_pump_tx --release` |
| **PumpSwap** | | |
| `pumpswap_with_metrics` | PumpSwap events with per-event and 10s performance stats | `cargo run --example pumpswap_with_metrics --release` |
| `pumpswap_low_latency` | PumpSwap ultra-low latency (Unordered, full event data) | `cargo run --example pumpswap_low_latency --release` |
| `pumpswap_ordered` | PumpSwap Buy/Sell/CreatePool with MicroBatch ordering | `cargo run --example pumpswap_ordered --release` |
| `parse_pumpswap_tx` | Parse a PumpSwap transaction from RPC by signature | `TX_SIGNATURE=<sig> cargo run --example parse_pumpswap_tx --release` |
| `debug_pumpswap_tx` | Debug PumpSwap transaction and gRPC conversion | `cargo run --example debug_pumpswap_tx --release` |
| **Meteora DAMM** | | |
| `meteora_damm_grpc` | Meteora DAMM V2 gRPC (Swap/AddLiquidity/RemoveLiquidity/CreatePosition/ClosePosition) | `cargo run --example meteora_damm_grpc --release` |
| `parse_meteora_damm_tx` | Parse a Meteora DAMM transaction from RPC by signature | `TX_SIGNATURE=<sig> cargo run --example parse_meteora_damm_tx --release` |
| **Account subscription** | | |
| `token_balance_listen` | Subscribe to a single token account balance updates | `TOKEN_ACCOUNT=<pubkey> cargo run --example token_balance_listen --release` |
| `nonce_listen` | Subscribe to a nonce account state changes | `NONCE_ACCOUNT=<pubkey> cargo run --example nonce_listen --release` |
| `token_decimals_listen` | Subscribe to a mint account (TokenInfo: decimals/supply) | `MINT_ACCOUNT=<pubkey> cargo run --example token_decimals_listen --release` |
| `pumpswap_pool_account_listen` | Subscribe to PumpSwap pool accounts via memcmp (e.g. mint at offset 32) | `cargo run --example pumpswap_pool_account_listen --release` |
| `mint_all_ata_account_listen` | Subscribe to all ATAs for one or more mints (memcmp offset 0) | `cargo run --example mint_all_ata_account_listen --release` |
| **Utility** | | |
| `dynamic_subscription` | Update transaction/account filters at runtime without reconnecting | `cargo run --example dynamic_subscription --release` |
| `test_account_filling` | Debug account filling for PumpSwap (RPC + account resolution) | `cargo run --example test_account_filling --release` |

### Basic Usage

```rust
use sol_parser_sdk::grpc::{YellowstoneGrpc, ClientConfig, OrderMode, EventTypeFilter, EventType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create gRPC client with default config (Unordered mode)
    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;
    
    // Or with custom config for ordered events
    let config = ClientConfig {
        order_mode: OrderMode::MicroBatch,  // Low latency + ordering
        micro_batch_us: 100,                // 100μs batch window
        ..ClientConfig::default()
    };
    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    // Filter for PumpFun Trade events only (ultra-fast path)
    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunTrade
    ]);

    // Subscribe and get lock-free queue
    let queue = grpc.subscribe_dex_events(
        vec![transaction_filter],
        vec![account_filter],
        Some(event_filter),
    ).await?;

    // Consume events with minimal latency
    tokio::spawn(async move {
        let mut spin_count = 0;
        loop {
            if let Some(event) = queue.pop() {
                spin_count = 0;
                // Process event (10-20μs latency!)
                println!("{:?}", event);
            } else {
                // Hybrid spin-wait strategy
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

## 🏗️ Supported Protocols

### DEX Protocols
- ✅ **PumpFun** - Meme coin trading (ultra-fast zero-copy path)
- ✅ **PumpSwap** - PumpFun swap protocol
- ✅ **Raydium AMM V4** - Automated Market Maker
- ✅ **Raydium CLMM** - Concentrated Liquidity
- ✅ **Raydium CPMM** - Concentrated Pool
- ✅ **Orca Whirlpool** - Concentrated liquidity AMM
- ✅ **Meteora AMM** - Dynamic AMM
- ✅ **Meteora DAMM** - Dynamic AMM V2
- ✅ **Meteora DLMM** - Dynamic Liquidity Market Maker
- ✅ **Bonk Launchpad** - Token launch platform

### Event Types
Each protocol supports:
- 📈 **Trade/Swap Events** - Buy/sell transactions
- 💧 **Liquidity Events** - Deposits/withdrawals
- 🏊 **Pool Events** - Pool creation/initialization
- 🎯 **Position Events** - Open/close positions (CLMM)

---

## ⚡ Performance Features

### Zero-Copy Parsing
```rust
// Stack-allocated 512-byte buffer for PumpFun Trade
const MAX_DECODE_SIZE: usize = 512;
let mut decode_buf: [u8; MAX_DECODE_SIZE] = [0u8; MAX_DECODE_SIZE];

// Decode directly to stack, no heap allocation
general_purpose::STANDARD
    .decode_slice(data_part.as_bytes(), &mut decode_buf)
    .ok()?;
```

### SIMD Pattern Matching
```rust
// Pre-compiled SIMD finders (initialized once)
static PUMPFUN_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));

// 3-10x faster than .contains()
if PUMPFUN_FINDER.find(log_bytes).is_some() {
    return LogType::PumpFun;
}
```

### Event Type Filtering
```rust
// Ultra-fast path for single event type
if include_only.len() == 1 && include_only[0] == EventType::PumpFunTrade {
    if log_type == LogType::PumpFun {
        return parse_pumpfun_trade(  // Zero-copy path
            log, signature, slot, block_time, grpc_recv_us, is_created_buy
        );
    }
}
```

### Lock-Free Queue
```rust
// ArrayQueue with 100,000 capacity
let queue = Arc::new(ArrayQueue::<DexEvent>::new(100_000));

// Non-blocking push/pop (no mutex overhead)
let _ = queue.push(event);
if let Some(event) = queue.pop() {
    // Process event
}
```

---

## 🎯 Event Filtering

Reduce processing overhead by filtering specific events:

### Example: Trading Bot
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunTrade,
    EventType::RaydiumAmmV4Swap,
    EventType::RaydiumClmmSwap,
    EventType::OrcaWhirlpoolSwap,
]);
```

### Example: Pool Monitor
```rust
let event_filter = EventTypeFilter::include_only(vec![
    EventType::PumpFunCreate,
    EventType::RaydiumClmmCreatePool,
    EventType::OrcaWhirlpoolInitialize,
]);
```

**Performance Impact:**
- 60-80% reduction in processing
- Lower memory usage
- Reduced network bandwidth

---

## 🔧 Advanced Features

### Create+Buy Detection
Automatically detects when a token is created and immediately bought in the same transaction:

```rust
// Detects "Program data: GB7IKAUcB3c..." pattern
let has_create = detect_pumpfun_create(logs);

// Sets is_created_buy flag on Trade events
if has_create {
    trade_event.is_created_buy = true;
}
```

### Dynamic Subscription
Update filters without reconnecting:

```rust
grpc.update_subscription(
    vec![new_transaction_filter],
    vec![new_account_filter],
).await?;
```

### Order Modes
Choose the right balance between latency and ordering:

```rust
use sol_parser_sdk::grpc::{ClientConfig, OrderMode};

// Ultra-low latency (no ordering guarantee)
let config = ClientConfig {
    order_mode: OrderMode::Unordered,
    ..ClientConfig::default()
};

// Low latency with micro-batch ordering (50-200μs)
let config = ClientConfig {
    order_mode: OrderMode::MicroBatch,
    micro_batch_us: 100,  // 100μs batch window
    ..ClientConfig::default()
};

// Stream ordering with continuous sequence release (0.1-5ms)
let config = ClientConfig {
    order_mode: OrderMode::StreamingOrdered,
    order_timeout_ms: 50,  // Timeout for incomplete sequences
    ..ClientConfig::default()
};

// Full slot ordering (1-50ms, wait for complete slot)
let config = ClientConfig {
    order_mode: OrderMode::Ordered,
    order_timeout_ms: 100,
    ..ClientConfig::default()
};
```

### Performance Metrics
```rust
let config = ClientConfig {
    enable_metrics: true,
    ..ClientConfig::default()
};

let grpc = YellowstoneGrpc::new_with_config(endpoint, token, config)?;
```

---

## 📁 Project Structure

```
src/
├── core/
│   └── events.rs          # Event definitions
├── grpc/
│   ├── client.rs          # Yellowstone gRPC client
│   ├── buffers.rs         # SlotBuffer & MicroBatchBuffer
│   └── types.rs           # OrderMode, ClientConfig, filters
├── logs/
│   ├── optimized_matcher.rs  # SIMD log detection
│   ├── zero_copy_parser.rs   # Zero-copy parsing
│   ├── pumpfun.rs         # PumpFun parser
│   ├── raydium_*.rs       # Raydium parsers
│   ├── orca_*.rs          # Orca parsers
│   └── meteora_*.rs       # Meteora parsers
├── instr/
│   └── *.rs               # Instruction parsers
├── warmup/
│   └── mod.rs             # Parser warmup (auto-called)
└── lib.rs
```

---

## 🚀 Optimization Techniques

### 1. **SIMD String Matching**
- Replaced all `.contains()` with `memmem::Finder`
- 3-10x performance improvement
- Pre-compiled static finders

### 2. **Zero-Copy Parsing**
- Stack-allocated buffers (512 bytes)
- No heap allocation in hot path
- Inline helper functions

### 3. **Event Type Filtering**
- Early filtering at protocol level
- Conditional Create detection
- Single-type ultra-fast path

### 4. **Lock-Free Queue**
- ArrayQueue (100K capacity)
- Spin-wait hybrid strategy
- No mutex overhead

### 5. **Aggressive Inlining**
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

## 📊 Benchmarks

### Parsing Latency (Release Mode)
| Protocol | Avg Latency | Min | Max |
|----------|-------------|-----|-----|
| PumpFun Trade (zero-copy) | 10-15μs | 8μs | 20μs |
| Raydium AMM V4 Swap | 15-20μs | 12μs | 25μs |
| Orca Whirlpool Swap | 15-20μs | 12μs | 25μs |

### SIMD Pattern Matching
| Operation | Before (contains) | After (SIMD) | Speedup |
|-----------|------------------|--------------|---------|
| Protocol detection | 50-100ns | 10-20ns | 3-10x |
| Create event detection | 150ns | 30ns | 5x |

---

## 📄 License

MIT License

## 📞 Contact

- **Repository**: https://github.com/0xfnzero/solana-streamer
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE

---

## ⚠️ Performance Tips

1. **Use Event Filtering** - Filter at the source for 60-80% performance gain
2. **Run in Release Mode** - `cargo build --release` for full optimization
3. **Test with sudo** - `sudo cargo run --example basic --release` for accurate timing
4. **Monitor Latency** - Check `grpc_recv_us` and queue latency in production
5. **Tune Queue Size** - Adjust ArrayQueue capacity based on your throughput
6. **Spin-Wait Strategy** - Tune spin count (default: 1000) for your use case

## 🔬 Development

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Generate docs
cargo doc --open
```
